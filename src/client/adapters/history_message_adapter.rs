use anyhow::Result;
use async_trait::async_trait;
use orwell::{
    decode_packet,
    pb::orwell::{OrwellPacket, PacketType, ServerHistoryMessage},
};
use prost::Message;

use crate::{
    packet_adapter::{ClientPacketAdapter, ClientPacketContext},
    service::Service,
};

pub struct HistoryMessageAdapter;

impl ClientPacketAdapter for HistoryMessageAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerHistoryMessage
    }

    fn process(&self, packet: OrwellPacket, _context: ClientPacketContext<'_>) -> Result<()> {
        let packet = decode_packet!(packet, ServerHistoryMessage);
        for message in packet.data {
            Service::handle_message(&message, true)?;
        }
        Ok(())
    }
}
