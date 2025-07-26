use anyhow::Result;
use orwell::pb::orwell::{ClientStatus, MessageType, ServerBroadcastMessage};

use crate::{
    message::{add_chat_message_rich, LineBuilder, TextSpan},
    message_adapter::{MessageAdapter, MessageContext},
    notify::Notifier,
    service::ClientManager,
};

pub struct LogoutMessageAdapter;

impl MessageAdapter for LogoutMessageAdapter {
    fn message_type(&self) -> MessageType {
        MessageType::Logout
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
                &format!("{} 下线了", message.sender_name),
            );
        }

        add_chat_message_rich(
            LineBuilder::new()
                .time(message.timestamp)
                .sender(TextSpan::new(
                    "←",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ))
                .colored(
                    message.sender_name.clone(),
                    Color::from_u32(message.color as u32),
                )
                .plain(" 下线了")
                .build(),
            if context.is_history { Some(0) } else { None },
        );

        ClientManager::update_status(message.sender_id.clone(), ClientStatus::Offline);

        Ok(())
    }
}

use ratatui::style::{Color, Modifier, Style};
