use std::{collections::HashMap, sync::RwLock};

use anyhow::{anyhow, Result};
use color_eyre::owo_colors::OwoColorize;
use lazy_static::lazy_static;
use orwell::{
    pb::orwell::{
        ClientAfk, ClientChangeColor, ClientMessage, ClientStatus, Key, MessageType, OrwellPacket,
        PacketType, ServerBroadcastMessage,
    },
    shared::{encryption::Encryption, helper::get_now_timestamp},
};
use rand::Rng;
use ratatui::style::{Color, Style};

use crate::{
    key::KEY_MANAGER,
    message::{
        add_chat_message, add_chat_message_rich, add_debug_message, LineBuilder, MessageLevel,
    },
    network::{Network, NETWORK},
    App, STATE,
};

use crate::message_adapter::MessageContext;
use crate::message_adapters::create_message_registry;
#[derive(Clone)]
pub struct ClientInfo {
    pub id: String,
    pub name: String,
    pub color: i32,
    pub kyber_pk: Vec<u8>,
    pub status: ClientStatus,
}

lazy_static! {
    pub static ref OTHER_CLIENTS: RwLock<HashMap<String, ClientInfo>> = RwLock::new(HashMap::new());
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

    pub fn add_client(client: ClientInfo) {
        let mut clients = OTHER_CLIENTS.write().unwrap();
        clients.insert(client.id.clone(), client);
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
    #[deprecated(note = "使用命令适配器模式，请使用 COMMAND_REGISTRY 处理命令")]
    pub fn check_command(_command: &str, _app: &mut App) {
        // 此方法已废弃，使用命令适配器模式
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
        let key_manager = KEY_MANAGER.read().unwrap();
        let profile = key_manager.as_ref().unwrap().profile.clone().unwrap();
        drop(key_manager);
        let key = Encryption::kyber_decrypt(&key.ciphertext, profile.kyber_sk.as_slice())?;
        let data = Encryption::aes_decrypt(&packet.data, &key)?;

        let registry = create_message_registry();
        let context = MessageContext { is_history };
        registry.process_message(packet, data, context)
    }

    pub fn handle_packet(packet: OrwellPacket, network: &mut Network) -> Result<()> {
        // 这个方法现在由adapter处理，保留用于向后兼容
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

    pub fn get_online_time(start_time: u64) -> String {
        let milliseconds = get_now_timestamp() - start_time;
        let hours = milliseconds / 3600000;
        let minutes = (milliseconds % 3600000) / 60000;
        let secs = (milliseconds % 60000) / 1000;
        let milliseconds = milliseconds % 1000;
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            hours, minutes, secs, milliseconds
        )
    }
}
