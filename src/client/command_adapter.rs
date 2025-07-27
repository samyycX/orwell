use anyhow::Result;

use crate::{
    message::add_chat_message,
    App,
};

/// Context for command processing
pub struct CommandContext<'a> {
    pub app: &'a mut App,
}

/// Trait for command adapters
pub trait CommandAdapter: Send + Sync {
    /// Get the command name this adapter handles
    fn command_name(&self) -> &'static str;

    /// Get command description for help
    fn description(&self) -> &'static str;

    /// Get usage information
    fn usage(&self) -> &'static str;

    /// Process the command
    fn process(&self, args: &[&str], context: CommandContext<'_>) -> Result<()>;
}

/// Registry for command adapters
pub struct CommandAdapterRegistry {
    adapters: std::collections::HashMap<String, Box<dyn CommandAdapter>>,
}

impl CommandAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Box<dyn CommandAdapter>) {
        self.adapters
            .insert(adapter.command_name().to_string(), adapter);
    }

    pub fn get(&self, command: &str) -> Option<&dyn CommandAdapter> {
        self.adapters.get(command).map(|a| a.as_ref())
    }

    pub fn get_all_commands(&self) -> Vec<(&str, &str, &str)> {
        self.adapters
            .values()
            .map(|a| (a.command_name(), a.description(), a.usage()))
            .collect()
    }

    pub fn process_command(&self, command: &str, context: CommandContext<'_>) -> Result<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        if parts[0] == "/help" {
            let commands = self.get_all_commands();
            if commands.is_empty() {
                add_chat_message("没有可用的命令".to_string());
            } else {
                for (name, desc, usage) in commands {
                    add_chat_message(format!("{} - {} (用法: {})", name, desc, usage));
                }
            }
            return Ok(());
        }

        let command_name = parts[0];
        let args = &parts[1..];

        if let Some(adapter) = self.get(command_name) {
            adapter.process(args, context)
        } else {
            add_chat_message(format!("未知命令: {}", command_name));
            add_chat_message("使用 /help 查看可用命令");
            Ok(())
        }
    }
}
