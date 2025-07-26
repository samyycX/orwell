use anyhow::Result;
use orwell::shared::helper::get_now_timestamp;
use orwell::{
    decode_packet,
    pb::orwell::{OrwellPacket, PacketType, ServerLoginResponse},
};
use prost::Message;

use crate::{
    message::add_chat_message,
    packet_adapter::{ClientPacketAdapter, ClientPacketContext},
    STATE,
};

pub struct LoginResponseAdapter;

impl ClientPacketAdapter for LoginResponseAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerLoginResponse
    }

    fn process(&self, packet: OrwellPacket, _context: ClientPacketContext<'_>) -> Result<()> {
        let packet = decode_packet!(packet, ServerLoginResponse);

        if packet.success {
            add_chat_message("登录成功");
            let mut state: std::sync::RwLockWriteGuard<'_, crate::State> = STATE.write().unwrap();
            state.connected = true;
            state.start_time = get_now_timestamp();
            drop(state);
        } else {
            add_chat_message(format!("登录失败, 原因: {}", packet.message.unwrap()));
        }

        Ok(())
    }
}
