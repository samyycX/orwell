use anyhow::Result;

use crate::{
    command_adapter::{CommandAdapter, CommandContext},
    message::add_chat_message,
    service::Service,
};

pub struct AfkCommand;

impl CommandAdapter for AfkCommand {
    fn command_name(&self) -> &'static str {
        "/afk"
    }

    fn description(&self) -> &'static str {
        "切换AFK状态"
    }

    fn usage(&self) -> &'static str {
        "/afk"
    }

    fn process(&self, _args: &[&str], _context: CommandContext<'_>) -> Result<()> {
        Service::afk();
        Ok(())
    }
}
