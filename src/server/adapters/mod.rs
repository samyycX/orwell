pub mod afk_adapter;
pub mod color_adapter;
pub mod login_adapter;
pub mod message_adapter;
pub mod pre_login_adapter;
pub mod register_adapter;

use crate::adapters::{
    afk_adapter::AfkAdapter, color_adapter::ColorAdapter, login_adapter::LoginAdapter,
    message_adapter::MessageAdapter, pre_login_adapter::PreLoginAdapter,
    register_adapter::RegisterAdapter,
};
use crate::packet_adapter::PacketAdapterRegistry;

/// Create and register all packet adapters
pub fn create_registry() -> PacketAdapterRegistry {
    let mut registry = PacketAdapterRegistry::new();

    registry.register(Box::new(PreLoginAdapter));
    registry.register(Box::new(RegisterAdapter));
    registry.register(Box::new(LoginAdapter));
    registry.register(Box::new(MessageAdapter));
    registry.register(Box::new(ColorAdapter));
    registry.register(Box::new(AfkAdapter));

    registry
}
