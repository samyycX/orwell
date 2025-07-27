use anyhow::Result;
use orwell::{
    decode_packet,
    pb::orwell::{OrwellPacket, PacketType, ServerBroadcastMessage},
};

use crate::{
    packet_adapter::{ClientPacketAdapter, ClientPacketContext},
    service::Service,
};
use prost::Message;

pub struct BroadcastMessageAdapter;

impl ClientPacketAdapter for BroadcastMessageAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerBroadcastMessage
    }

    fn process(&self, packet: OrwellPacket, _context: ClientPacketContext<'_>) -> Result<()> {
        let packet: ServerBroadcastMessage = decode_packet!(packet, ServerBroadcastMessage);
        Service::handle_message(&packet, false)?;
        Ok(())
    }
}
