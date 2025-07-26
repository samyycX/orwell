pub mod afk_command;
pub mod color_command;
pub mod connect_command;
pub mod login_command;
pub mod register_command;

use crate::command_adapter::CommandAdapterRegistry;

use self::{
    afk_command::AfkCommand, color_command::ColorCommand, connect_command::ConnectCommand,
    login_command::LoginCommand, register_command::RegisterCommand,
};

/// Create and register all command adapters
pub fn create_command_registry() -> CommandAdapterRegistry {
    let mut registry = CommandAdapterRegistry::new();

    registry.register(Box::new(RegisterCommand));
    registry.register(Box::new(LoginCommand));
    registry.register(Box::new(ConnectCommand));
    registry.register(Box::new(ColorCommand));
    registry.register(Box::new(AfkCommand));

    registry
}
