use crate::{
    client::ClientManager,
    packet_adapter::{PacketAdapter, PacketContext},
    send_packet,
    token::TokenManager,
};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::SinkExt;
use orwell::{
    decode_packet,
    pb::orwell::{ClientPreLogin, PacketType, ServerPreLogin},
};
use prost::Message;

pub struct PreLoginAdapter;

#[async_trait]
impl PacketAdapter for PreLoginAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ClientPreLogin
    }

    async fn process(
        &self,
        packet: orwell::pb::orwell::OrwellPacket,
        context: PacketContext,
    ) -> Result<()> {
        let packet = decode_packet!(packet, ClientPreLogin);
        let client = ClientManager::find_client(&packet.dilithium_pk);

        if orwell::shared::helper::get_version() != packet.version {
            let response = ServerPreLogin {
                registered: false,
                can_register: false,
                token: None,
                version_mismatch: true,
            };
            send_packet(context.conn_id, PacketType::ServerPreLogin, response).await?;
            context.ws_sender.lock().await.close().await?;
            return Ok(());
        }

        let response = if client.is_none() {
            ServerPreLogin {
                registered: false,
                can_register: true,
                token: None,
                version_mismatch: false,
            }
        } else {
            let token = TokenManager::generate_token(context.conn_id, &packet.dilithium_pk).await?;
            let token = orwell::shared::encryption::Encryption::kyber_encrypt(
                &token,
                &client.unwrap().kyber_pk_,
            )?;
            ServerPreLogin {
                registered: true,
                can_register: false,
                token: Some(token),
                version_mismatch: false,
            }
        };

        send_packet(context.conn_id, PacketType::ServerPreLogin, response).await?;
        Ok(())
    }
}
