use anyhow::Result;

use crate::{
    command_adapter::{CommandAdapter, CommandContext},
    message::add_chat_message,
    service::Service,
    STATE,
};

pub struct ColorCommand;

impl CommandAdapter for ColorCommand {
    fn command_name(&self) -> &'static str {
        "/color"
    }

    fn description(&self) -> &'static str {
        "更改聊天颜色"
    }

    fn usage(&self) -> &'static str {
        "/color <颜色代码>"
    }

    fn process(&self, args: &[&str], _context: CommandContext<'_>) -> Result<()> {
        if args.len() != 1 {
            add_chat_message("使用方法: /color <颜色代码>");
            return Ok(());
        }

        let state = STATE.read().unwrap();
        if !state.connected {
            add_chat_message("您尚未连接至服务器，无法改变颜色");
            return Ok(());
        }
        drop(state);

        let color = args[0];
        if !color.starts_with('#') || color.len() != 7 {
            add_chat_message("颜色代码必须是 #RRGGBB 格式");
            return Ok(());
        }

        match i32::from_str_radix(&color[1..], 16) {
            Ok(color) => {
                Service::change_color(color);
            }
            Err(_) => {
                add_chat_message("颜色代码格式错误，请使用 #RRGGBB 格式");
            }
        }

        Ok(())
    }
}
