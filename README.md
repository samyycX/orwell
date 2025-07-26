# Orwell协议技术文档

## 概述

Orwell是一个基于后量子密码学的安全即时通讯协议，采用Kyber双棘轮算法（Kyber Double Ratchet）实现前向保密和密钥轮换，结合Dilithium数字签名确保消息完整性和身份认证。

## 核心特性

- **后量子安全**：基于NIST标准化的后量子算法
- **前向保密**：通过双棘轮机制实现密钥轮换
- **端到端加密**：所有消息端到端加密传输
- **身份认证**：基于数字签名的身份验证机制
- **重放攻击防护**：基于时间戳和盐值的防重放机制

## 密码学算法

### 1. 密钥交换算法
- **Kyber-1024**：基于格理论的密钥封装机制（KEM）
- 公钥大小：1568字节
- 密文大小：1568字节
- 共享密钥：32字节

### 2. 数字签名算法
- **Dilithium-5**：基于格理论的数字签名算法
- 公钥大小：2592字节
- 私钥大小：4864字节
- 签名大小：4595字节

### 3. 对称加密算法
- **AES-256-GCM**：用于消息内容加密
- 密钥派生：HKDF-SHA256
- 密钥强化：Argon2id

## 协议架构

### 数据包结构

#### 1. 基础数据包格式
```protobuf
message OrwellPacket {
  uint64 timestamp = 1;    // 时间戳（毫秒）
  bytes salt = 2;          // 128字节随机盐值
  PacketType packet_type = 3;  // 数据包类型
  bytes data = 4;          // 序列化的消息数据
}
```

#### 2. 签名数据包格式
```protobuf
message OrwellSignedPacket {
  OrwellPacket data = 1;   // 基础数据包
  bytes sign = 2;          // Dilithium数字签名
}
```

#### 3. 棘轮数据包格式
```protobuf
message OrwellRatchetPacket {
  bytes kyber_pk = 1;      // Kyber公钥
  uint64 send_counter = 2; // 发送计数器
  uint64 recv_counter = 3; // 接收计数器
  bytes data = 4;          // 加密的OrwellSignedPacket
}
```

### 消息类型

#### 客户端消息类型
| 类型值 | 消息类型 | 描述 |
|--------|----------|------|
| 0 | ClientHeartbeat | 心跳包 |
| 1 | ClientError | 错误消息 |
| 2 | ClientInformation | 信息消息 |
| 3 | ClientHello | 握手第一阶段 |
| 4 | ClientPreLogin | 预登录请求 |
| 5 | ClientRegister | 注册请求 |
| 6 | ClientLogin | 登录请求 |
| 7 | ClientMessage | 聊天消息 |
| 8 | ClientChangeColor | 颜色变更 |
| 9 | ClientAfk | AFK状态变更 |

#### 服务器消息类型
| 类型值 | 消息类型 | 描述 |
|--------|----------|------|
| 10000 | ServerHeartbeat | 服务器心跳 |
| 10001 | ServerError | 服务器错误 |
| 10002 | ServerInformation | 服务器信息 |
| 10003 | ServerHello | 握手响应 |
| 10004 | ServerPreLogin | 预登录响应 |
| 10005 | ServerRegisterResponse | 注册响应 |
| 10006 | ServerLoginResponse | 登录响应 |
| 10007 | ServerClientInfo | 客户端信息列表 |
| 10008 | ServerBroadcastMessage | 广播消息 |
| 10009 | ServerHistoryMessage | 历史消息 |
| 10010 | ServerChangeColorResponse | 颜色变更响应 |
| 10011 | ServerOrwellRatchetStep | 棘轮步进 |

## 握手协议

### 1. 初始握手（Kyber密钥交换）

#### 第一阶段：ClientHello
```
客户端 → 服务器
- 发送ClientHello包含客户端Kyber公钥
- 触发服务器创建新的棘轮会话
```

#### 第二阶段：ServerHello
```
服务器 → 客户端
- 发送ServerHello包含：
  - 服务器Kyber公钥
  - 加密的共享密钥密文
  - 服务器Dilithium公钥
```

