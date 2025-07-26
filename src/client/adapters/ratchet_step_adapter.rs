use anyhow::Result;
use orwell::{
    decode_packet,
    pb::orwell::{OrwellPacket, OrwellRatchetStep, PacketType},
};

use crate::{
    message::{add_debug_message, MessageLevel},
    packet_adapter::{ClientPacketAdapter, ClientPacketContext},
};
use prost::Message;

pub struct RatchetStepAdapter;

impl ClientPacketAdapter for RatchetStepAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerOrwellRatchetStep
    }

    fn process(&self, packet: OrwellPacket, context: ClientPacketContext<'_>) -> Result<()> {
        let packet = decode_packet!(packet, OrwellRatchetStep);
        add_debug_message(MessageLevel::Info, "正在轮换...");
        context.network.ratchet_step(packet.ct)?;
        Ok(())
    }
}
