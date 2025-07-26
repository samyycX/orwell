use anyhow::Result;

use crate::{
    command_adapter::{CommandAdapter, CommandContext},
    key::KeyManager,
    message::add_chat_message,
};

pub struct LoginCommand;

impl CommandAdapter for LoginCommand {
    fn command_name(&self) -> &'static str {
        "/login"
    }

    fn description(&self) -> &'static str {
        "登录用户"
    }

    fn usage(&self) -> &'static str {
        "/login <用户名> <密码>"
    }

    fn process(&self, args: &[&str], _context: CommandContext<'_>) -> Result<()> {
        if args.len() != 2 {
            add_chat_message("使用方法: /login <用户名> <密码>");
            return Ok(());
        }

        let name = args[0];
        let password = args[1];
        KeyManager::load_key(name, password);

        Ok(())
    }
}
