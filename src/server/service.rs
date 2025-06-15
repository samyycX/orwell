use anyhow::{anyhow, Result};
use orwell::{
    pb::orwell::{
        ClientMessage, Message as PbMessage, MessageType, PacketType, ServerBroadcastClientLogin,
        ServerBroadcastClientLogout, ServerBroadcastMessage, ServerHistoryMessage,
    },
    schema::messages_::msg_type_,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::{
    broadcast_message_from_server,
    client::{Client, ClientManager, CLIENT_MANAGER},
    message::MessageManager,
    send_packet, WsSender,
};

pub struct Service {}

impl Service {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn login_client(
        conn_id: u32,
        client: Client,
        ws_sender: Arc<Mutex<WsSender>>,
    ) -> Result<()> {
        ClientManager::login_client(conn_id, client.clone()).await;

        match broadcast_message_from_server(
            MessageType::Login,
            &vec![],
            Some(client.id_.clone()),
            Some(client.name_.clone()),
            Some(client.color_.clone()),
            ws_sender.clone(),
            true,
        )
        .await
        {
            Ok(_) => info!("{} 已登录至服务器", client.name_.clone()),
            Err(e) => error!("{} 退出时广播失败: {:?}", client.name_.clone(), e),
        };

        let mut packet = ServerHistoryMessage { data: vec![] };
        for message in MessageManager::get_history_messages(client.id_, 50).await {
            let client = ClientManager::get_client_by_id(&message.sender_id_).await;
            let client = client.unwrap_or_default();
            let sender_id = client.id_.clone();
            let sender_name = client.name_.clone();
            let color = client.color_.clone();
            packet.data.push(ServerBroadcastMessage {
                sender_id,
                sender_name,
                color,
                data: Some(PbMessage {
                    id: message.receiver_id_,
                    msg_type: message.msg_type_,
                    ciphertext: message.data_,
                    timestamp: message.timestamp_ as u64,
                }),
            });
        }

        send_packet(
            conn_id,
            ws_sender.clone(),
            PacketType::ServerHistoryMessage,
            packet,
        )
        .await
        .map_err(|e| anyhow!("{} 发送历史消息失败: {:?}", client.name_.clone(), e))
    }

    pub async fn logout_client(conn_id: u32, ws_sender: Arc<Mutex<WsSender>>) {
        if let Some(client) = ClientManager::get_client_by_connection(conn_id).await {
            ClientManager::remove_connection(conn_id).await;
            match broadcast_message_from_server(
                MessageType::Logout,
                &vec![],
                Some(client.id_.clone()),
                Some(client.name_.clone()),
                Some(client.color_.clone()),
                ws_sender.clone(),
                true,
            )
            .await
            {
                Ok(_) => info!("{} 已登录至服务器", client.name_.clone()),
                Err(e) => error!("{} 退出时广播失败: {:?}", client.name_.clone(), e),
            }
        }
    }
}
