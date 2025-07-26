use anyhow::Result;
use async_trait::async_trait;
use orwell::pb::orwell::{OrwellPacket, PacketType};

use crate::network::Network;

/// Context for packet processing on client side
pub struct ClientPacketContext<'a> {
    pub network: &'a mut Network,
}

/// Trait for client packet adapters
#[async_trait]
pub trait ClientPacketAdapter: Send + Sync {
    /// Get the packet type this adapter handles
    fn packet_type(&self) -> PacketType;

    /// Process the packet
    async fn process(&self, packet: OrwellPacket, context: ClientPacketContext<'_>) -> Result<()>;
}

/// Registry for client packet adapters
pub struct ClientPacketAdapterRegistry {
    adapters: std::collections::HashMap<PacketType, Box<dyn ClientPacketAdapter>>,
}

impl ClientPacketAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Box<dyn ClientPacketAdapter>) {
        self.adapters.insert(adapter.packet_type(), adapter);
    }

    pub fn get(&self, packet_type: PacketType) -> Option<&dyn ClientPacketAdapter> {
        self.adapters.get(&packet_type).map(|a| a.as_ref())
    }
}
