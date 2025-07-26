use anyhow::Result;
use orwell::pb::orwell::{MessageType, ServerBroadcastMessage};

use crate::{
    message::{add_chat_message_rich, LineBuilder, TextSpan},
    message_adapter::{MessageAdapter, MessageContext},
    notify::Notifier,
};

pub struct EnterAfkMessageAdapter;

impl MessageAdapter for EnterAfkMessageAdapter {
    fn message_type(&self) -> MessageType {
        MessageType::EnterAfk
    }

    fn process(
        &self,
        message: &ServerBroadcastMessage,
        _data: Vec<u8>,
        context: MessageContext,
    ) -> Result<()> {
        if !context.is_history {
            Notifier::notify_message(
                &message.sender_name,
                &format!("{} 进入了AFK状态", message.sender_name),
            );
        }

        add_chat_message_rich(
            LineBuilder::new()
                .time(message.timestamp)
                .sender(TextSpan::new(
                    "\u{f04b2}",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
                .colored(
                    message.sender_name.clone(),
                    Color::from_u32(message.color as u32),
                )
                .plain(" 进入了AFK状态")
                .build(),
            if context.is_history { Some(0) } else { None },
        );

        Ok(())
    }
}

use ratatui::style::{Color, Modifier, Style};
