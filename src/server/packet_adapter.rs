use anyhow::Result;
use async_trait::async_trait;
use orwell::pb::orwell::{OrwellPacket, OrwellSignedPacket, PacketType};

use crate::{client::ClientInfo, WsSender};

/// Context for packet processing
pub struct PacketContext {
    pub conn_id: u32,
    pub ws_sender: std::sync::Arc<tokio::sync::Mutex<WsSender>>,
    pub client_info: Option<ClientInfo>,
}

/// Trait for packet adapters
#[async_trait]
pub trait PacketAdapter: Send + Sync {
    /// Get the packet type this adapter handles
    fn packet_type(&self) -> PacketType;

    /// Process the packet
    async fn process(&self, packet: OrwellPacket, context: PacketContext) -> Result<()>;
}

/// Registry for packet adapters
pub struct PacketAdapterRegistry {
    adapters: std::collections::HashMap<PacketType, Box<dyn PacketAdapter>>,
}

impl PacketAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Box<dyn PacketAdapter>) {
        self.adapters.insert(adapter.packet_type(), adapter);
    }

    pub fn get(&self, packet_type: PacketType) -> Option<&dyn PacketAdapter> {
        self.adapters.get(&packet_type).map(|a| a.as_ref())
    }

    pub fn get_all_types(&self) -> Vec<PacketType> {
        self.adapters.keys().cloned().collect()
    }
}
