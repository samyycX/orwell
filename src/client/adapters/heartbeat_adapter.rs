use anyhow::Result;
use orwell::pb::orwell::{ClientHeartbeat, OrwellPacket, PacketType};

use crate::packet_adapter::{ClientPacketAdapter, ClientPacketContext};

pub struct HeartbeatAdapter;

impl ClientPacketAdapter for HeartbeatAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerHeartbeat
    }

    fn process(&self, _packet: OrwellPacket, context: ClientPacketContext<'_>) -> Result<()> {
        context
            .network
            .send_packet(PacketType::ClientHeartbeat, ClientHeartbeat {});
        Ok(())
    }
}
