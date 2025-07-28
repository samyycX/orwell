use anyhow::Result;
use async_trait::async_trait;
use orwell::shared::helper::get_now_timestamp;
use orwell::{
    decode_packet,
    pb::orwell::{OrwellPacket, PacketType, ServerRegisterResponse},
};
use prost::Message;

use crate::{
    message::add_chat_message,
    packet_adapter::{ClientPacketAdapter, ClientPacketContext},
    STATE,
};

pub struct RegisterResponseAdapter;

#[async_trait]
impl ClientPacketAdapter for RegisterResponseAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerRegisterResponse
    }

    fn process(&self, packet: OrwellPacket, _context: ClientPacketContext<'_>) -> Result<()> {
        let packet = decode_packet!(packet, ServerRegisterResponse);

        if packet.success {
            add_chat_message(format!(
                "注册成功, 您的聊天颜色为 {}",
                orwell::shared::helper::color_code_to_hex(packet.color)
            ));
            let mut state = STATE.write().unwrap();
            state.connected = true;
            state.start_time = get_now_timestamp();
            drop(state);
        } else {
            add_chat_message(format!("注册失败, 原因: {}", packet.message));
        }

        Ok(())
    }
}
