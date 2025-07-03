use core::net;
use std::{collections::HashMap, sync::RwLock, thread};

use anyhow::{anyhow, Result};
use color_eyre::owo_colors::OwoColorize;
use lazy_static::lazy_static;
use orwell::{
    decode_packet,
    pb::orwell::{
        ClientAfk, ClientChangeColor, ClientHeartbeat, ClientLogin, ClientMessage, ClientRegister,
        ClientStatus, Key, MessageType, OrwellPacket, OrwellRatchetStep, PacketType,
        ServerBroadcastChangeColor, ServerBroadcastMessage, ServerChangeColorResponse,
        ServerClientInfo, ServerHistoryMessage, ServerLoginResponse, ServerPreLogin,
        ServerRegisterResponse,
    },
    shared::{
        encryption::Encryption,
        helper::{color_code_to_hex, get_now_timestamp},
    },
};
use prost::Message as ProstMessage;
use rand::{Rng, RngCore};
use ratatui::{
    crossterm::style::Stylize,
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::{
    key::KeyManager,
    message::{
        self, add_chat_message, add_chat_message_rich, add_debug_message, clear_chat_messages,
        Line, LineBuilder, MessageLevel, TextSpan,
    },
    network::{Network, NETWORK},
    notify::Notifier,
    App, STATE,
};

#[derive(Clone)]
pub struct ClientInfo {
    pub id: String,
    pub name: String,
    pub color: i32,
    pub kyber_pk: Vec<u8>,
    pub status: ClientStatus,
}

lazy_static! {
    static ref OTHER_CLIENTS: RwLock<HashMap<String, ClientInfo>> = RwLock::new(HashMap::new());
}

pub struct ClientManager {}

impl ClientManager {
    pub fn get_all_clients() -> Vec<ClientInfo> {
        let clients = OTHER_CLIENTS.read().unwrap();
        clients.values().cloned().collect()
    }

    pub fn get_all_clients_sorted() -> Vec<ClientInfo> {
        let mut clients = Self::get_all_clients();
        clients.sort_by(|a, b| {
            let status_order = |status: &ClientStatus| match status {
                ClientStatus::Online => 0,
                ClientStatus::Afk => 1,
                ClientStatus::Offline => 2,
            };
            status_order(&a.status).cmp(&status_order(&b.status))
        });
        clients
    }

    pub fn update_color(id: String, color: i32) {
        let mut clients = OTHER_CLIENTS.write().unwrap();
        if let Some(client) = clients.get_mut(&id) {
            client.color = color;
        }
    }
    pub fn update_status(id: String, status: ClientStatus) {
        let mut clients = OTHER_CLIENTS.write().unwrap();
        if let Some(client) = clients.get_mut(&id) {
            client.status = status;
        }
    }
}

pub struct Service {}

impl Service {
    pub fn check_command(command: &str, app: &mut App) {
        let command = command.split_whitespace().collect::<Vec<&str>>();
        match command[0] {
            "/help" => {
                add_chat_message("/register <用户名> <密码> <确认密码>");
                add_chat_message("/login <用户名> <密码>");
                add_chat_message("/connect <服务器地址>");
                add_chat_message("/color <颜色代码>");
            }
            "/register" => {
                if command.len() != 4 {
                    add_chat_message("使用方法: /register <用户名> <密码> <确认密码>");
                } else {
                    let name = command[1];
                    let password = command[2];
                    let confirm_password = command[3];
                    if password != confirm_password {
                        add_chat_message("密码不一致！");
                    } else {
                        add_chat_message("正在创建密钥...此过程需要数十秒，请耐心等候");
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        KeyManager::create_key(name, password);
                    }
                }
            }
            "/login" => {
                if command.len() != 3 {
                    add_chat_message("使用方法: /login <用户名> <密码>");
                } else {
                    let name = command[1];
                    let password = command[2];
                    KeyManager::load_key(name, password);
                }
            }
            "/connect" => {
                if command.len() != 2 {
                    add_chat_message("使用方法: /connect <服务器地址>");
                } else {
                    let server_address = command[1];
                    Network::start(server_address.to_string());
                }
            }
            "/color" => {
                if command.len() != 2 {
                    add_chat_message("使用方法: /color <颜色代码>");
                    return;
                }
                let state = STATE.read().unwrap();
                if !state.connected {
                    add_chat_message("您尚未连接至服务器，无法改变颜色");
                    return;
                }
                let color = command[1];
                if !color.starts_with('#') || color.len() != 7 {
                    add_chat_message("颜色代码必须是 #RRGGBB 格式");
                    return;
                }

                match i32::from_str_radix(&color[1..], 16) {
                    Ok(color) => {
                        Self::change_color(color);
                    }
                    Err(_) => {
                        add_chat_message("颜色代码格式错误，请使用 #RRGGBB 格式");
                    }
                }
            }
            "/afk" => {
                Self::afk();
            }
            _ => {}
        }
    }

    pub fn change_color(color: i32) {
        let mut network = NETWORK.write().unwrap();
        if network.is_none() {
            add_chat_message("尚未连接至服务器，无法改变颜色");
            return;
        }
        let network = network.as_mut().unwrap();
        network.send_packet(PacketType::ClientChangeColor, ClientChangeColor { color });
    }

    pub fn afk() {
        let mut network = NETWORK.write().unwrap();
        if network.is_none() {
            add_chat_message("未连接到服务器，无法设置为离开状态");
            return;
        }
        let network = network.as_mut().unwrap();
        network.send_packet(PacketType::ClientAfk, ClientAfk {});
    }

    pub fn check_login(app: &App) {
        add_debug_message(MessageLevel::Info, "正在检查登录状态...");
        if STATE.read().unwrap().logged {
            add_chat_message("您已经登录了！(｡･ω･｡)");
        } else {
            add_chat_message("使用 /login <用户名> <密码> 以登录");
            add_chat_message("使用 /register <用户名> <密码> <确认密码> 以注册");
            add_chat_message_rich(
                LineBuilder::new()
                    .styled(
                        "使用 /help 以查看更多命令",
                        Style::default().fg(Color::Green),
                    )
                    .build(),
                None,
            );
        }
    }

    pub fn handle_message(packet: &ServerBroadcastMessage, is_history: bool) -> Result<()> {
        let key = packet.key.clone();
        if key.is_none() {
            return Err(anyhow!("数据异常"));
        }
        let key = key.unwrap();
        let profile = KeyManager::get_profile().unwrap();
        let key = Encryption::kyber_decrypt(&key.ciphertext, profile.kyber_sk.as_slice())?;
        let data = Encryption::aes_decrypt(&packet.data, &key)?;
        let (msg_type, data) = data.split_at(1);
        let data = data.to_vec();
        let msg_type = MessageType::try_from(msg_type[0] as i32);
        if msg_type.is_err() {
            return Err(anyhow!("收到未知消息类型"));
        }
        let msg_type = msg_type.unwrap();
        match msg_type {
            MessageType::Text => {
                if !is_history {
                    Notifier::notify_message(
                        &packet.sender_name,
                        &format!("{}", String::from_utf8(data.clone()).unwrap()),
                    );
                }
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(packet.timestamp)
                        .sender(TextSpan::new(
                            packet.sender_name.clone(),
                            Style::default()
                                .fg(Color::from_u32(packet.color as u32))
                                .add_modifier(Modifier::BOLD),
                        ))
                        .plain(String::from_utf8(data).unwrap())
                        .build(),
                    if is_history { Some(0) } else { None },
                );
            }
            MessageType::Login => {
                if !is_history {
                    Notifier::notify_message(
                        &packet.sender_name,
                        &format!("{} 上线了", packet.sender_name),
                    );
                }
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(packet.timestamp)
                        .sender(TextSpan::new(
                            "→",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .colored(
                            packet.sender_name.clone(),
                            Color::from_u32(packet.color as u32),
                        )
                        .plain(" 上线了")
                        .build(),
                    if is_history { Some(0) } else { None },
                );
            }
            MessageType::Logout => {
                if !is_history {
                    Notifier::notify_message(
                        &packet.sender_name,
                        &format!("{} 下线了", packet.sender_name),
                    );
                }
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(packet.timestamp)
                        .sender(TextSpan::new(
                            "←",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ))
                        .colored(
                            packet.sender_name.clone(),
                            Color::from_u32(packet.color as u32),
                        )
                        .plain(" 下线了")
                        .build(),
                    if is_history { Some(0) } else { None },
                );
                ClientManager::update_status(packet.sender_id.clone(), ClientStatus::Offline);
            }
            MessageType::ChangeColor => {
                let data = ServerBroadcastChangeColor::decode(data.as_slice())?;
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(packet.timestamp)
                        .colored(data.name, Color::from_u32(data.new_color as u32))
                        .plain(" 的聊天颜色从 ")
                        .colored(
                            color_code_to_hex(data.old_color),
                            Color::from_u32(data.old_color as u32),
                        )
                        .plain(" 更改至 ")
                        .colored(
                            color_code_to_hex(data.new_color),
                            Color::from_u32(data.new_color as u32),
                        )
                        .build(),
                    if is_history { Some(0) } else { None },
                );
                ClientManager::update_color(data.id.clone(), data.new_color);
            }
            MessageType::EnterAfk => {
                if !is_history {
                    Notifier::notify_message(
                        &packet.sender_name,
                        &format!("{} 进入了AFK状态", packet.sender_name),
                    );
                }
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(packet.timestamp)
                        .sender(TextSpan::new(
                            "\u{f04b2}",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .colored(
                            packet.sender_name.clone(),
                            Color::from_u32(packet.color as u32),
                        )
                        .plain(" 进入了AFK状态")
                        .build(),
                    if is_history { Some(0) } else { None },
                );
            }
            MessageType::LeftAfk => {
                if !is_history {
                    Notifier::notify_message(
                        &packet.sender_name,
                        &format!("{} 离开了AFK状态", packet.sender_name),
                    );
                }
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(packet.timestamp)
                        .sender(TextSpan::new(
                            "\u{f04b3}",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .colored(
                            packet.sender_name.clone(),
                            Color::from_u32(packet.color as u32),
                        )
                        .plain(" 离开了AFK状态")
                        .build(),
                    if is_history { Some(0) } else { None },
                );
            }
            _ => {}
        };

        Ok(())
    }

    pub fn handle_packet(packet: OrwellPacket, network: &mut Network) -> Result<()> {
        let type_ = PacketType::try_from(packet.packet_type);
        if type_.is_err() {
            return Err(anyhow::anyhow!("收到未知消息类型"));
        }
        let type_ = type_.unwrap();
        match type_ {
            PacketType::ServerHeartbeat => {
                network.send_packet(PacketType::ClientHeartbeat, ClientHeartbeat {});
            }
            PacketType::ServerPreLogin => {
                let packet = decode_packet!(packet, ServerPreLogin);
                let profile = KeyManager::get_profile().unwrap();
                if packet.version_mismatch {
                    add_chat_message("服务器版本不匹配，请更新客户端");
                    return Err(anyhow::anyhow!("服务器版本不匹配，请更新客户端"));
                }
                if packet.registered {
                    add_debug_message(MessageLevel::Info, "正在登录...");
                    let profile = KeyManager::get_profile().unwrap();
                    let token = packet.token.unwrap();
                    let token = Encryption::kyber_decrypt(&token, profile.kyber_sk.as_slice())?;

                    let token_sign = Encryption::dilithium_sign(
                        token.as_slice(),
                        profile.dilithium_sk.as_slice(),
                    )?;
                    let login_packet = ClientLogin {
                        token_sign: token_sign,
                    };
                    network.send_packet(PacketType::ClientLogin, login_packet);
                } else {
                    if !packet.can_register {
                        return Err(anyhow::anyhow!("服务器已禁止新用户注册"));
                    }
                    // user is not registered, lets register
                    let packet = ClientRegister {
                        name: profile.name,
                        dilithium_pk: profile.dilithium_pk,
                        kyber_pk: profile.kyber_pk,
                    };
                    network.send_packet(PacketType::ClientRegister, packet);
                }
                clear_chat_messages();
            }
            PacketType::ServerRegisterResponse => {
                let packet = decode_packet!(packet, ServerRegisterResponse);
                if packet.success {
                    add_chat_message(format!(
                        "注册成功, 您的聊天颜色为 {}",
                        color_code_to_hex(packet.color.unwrap())
                    ));
                    let mut state = STATE.write().unwrap();
                    state.connected = true;
                } else {
                    add_chat_message(format!("注册失败, 原因: {}", packet.message.unwrap()));
                }
            }
            PacketType::ServerLoginResponse => {
                let packet = decode_packet!(packet, ServerLoginResponse);
                if packet.success {
                    add_chat_message("登录成功");
                    let mut state = STATE.write().unwrap();
                    state.connected = true;
                } else {
                    add_chat_message(format!("登录失败, 原因: {}", packet.message.unwrap()));
                }
            }
            PacketType::ServerClientInfo => {
                let packet = decode_packet!(packet, ServerClientInfo);
                let other_clients = packet.data;
                for client in other_clients {
                    OTHER_CLIENTS.write().unwrap().insert(
                        client.id.clone(),
                        ClientInfo {
                            id: client.id.clone(),
                            name: client.name.clone(),
                            color: client.color as i32,
                            kyber_pk: client.kyber_pk,
                            status: ClientStatus::try_from(client.status as i32).unwrap(),
                        },
                    );
                }
            }
            PacketType::ServerBroadcastMessage => {
                let packet: ServerBroadcastMessage = decode_packet!(packet, ServerBroadcastMessage);
                Self::handle_message(&packet, false)?;
            }
            PacketType::ServerHistoryMessage => {
                let packet = decode_packet!(packet, ServerHistoryMessage);
                for message in packet.data {
                    Self::handle_message(&message, true)?;
                }
            }
            PacketType::ServerChangeColorResponse => {
                let packet = decode_packet!(packet, ServerChangeColorResponse);
                match packet.success {
                    true => {
                        add_chat_message_rich(
                            LineBuilder::new()
                                .plain("更改颜色至 ")
                                .colored(
                                    format!("{}", color_code_to_hex(packet.color.unwrap())),
                                    Color::from_u32(packet.color.unwrap() as u32),
                                )
                                .plain(" 成功")
                                .build(),
                            None,
                        );
                    }
                    false => {
                        add_chat_message(format!("更改颜色失败: {}", packet.message.unwrap()));
                    }
                }
            }
            PacketType::ServerOrwellRatchetStep => {
                let packet = decode_packet!(packet, OrwellRatchetStep);
                add_debug_message(MessageLevel::Info, "正在轮换...");
                network.ratchet_step(packet.ct)?;
            }
            _ => {}
        };

        Ok(())
    }

    pub fn broadcast_message(message: String) -> Result<()> {
        let mut network = NETWORK.write().unwrap();
        if network.is_none() {
            return Err(anyhow::anyhow!("未连接到服务器"));
        }
        let network = network.as_mut().unwrap();
        let mut message = message.as_bytes().to_vec();
        message.insert(0, MessageType::Text as u8);
        let (keys, data) = Self::broadcast_data(message)?;
        let mut packet = ClientMessage { keys: vec![], data };
        for (id, ciphertext) in keys.iter() {
            packet.keys.push(Key {
                receiver_id: id.clone(),
                ciphertext: ciphertext.clone(),
            });
        }
        add_debug_message(
            MessageLevel::Info,
            format!("正在发送消息到 {} 个客户端", packet.keys.len()),
        );
        network.send_packet(PacketType::ClientMessage, packet);
        Ok(())
    }

    pub fn broadcast_data(data: Vec<u8>) -> Result<(HashMap<String, Vec<u8>>, Vec<u8>)> {
        let mut result = HashMap::new();
        let other_clients = ClientManager::get_all_clients();

        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        let data = Encryption::aes_encrypt(&data, &key);

        for client in other_clients {
            let ciphertext = Encryption::kyber_encrypt(&key, &client.kyber_pk)?;
            result.insert(client.id.clone(), ciphertext);
        }

        Ok((result, data))
    }
}
