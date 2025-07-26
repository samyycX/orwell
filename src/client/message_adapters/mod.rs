pub mod color_change_message_adapter;
pub mod enter_afk_message_adapter;
pub mod left_afk_message_adapter;
pub mod login_message_adapter;
pub mod logout_message_adapter;
pub mod text_message_adapter;

use crate::message_adapter::MessageAdapterRegistry;

use self::{
    color_change_message_adapter::ColorChangeMessageAdapter,
    enter_afk_message_adapter::EnterAfkMessageAdapter,
    left_afk_message_adapter::LeftAfkMessageAdapter, login_message_adapter::LoginMessageAdapter,
    logout_message_adapter::LogoutMessageAdapter, text_message_adapter::TextMessageAdapter,
};

/// Create and register all message adapters
pub fn create_message_registry() -> MessageAdapterRegistry {
    let mut registry = MessageAdapterRegistry::new();

    registry.register(Box::new(TextMessageAdapter));
    registry.register(Box::new(LoginMessageAdapter));
    registry.register(Box::new(LogoutMessageAdapter));
    registry.register(Box::new(ColorChangeMessageAdapter));
    registry.register(Box::new(EnterAfkMessageAdapter));
    registry.register(Box::new(LeftAfkMessageAdapter));

    registry
}
