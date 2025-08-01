// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientInfo {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(uint32, tag = "3")]
    pub color: u32,
    #[prost(bytes = "vec", tag = "4")]
    pub kyber_pk: ::prost::alloc::vec::Vec<u8>,
    #[prost(enumeration = "ClientStatus", tag = "5")]
    pub status: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Profile {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub kyber_pk: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub kyber_sk: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub dilithium_pk: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub dilithium_sk: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OrwellRatchetPacket {
    #[prost(bytes = "vec", tag = "1")]
    pub kyber_pk: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub send_counter: u64,
    #[prost(uint64, tag = "3")]
    pub recv_counter: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OrwellPacket {
    #[prost(uint64, tag = "1")]
    pub timestamp: u64,
    #[prost(bytes = "vec", tag = "2")]
    pub salt: ::prost::alloc::vec::Vec<u8>,
    #[prost(enumeration = "PacketType", tag = "3")]
    pub packet_type: i32,
    #[prost(bytes = "vec", tag = "4")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OrwellSignedPacket {
    #[prost(message, optional, tag = "1")]
    pub data: ::core::option::Option<OrwellPacket>,
    #[prost(bytes = "vec", tag = "2")]
    pub sign: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ClientHeartbeat {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientError {
    #[prost(string, tag = "1")]
    pub error: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientInformation {
    #[prost(string, tag = "1")]
    pub information: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientHello {
    #[prost(bytes = "vec", tag = "1")]
    pub pk: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientHello2 {
    #[prost(bytes = "vec", tag = "1")]
    pub ciphertext: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientPreLogin {
    #[prost(bytes = "vec", tag = "1")]
    pub dilithium_pk: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub version: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientRegister {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub kyber_pk: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub dilithium_pk: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientLogin {
    #[prost(bytes = "vec", tag = "1")]
    pub token_sign: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Key {
    #[prost(string, tag = "1")]
    pub receiver_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub ciphertext: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientMessage {
    #[prost(message, repeated, tag = "1")]
    pub keys: ::prost::alloc::vec::Vec<Key>,
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ClientChangeColor {
    #[prost(int32, tag = "1")]
    pub color: i32,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ClientAfk {}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ServerHeartbeat {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerError {
    #[prost(string, tag = "1")]
    pub error: ::prost::alloc::string::String,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ServerInformation {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerHello {
    #[prost(bytes = "vec", tag = "1")]
    pub ciphertext: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub pk: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub dilithium_pk: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerPreLogin {
    #[prost(bool, tag = "1")]
    pub registered: bool,
    #[prost(bool, tag = "2")]
    pub can_register: bool,
    #[prost(bytes = "vec", tag = "3")]
    pub token: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "4")]
    pub version_mismatch: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerRegisterResponse {
    #[prost(bool, tag = "1")]
    pub success: bool,
    #[prost(int32, tag = "2")]
    pub color: i32,
    #[prost(string, tag = "3")]
    pub message: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerLoginResponse {
    #[prost(bool, tag = "1")]
    pub success: bool,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerClientInfo {
    #[prost(message, repeated, tag = "1")]
    pub data: ::prost::alloc::vec::Vec<ClientInfo>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerBroadcastMessage {
    #[prost(string, tag = "1")]
    pub sender_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub sender_name: ::prost::alloc::string::String,
    #[prost(int32, tag = "3")]
    pub color: i32,
    #[prost(message, optional, tag = "4")]
    pub key: ::core::option::Option<Key>,
    #[prost(bytes = "vec", tag = "5")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub timestamp: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerBroadcastClientLogin {
    #[prost(string, tag = "1")]
    pub sender_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub sender_name: ::prost::alloc::string::String,
    #[prost(int32, tag = "3")]
    pub color: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerBroadcastClientLogout {
    #[prost(string, tag = "1")]
    pub sender_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub sender_name: ::prost::alloc::string::String,
    #[prost(int32, tag = "3")]
    pub color: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerHistoryMessage {
    #[prost(message, repeated, tag = "1")]
    pub data: ::prost::alloc::vec::Vec<ServerBroadcastMessage>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerChangeColorResponse {
    #[prost(bool, tag = "1")]
    pub success: bool,
    #[prost(int32, tag = "2")]
    pub color: i32,
    #[prost(string, tag = "3")]
    pub message: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerBroadcastChangeColor {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(int32, tag = "3")]
    pub old_color: i32,
    #[prost(int32, tag = "4")]
    pub new_color: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OrwellRatchetStep {
    #[prost(bytes = "vec", tag = "1")]
    pub ct: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum PacketType {
    ClientHeartbeat = 0,
    ClientError = 1,
    ClientInformation = 2,
    ClientHello = 3,
    ClientPreLogin = 4,
    ClientRegister = 5,
    ClientLogin = 6,
    ClientMessage = 7,
    ClientChangeColor = 8,
    ClientAfk = 9,
    ServerHeartbeat = 10000,
    ServerError = 10001,
    ServerInformation = 10002,
    ServerHello = 10003,
    ServerPreLogin = 10004,
    ServerRegisterResponse = 10005,
    ServerLoginResponse = 10006,
    ServerClientInfo = 10007,
    ServerBroadcastMessage = 10008,
    ServerHistoryMessage = 10009,
    ServerChangeColorResponse = 10010,
    ServerOrwellRatchetStep = 10011,
}
impl PacketType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::ClientHeartbeat => "Client_Heartbeat",
            Self::ClientError => "Client_Error",
            Self::ClientInformation => "Client_Information",
            Self::ClientHello => "Client_Hello",
            Self::ClientPreLogin => "Client_PreLogin",
            Self::ClientRegister => "Client_Register",
            Self::ClientLogin => "Client_Login",
            Self::ClientMessage => "Client_Message",
            Self::ClientChangeColor => "Client_ChangeColor",
            Self::ClientAfk => "Client_Afk",
            Self::ServerHeartbeat => "Server_Heartbeat",
            Self::ServerError => "Server_Error",
            Self::ServerInformation => "Server_Information",
            Self::ServerHello => "Server_Hello",
            Self::ServerPreLogin => "Server_PreLogin",
            Self::ServerRegisterResponse => "Server_RegisterResponse",
            Self::ServerLoginResponse => "Server_LoginResponse",
            Self::ServerClientInfo => "Server_ClientInfo",
            Self::ServerBroadcastMessage => "Server_BroadcastMessage",
            Self::ServerHistoryMessage => "Server_HistoryMessage",
            Self::ServerChangeColorResponse => "Server_ChangeColorResponse",
            Self::ServerOrwellRatchetStep => "Server_OrwellRatchetStep",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "Client_Heartbeat" => Some(Self::ClientHeartbeat),
            "Client_Error" => Some(Self::ClientError),
            "Client_Information" => Some(Self::ClientInformation),
            "Client_Hello" => Some(Self::ClientHello),
            "Client_PreLogin" => Some(Self::ClientPreLogin),
            "Client_Register" => Some(Self::ClientRegister),
            "Client_Login" => Some(Self::ClientLogin),
            "Client_Message" => Some(Self::ClientMessage),
            "Client_ChangeColor" => Some(Self::ClientChangeColor),
            "Client_Afk" => Some(Self::ClientAfk),
            "Server_Heartbeat" => Some(Self::ServerHeartbeat),
            "Server_Error" => Some(Self::ServerError),
            "Server_Information" => Some(Self::ServerInformation),
            "Server_Hello" => Some(Self::ServerHello),
            "Server_PreLogin" => Some(Self::ServerPreLogin),
            "Server_RegisterResponse" => Some(Self::ServerRegisterResponse),
            "Server_LoginResponse" => Some(Self::ServerLoginResponse),
            "Server_ClientInfo" => Some(Self::ServerClientInfo),
            "Server_BroadcastMessage" => Some(Self::ServerBroadcastMessage),
            "Server_HistoryMessage" => Some(Self::ServerHistoryMessage),
            "Server_ChangeColorResponse" => Some(Self::ServerChangeColorResponse),
            "Server_OrwellRatchetStep" => Some(Self::ServerOrwellRatchetStep),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ClientStatus {
    Online = 0,
    Offline = 1,
    Afk = 2,
}
impl ClientStatus {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::Online => "Online",
            Self::Offline => "Offline",
            Self::Afk => "Afk",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "Online" => Some(Self::Online),
            "Offline" => Some(Self::Offline),
            "Afk" => Some(Self::Afk),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MessageType {
    Text = 0,
    Login = 1,
    Logout = 2,
    ChangeColor = 3,
    Me = 4,
    EnterAfk = 5,
    LeftAfk = 6,
    Image = 7,
}
impl MessageType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Login => "Login",
            Self::Logout => "Logout",
            Self::ChangeColor => "ChangeColor",
            Self::Me => "Me",
            Self::EnterAfk => "EnterAfk",
            Self::LeftAfk => "LeftAfk",
            Self::Image => "Image",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "Text" => Some(Self::Text),
            "Login" => Some(Self::Login),
            "Logout" => Some(Self::Logout),
            "ChangeColor" => Some(Self::ChangeColor),
            "Me" => Some(Self::Me),
            "EnterAfk" => Some(Self::EnterAfk),
            "LeftAfk" => Some(Self::LeftAfk),
            "Image" => Some(Self::Image),
            _ => None,
        }
    }
}
