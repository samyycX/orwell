use crate::{
    client::ClientManager,
    message::MessageManager,
    packet_adapter::{PacketAdapter, PacketContext},
    send_packet,
};
use anyhow::Result;
use async_trait::async_trait;
use orwell::{
    decode_packet,
    pb::orwell::{ClientMessage, PacketType, ServerBroadcastMessage},
    shared::helper::get_now_timestamp,
};
use prost::Message;

pub struct MessageAdapter;

#[async_trait]
impl PacketAdapter for MessageAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ClientMessage
    }

    async fn process(
        &self,
        packet: orwell::pb::orwell::OrwellPacket,
        context: PacketContext,
    ) -> Result<()> {
        let packet = decode_packet!(packet, ClientMessage);
        let sender = context.client_info.as_ref().unwrap().client.clone();
        let data = packet.data;

        for key in &packet.keys {
            let client = ClientManager::get_client_by_id(&key.receiver_id).await;
            if client.is_none() {
                continue;
            }
            let client = client.unwrap();
            if let Some(conn_id) =
                ClientManager::get_client_connection_by_id(&client.id_.clone()).await
            {
                send_packet(
                    conn_id,
                    PacketType::ServerBroadcastMessage,
                    ServerBroadcastMessage {
                        sender_id: sender.id_.clone(),
                        sender_name: sender.name_.clone(),
                        key: Some(key.clone()),
                        data: data.clone(),
                        color: sender.color_,
                        timestamp: get_now_timestamp(),
                    },
                )
                .await?;
            }
        }

        MessageManager::add_message(sender.id_.clone(), data.clone(), packet.keys).await;
        Ok(())
    }
}
