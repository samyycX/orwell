use crate::{
    broadcast_message_from_server,
    client::ClientManager,
    packet_adapter::{PacketAdapter, PacketContext},
    service::Service,
};
use anyhow::Result;
use async_trait::async_trait;
use orwell::{
    decode_packet,
    pb::orwell::{ClientAfk, ClientStatus, PacketType},
};
use prost::Message;

use orwell::pb::orwell::MessageType;
use orwell::shared::helper::get_now_timestamp;
pub struct AfkAdapter;

#[async_trait]
impl PacketAdapter for AfkAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ClientAfk
    }

    async fn process(
        &self,
        packet: orwell::pb::orwell::OrwellPacket,
        context: PacketContext,
    ) -> Result<()> {
        let _packet = decode_packet!(packet, ClientAfk);
        let client = context.client_info.as_ref().unwrap().client.clone();
        let status = ClientManager::get_status(context.conn_id).await;

        if status == ClientStatus::Afk {
            ClientManager::update_status(context.conn_id, ClientStatus::Online).await;
            broadcast_message_from_server(
                MessageType::LeftAfk,
                &[],
                Some(client.id_.clone()),
                Some(client.name_.clone()),
                Some(client.color_),
                false,
            )
            .await?;
        } else {
            ClientManager::update_status(context.conn_id, ClientStatus::Afk).await;
            broadcast_message_from_server(
                MessageType::EnterAfk,
                &[],
                Some(client.id_.clone()),
                Some(client.name_.clone()),
                Some(client.color_),
                false,
            )
            .await?;
        }

        Service::broadcast_resync_client().await?;
        Ok(())
    }
}
