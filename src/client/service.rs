use core::net;
use std::{collections::HashMap, sync::RwLock, thread};

use anyhow::{anyhow, Result};
use color_eyre::owo_colors::OwoColorize;
use lazy_static::lazy_static;
use orwell::{
    decode_packet,
    pb::orwell::{
        ClientChangeColor, ClientLogin, ClientMessage, ClientRegister, Message, MessageType,
        OrwellPacket, PacketType, ServerBroadcastChangeColor, ServerBroadcastMessage,
        ServerChangeColorResponse, ServerHistoryMessage, ServerLoginResponse, ServerOtherClientPk,
        ServerPreLogin, ServerRegisterResponse,
    },
    shared::{
        encryption::Encryption,
        helper::{color_code_to_hex, get_now_timestamp},
    },
};
use prost::Message as ProstMessage;
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
    App, STATE,
};

struct OtherClient {
    id: String,
    kyber_pk: Vec<u8>,
}

lazy_static! {
    static ref OTHER_CLIENTS: RwLock<HashMap<String, OtherClient>> = RwLock::new(HashMap::new());
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
            _ => {}
        }
    }

    pub fn change_color(color: i32) {
        let network = NETWORK.read().unwrap();
        if network.is_none() {
            add_chat_message("尚未连接至服务器，无法改变颜色");
        }
        let network = network.as_ref().unwrap();
        network.send_packet(PacketType::ClientChangeColor, ClientChangeColor { color });
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

    pub fn handle_message(packet: &ServerBroadcastMessage, is_reversed: bool) -> Result<()> {
        if let None = packet.data {
            return Err(anyhow!("数据为空"));
        }
        let message = packet.data.clone().unwrap();
        let msg_type = MessageType::try_from(message.msg_type);
        if msg_type.is_err() {
            return Err(anyhow!("收到未知消息类型"));
        }
        let msg_type = msg_type.unwrap();
        let profile = KeyManager::get_profile().unwrap();
        match msg_type {
            MessageType::Text => {
                let text =
                    Encryption::kyber_decrypt(&message.ciphertext, profile.kyber_sk.as_slice())?;
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(message.timestamp)
                        .sender(TextSpan::new(
                            packet.sender_name.clone(),
                            Style::default()
                                .fg(Color::from_u32(packet.color as u32))
                                .add_modifier(Modifier::BOLD),
                        ))
                        .plain(String::from_utf8(text).unwrap())
                        .build(),
                    if is_reversed { Some(0) } else { None },
                );
            }
            MessageType::Login => {
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(message.timestamp)
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
                    if is_reversed { Some(0) } else { None },
                );
            }
            MessageType::Logout => {
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(message.timestamp)
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
                    if is_reversed { Some(0) } else { None },
                );
            }
            MessageType::ChangeColor => {
                if packet.data.is_none() {
                    return Err(anyhow!("数据异常"));
                }
                let data = packet.data.as_ref().unwrap();
                let data =
                    Encryption::kyber_decrypt(&data.ciphertext, profile.kyber_sk.as_slice())?;
                let packet = ServerBroadcastChangeColor::decode(data.as_slice())?;
                add_chat_message_rich(
                    LineBuilder::new()
                        .time(message.timestamp)
                        .colored(packet.name, Color::from_u32(packet.new_color as u32))
                        .plain(" 的聊天颜色从 ")
                        .colored(
                            color_code_to_hex(packet.old_color),
                            Color::from_u32(packet.old_color as u32),
                        )
                        .plain(" 更改至 ")
                        .colored(
                            color_code_to_hex(packet.new_color),
                            Color::from_u32(packet.new_color as u32),
                        )
                        .build(),
                    if is_reversed { Some(0) } else { None },
                );
            }
            _ => {}
        };

        Ok(())
    }

    pub fn handle_packet(packet: OrwellPacket, network: &Network) -> Result<()> {
        let type_ = PacketType::try_from(packet.packet_type);
        if type_.is_err() {
            return Err(anyhow::anyhow!("收到未知消息类型"));
        }
        let type_ = type_.unwrap();
        match type_ {
            PacketType::ServerPreLogin => {
                let packet = decode_packet!(packet, ServerPreLogin);
                let profile = KeyManager::get_profile().unwrap();
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
                    let other_clients = packet.information.unwrap().other_clients_pk;
                    for client in other_clients {
                        OTHER_CLIENTS.write().unwrap().insert(
                            client.id.clone(),
                            OtherClient {
                                id: client.id.clone(),
                                kyber_pk: client.kyber_pk,
                            },
                        );
                    }
                } else {
                    add_chat_message(format!("注册失败, 原因: {}", packet.message.unwrap()));
                }
            }
            PacketType::ServerLoginResponse => {
                let packet = decode_packet!(packet, ServerLoginResponse);
                if packet.success {
                    add_chat_message("登录成功");
                    let other_clients = packet.information.unwrap().other_clients_pk;
                    for client in other_clients {
                        OTHER_CLIENTS.write().unwrap().insert(
                            client.id.clone(),
                            OtherClient {
                                id: client.id.clone(),
                                kyber_pk: client.kyber_pk,
                            },
                        );
                    }
                    let mut state = STATE.write().unwrap();
                    state.connected = true;
                } else {
                    add_chat_message(format!("登录失败, 原因: {}", packet.message.unwrap()));
                }
            }
            PacketType::ServerOtherClientPk => {
                let packet = decode_packet!(packet, ServerOtherClientPk);
                let mut other_clients = OTHER_CLIENTS.write().unwrap();
                other_clients.insert(
                    packet.id.clone(),
                    OtherClient {
                        id: packet.id.clone(),
                        kyber_pk: packet.kyber_pk,
                    },
                );
            }
            PacketType::ServerBroadcastMessage => {
                let packet = decode_packet!(packet, ServerBroadcastMessage);
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
            _ => {}
        };

        Ok(())
    }

    pub fn broadcast_message(message: String) -> Result<()> {
        let network = NETWORK.read().unwrap();
        if network.is_none() {
            return Err(anyhow::anyhow!("未连接到服务器"));
        }
        let network = network.as_ref().unwrap();
        let data = Self::broadcast_data(message.as_bytes().to_vec())?;
        let mut packet = ClientMessage { data: vec![] };
        let timestamp = get_now_timestamp();
        for (id, ciphertext) in data.iter() {
            packet.data.push(Message {
                id: id.clone(),
                ciphertext: ciphertext.clone(),
                msg_type: MessageType::Text as i32,
                timestamp,
            });
        }
        add_debug_message(
            MessageLevel::Info,
            format!("正在发送消息到 {} 个客户端", packet.data.len()),
        );
        network.send_packet(PacketType::ClientMessage, packet);
        Ok(())
    }

    pub fn broadcast_data(data: Vec<u8>) -> Result<HashMap<String, Vec<u8>>> {
        let mut result = HashMap::new();
        let other_clients = OTHER_CLIENTS.read().unwrap();
        for (id, other_client) in other_clients.iter() {
            let ciphertext = Encryption::kyber_encrypt(&data, &other_client.kyber_pk)?;
            result.insert(id.clone(), ciphertext);
        }

        Ok(result)
    }
}
