use anyhow::{anyhow, Result};
use orwell::pb::orwell::{
    Key, MessageType, PacketType, ServerBroadcastMessage, ServerClientInfo, ServerHistoryMessage,
};

use crate::{
    broadcast_message_from_server,
    client::{Client, ClientManager},
    message::MessageManager,
    send_packet,
};

pub struct Service {}

impl Service {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn login_client(conn_id: u32, client: Client) -> Result<()> {
        let login_client_info = ClientManager::login_client(conn_id, client.clone()).await;

        broadcast_message_from_server(
            MessageType::Login,
            &[],
            Some(client.id_.clone()),
            Some(client.name_.clone()),
            Some(client.color_),
            true,
        )
        .await?;

        let mut packet = ServerHistoryMessage { data: vec![] };
        for (message, key) in MessageManager::get_history_messages(client.id_.clone(), 50).await {
            let client = ClientManager::get_client_by_id(&message.sender_id_).await;
            let client = client.unwrap_or_default();
            let sender_id = client.id_.clone();
            let sender_name = client.name_.clone();
            let color = client.color_;
            packet.data.push(ServerBroadcastMessage {
                sender_id,
                sender_name,
                color,
                data: message.data_,
                key: Some(Key {
                    receiver_id: key.receiver_id_,
                    ciphertext: key.data_,
                }),
                timestamp: message.timestamp_ as u64,
            });
        }

        send_packet(conn_id, PacketType::ServerHistoryMessage, packet)
            .await
            .map_err(|e| anyhow!("{} 发送历史消息失败: {:?}", client.name_.clone(), e))?;

        let infos = ClientManager::get_all_clients()
            .await
            .into_iter()
            .map(|info: crate::client::ClientInfo| info.to_pb_client_info())
            .collect::<Vec<_>>();

        for online_client_info in ClientManager::get_all_online_clients().await {
            let conn_id =
                ClientManager::get_client_connection_by_id(&online_client_info.client.id_)
                    .await
                    .unwrap();
            send_packet(
                conn_id,
                PacketType::ServerClientInfo,
                ServerClientInfo {
                    data: infos.clone(),
                },
            )
            .await?;
        }

        Ok(())
    }

    pub async fn logout_client(conn_id: u32) -> Result<()> {
        if let Some(client_info) = ClientManager::get_client_by_connection(conn_id).await {
            let client = client_info.client;
            ClientManager::remove_connection(conn_id).await;
            broadcast_message_from_server(
                MessageType::Logout,
                &[],
                Some(client.id_.clone()),
                Some(client.name_.clone()),
                Some(client.color_),
                true,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn broadcast_resync_client() -> Result<()> {
        let infos = ClientManager::get_all_clients()
            .await
            .into_iter()
            .map(|info| info.to_pb_client_info())
            .collect::<Vec<_>>();

        for online_client_info in ClientManager::get_all_online_clients().await {
            let conn_id: u32 =
                ClientManager::get_client_connection_by_id(&online_client_info.client.id_)
                    .await
                    .unwrap();
            send_packet(
                conn_id,
                PacketType::ServerClientInfo,
                ServerClientInfo {
                    data: infos.clone(),
                },
            )
            .await?;
        }
        Ok(())
    }
}
