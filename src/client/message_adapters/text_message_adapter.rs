use anyhow::Result;
use orwell::pb::orwell::{MessageType, ServerBroadcastMessage};

use crate::{
    message::{add_chat_message_rich, LineBuilder, TextSpan},
    message_adapter::{MessageAdapter, MessageContext},
    notify::Notifier,
};

pub struct TextMessageAdapter;

impl MessageAdapter for TextMessageAdapter {
    fn message_type(&self) -> MessageType {
        MessageType::Text
    }

    fn process(
        &self,
        message: &ServerBroadcastMessage,
        data: Vec<u8>,
        context: MessageContext,
    ) -> Result<()> {
        let text = String::from_utf8(data)?;

        if !context.is_history {
            Notifier::notify_message(&message.sender_name, &text);
        }

        add_chat_message_rich(
            LineBuilder::new()
                .time(message.timestamp)
                .sender(TextSpan::new(
                    message.sender_name.clone(),
                    Style::default()
                        .fg(Color::from_u32(message.color as u32))
                        .add_modifier(Modifier::BOLD),
                ))
                .plain(text)
                .build(),
            if context.is_history { Some(0) } else { None },
        );

        Ok(())
    }
}

use ratatui::style::{Color, Modifier, Style};
