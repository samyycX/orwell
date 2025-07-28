use anyhow::Result;
use orwell::{
    decode_packet,
    pb::orwell::{ClientRegister, OrwellPacket, PacketType, ServerPreLogin},
};

use crate::key;
use crate::{
    message::{add_chat_message, add_debug_message, clear_chat_messages, MessageLevel},
    packet_adapter::{ClientPacketAdapter, ClientPacketContext},
};
use prost::Message;

pub struct PreLoginAdapter;

impl ClientPacketAdapter for PreLoginAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerPreLogin
    }

    fn process(&self, packet: OrwellPacket, context: ClientPacketContext<'_>) -> Result<()> {
        let packet = decode_packet!(packet, ServerPreLogin);
        let key_manager = key::KEY_MANAGER.read().unwrap();
        let profile = key_manager.as_ref().unwrap().profile.clone().unwrap();
        drop(key_manager);

        if packet.version_mismatch {
            add_chat_message("服务器版本不匹配，请更新客户端");
            return Err(anyhow::anyhow!("服务器版本不匹配，请更新客户端"));
        }

        if packet.registered {
            add_debug_message(MessageLevel::Info, "正在登录...");
            let token = packet.token;
            let token = orwell::shared::encryption::Encryption::kyber_decrypt(
                &token,
                profile.kyber_sk.as_slice(),
            )?;

            let token_sign = orwell::shared::encryption::Encryption::dilithium_sign(
                token.as_slice(),
                profile.dilithium_sk.as_slice(),
            )?;

            let login_packet = orwell::pb::orwell::ClientLogin { token_sign };
            context
                .network
                .send_packet(PacketType::ClientLogin, login_packet);
        } else {
            if !packet.can_register {
                return Err(anyhow::anyhow!("服务器已禁止新用户注册"));
            }

            let register_packet = ClientRegister {
                name: profile.name,
                dilithium_pk: profile.dilithium_pk,
                kyber_pk: profile.kyber_pk,
            };
            context
                .network
                .send_packet(PacketType::ClientRegister, register_packet);
        }

        clear_chat_messages();
        Ok(())
    }
}
