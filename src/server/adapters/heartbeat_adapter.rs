use anyhow::Result;
use async_trait::async_trait;
use orwell::pb::orwell::PacketType;

use crate::packet_adapter::{PacketAdapter, PacketContext};

pub struct HeartbeatAdapter;

#[async_trait]
impl PacketAdapter for HeartbeatAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ClientHeartbeat
    }

    async fn process(
        &self,
        _packet: orwell::pb::orwell::OrwellPacket,
        context: PacketContext,
    ) -> Result<()> {
        // do nothing for now.
        Ok(())
    }
}
