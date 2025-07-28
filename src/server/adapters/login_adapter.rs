use crate::{
    packet_adapter::{PacketAdapter, PacketContext},
    send_packet,
    service::Service,
    token::TokenManager,
};
use anyhow::Result;
use async_trait::async_trait;
use orwell::{
    decode_packet,
    pb::orwell::{ClientLogin, PacketType, ServerLoginResponse},
};
use prost::Message;

pub struct LoginAdapter;

#[async_trait]
impl PacketAdapter for LoginAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ClientLogin
    }

    async fn process(
        &self,
        packet: orwell::pb::orwell::OrwellPacket,
        context: PacketContext,
    ) -> Result<()> {
        let packet = decode_packet!(packet, ClientLogin);
        let token = TokenManager::validate_token(context.conn_id, &packet.token_sign).await;
        let mut login_client = None;

        let response = if token.is_none() {
            ServerLoginResponse {
                success: false,
                message: "身份校验失败".to_string(),
            }
        } else {
            let pk = token.clone().unwrap().1;
            let client = crate::client::ClientManager::find_client(&pk).unwrap();
            login_client.replace(client);
            ServerLoginResponse {
                success: true,
                message: "登录成功".to_string(),
            }
        };

        send_packet(context.conn_id, PacketType::ServerLoginResponse, response).await?;

        if let Some(client) = login_client {
            Service::login_client(context.conn_id, client).await?;
        }

        Ok(())
    }
}
