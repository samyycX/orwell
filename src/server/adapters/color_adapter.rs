use crate::{
    broadcast_message_from_server,
    client::ClientManager,
    packet_adapter::{PacketAdapter, PacketContext},
    send_packet,
};
use anyhow::Result;
use async_trait::async_trait;
use orwell::{
    decode_packet,
    pb::orwell::{
        ClientChangeColor, MessageType, PacketType, ServerBroadcastChangeColor,
        ServerChangeColorResponse,
    },
};
use prost::Message;

pub struct ColorAdapter;

#[async_trait]
impl PacketAdapter for ColorAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ClientChangeColor
    }

    async fn process(
        &self,
        packet: orwell::pb::orwell::OrwellPacket,
        context: PacketContext,
    ) -> Result<()> {
        let packet = decode_packet!(packet, ClientChangeColor);
        let client = context.client_info.as_ref().unwrap().client.clone();
        let clients = ClientManager::get_all_clients().await;

        if clients
            .iter()
            .any(|c| c.client.color_ == packet.color && c.client.id_ != client.id_)
        {
            send_packet(
                context.conn_id,
                PacketType::ServerChangeColorResponse,
                ServerChangeColorResponse {
                    success: false,
                    color: 0,
                    message: "已经有用户在使用此颜色".to_string(),
                },
            )
            .await?;
        } else {
            broadcast_message_from_server(
                MessageType::ChangeColor,
                &ServerBroadcastChangeColor {
                    id: client.id_.clone(),
                    name: client.name_.clone(),
                    old_color: client.color_,
                    new_color: packet.color,
                }
                .encode_to_vec(),
                None,
                None,
                None,
                true,
            )
            .await?;

            ClientManager::update_color(&client.id_, packet.color).await;

            send_packet(
                context.conn_id,
                PacketType::ServerChangeColorResponse,
                ServerChangeColorResponse {
                    success: true,
                    color: packet.color,
                    message: "".to_string(),
                },
            )
            .await?;
        }

        Ok(())
    }
}
