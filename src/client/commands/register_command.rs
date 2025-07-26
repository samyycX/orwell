use anyhow::Result;
use std::thread;
use std::time::Duration;

use crate::{
    command_adapter::{CommandAdapter, CommandContext},
    key::KeyManager,
    message::add_chat_message,
};

pub struct RegisterCommand;

impl CommandAdapter for RegisterCommand {
    fn command_name(&self) -> &'static str {
        "/register"
    }

    fn description(&self) -> &'static str {
        "注册新用户"
    }

    fn usage(&self) -> &'static str {
        "/register <用户名> <密码> <确认密码>"
    }

    fn process(&self, args: &[&str], _context: CommandContext<'_>) -> Result<()> {
        if args.len() != 3 {
            add_chat_message("使用方法: /register <用户名> <密码> <确认密码>");
            return Ok(());
        }

        let name = args[0];
        let password = args[1];
        let confirm_password = args[2];

        if password != confirm_password {
            add_chat_message("密码不一致！");
            return Ok(());
        }

        add_chat_message("正在创建密钥...此过程需要数十秒，请耐心等候");
        thread::sleep(Duration::from_millis(500));
        KeyManager::create_key(name, password);

        Ok(())
    }
}
