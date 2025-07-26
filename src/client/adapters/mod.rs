pub mod broadcast_message_adapter;
pub mod client_info_adapter;
pub mod color_response_adapter;
pub mod heartbeat_adapter;
pub mod history_message_adapter;
pub mod login_response_adapter;
pub mod pre_login_adapter;
pub mod ratchet_step_adapter;
pub mod register_response_adapter;

use crate::adapters::{
    broadcast_message_adapter::BroadcastMessageAdapter, client_info_adapter::ClientInfoAdapter,
    color_response_adapter::ColorResponseAdapter, heartbeat_adapter::HeartbeatAdapter,
    history_message_adapter::HistoryMessageAdapter, login_response_adapter::LoginResponseAdapter,
    pre_login_adapter::PreLoginAdapter, ratchet_step_adapter::RatchetStepAdapter,
    register_response_adapter::RegisterResponseAdapter,
};
use crate::packet_adapter::{ClientPacketAdapterRegistry, ClientPacketContext};

/// Create and register all client packet adapters
pub fn create_client_registry() -> ClientPacketAdapterRegistry {
    let mut registry = ClientPacketAdapterRegistry::new();

    registry.register(Box::new(HeartbeatAdapter));
    registry.register(Box::new(PreLoginAdapter));
    registry.register(Box::new(RegisterResponseAdapter));
    registry.register(Box::new(LoginResponseAdapter));
    registry.register(Box::new(ClientInfoAdapter));
    registry.register(Box::new(BroadcastMessageAdapter));
    registry.register(Box::new(HistoryMessageAdapter));
    registry.register(Box::new(ColorResponseAdapter));
    registry.register(Box::new(RatchetStepAdapter));

    registry
}
