use anyhow::Result;

use crate::{
    command_adapter::{CommandAdapter, CommandContext},
    message::add_chat_message,
    network::Network,
};

pub struct ConnectCommand;

impl CommandAdapter for ConnectCommand {
    fn command_name(&self) -> &'static str {
        "/connect"
    }

    fn description(&self) -> &'static str {
        "连接到服务器"
    }

    fn usage(&self) -> &'static str {
        "/connect <服务器地址>"
    }

    fn process(&self, args: &[&str], _context: CommandContext<'_>) -> Result<()> {
        if args.len() != 1 {
            add_chat_message("使用方法: /connect <服务器地址>");
            return Ok(());
        }

        let server_address = args[0];
        Network::start(server_address.to_string());

        Ok(())
    }
}