#### 第三阶段：ClientHello2
```
客户端 → 服务器
- 发送ClientHello2包含加密的确认密文
- 完成密钥交换，建立安全通道
```

### 2. 双棘轮算法流程

#### 密钥派生结构
```
Root Key (32字节)
├── Send Chain Key (32字节) → 消息密钥派生
└── Recv Chain Key (32字节) → 消息密钥派生
```

#### 消息密钥派生
```
Message Key = HMAC-SHA256(Chain Key, "OrwellKDRMessageKey")
Chain Key = HMAC-SHA256(Chain Key, "OrwellKDRChainKey")
```

#### 棘轮步进
- **发送步进**：每次发送消息后，更新发送链密钥
- **接收步进**：每次接收消息后，更新接收链密钥
- **DH棘轮**：定期执行完整的密钥轮换

## 消息加密流程

### 1. 消息发送流程
```
1. 序列化消息数据
2. 计算SHA3-512哈希
3. 使用Dilithium私钥签名
4. 使用当前消息密钥AES-256-GCM加密
5. 打包为OrwellRatchetPacket
```

### 2. 消息接收流程
```
1. 解密OrwellRatchetPacket
2. 验证Dilithium签名
3. 检查时间戳有效性（10秒窗口）
4. 验证盐值唯一性
5. 反序列化消息数据
```

## 安全机制

### 1. 重放攻击防护
- **时间戳验证**：消息有效期10秒
- **盐值唯一性**：128字节随机盐值，10秒内不可重复使用
- **序列号机制**：基于棘轮计数器防重放

### 2. 前向保密
- **密钥轮换**：基于双棘轮算法的持续密钥更新
- **密钥派生**：使用HKDF进行安全密钥派生
- **密钥销毁**：旧密钥使用后立即销毁

### 3. 身份认证
- **数字签名**：所有消息使用Dilithium签名
- **公钥验证**：基于Dilithium公钥的身份验证
- **证书链**：服务器使用TLS证书进行身份验证

## 网络传输

### 1. 传输层安全
- **WebSocket over TLS**：使用WSS协议
- **TLS 1.3**：最新TLS协议版本
- **证书验证**：服务器端证书验证

### 2. 数据编码
- **二进制传输**：使用Protocol Buffers序列化
- **压缩优化**：消息数据最小化传输
- **分片支持**：支持大数据包分片传输

## 客户端状态管理

### 1. 连接状态
- **HandshakePhase1**：初始连接状态
- **HandshakePhase2**：密钥交换进行中
- **HandshakeFinished**：安全通道已建立

### 2. 用户状态
- **Online**：在线状态
- **Offline**：离线状态
- **Afk**：离开状态

## 性能优化

### 1. 密钥缓存
- **跳过的密钥**：缓存未来可能使用的密钥
- **内存管理**：自动清理过期密钥
- **计数器同步**：维护发送/接收计数器同步

### 2. 批量处理
- **消息批处理**：支持批量消息发送
- **心跳优化**：随机化心跳间隔减少负载
- **连接池**：高效的连接管理

## 错误处理

### 1. 加密错误
- **密钥验证失败**：触发重新握手
- **消息解密失败**：记录错误并跳过
- **签名验证失败**：拒绝消息并记录

### 2. 网络错误
- **连接断开**：自动清理相关状态
- **超时处理**：合理的超时和重试机制
- **错误恢复**：优雅的错误恢复流程

## 部署配置

### 1. 服务器配置
- **数据库**：SQLite持久化存储
- **TLS证书**：支持自定义证书路径
- **端口配置**：可配置的监听端口

### 2. 客户端配置
- **服务器地址**：支持自定义服务器地址
- **自动重连**：断线自动重连机制
- **本地存储**：用户配置本地持久化

## 协议版本

- **当前版本**：基于Git提交哈希的版本标识
- **向后兼容**：支持协议版本协商
- **升级机制**：平滑的协议升级支持