use crate::{
    client::{Client, ClientManager},
    packet_adapter::{PacketAdapter, PacketContext},
    send_packet,
    service::Service,
};
use anyhow::Result;
use async_trait::async_trait;
use orwell::{
    decode_packet,
    pb::orwell::{ClientRegister, PacketType, ServerRegisterResponse},
};
use prost::Message;
use rand::Rng;

pub struct RegisterAdapter;

#[async_trait]
impl PacketAdapter for RegisterAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ClientRegister
    }

    async fn process(
        &self,
        packet: orwell::pb::orwell::OrwellPacket,
        context: PacketContext,
    ) -> Result<()> {
        let packet = decode_packet!(packet, ClientRegister);
        let client = ClientManager::find_client(&packet.dilithium_pk);
        let mut registered_client = None;

        let response = if client.is_some() {
            ServerRegisterResponse {
                success: false,
                color: None,
                message: Some("您已经注册过了".to_string()),
            }
        } else if ClientManager::is_name_taken(&packet.name) {
            ServerRegisterResponse {
                success: false,
                color: None,
                message: Some("该用户名已被占用".to_string()),
            }
        } else {
            let color = rand::thread_rng().gen_range(0..0x00FFFFFF);
            let client = ClientManager::register_client(
                &packet.name,
                &packet.kyber_pk,
                &packet.dilithium_pk,
                color,
            );
            registered_client.replace(client);
            ServerRegisterResponse {
                success: true,
                color: Some(color),
                message: Some("注册成功".to_string()),
            }
        };

        send_packet(
            context.conn_id,
            PacketType::ServerRegisterResponse,
            response,
        )
        .await?;

        if let Some(client) = registered_client {
            Service::login_client(context.conn_id, client.clone()).await?;
        }

        Ok(())
    }
}
