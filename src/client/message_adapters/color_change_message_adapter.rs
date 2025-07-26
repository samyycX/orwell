use anyhow::Result;
use orwell::pb::orwell::{MessageType, ServerBroadcastChangeColor, ServerBroadcastMessage};

use crate::{
    message::{add_chat_message_rich, LineBuilder},
    message_adapter::{MessageAdapter, MessageContext},
    service::ClientManager,
};
use orwell::shared::helper::color_code_to_hex;
pub struct ColorChangeMessageAdapter;

impl MessageAdapter for ColorChangeMessageAdapter {
    fn message_type(&self) -> MessageType {
        MessageType::ChangeColor
    }

    fn process(
        &self,
        message: &ServerBroadcastMessage,
        data: Vec<u8>,
        context: MessageContext,
    ) -> Result<()> {
        let color_data = ServerBroadcastChangeColor::decode(data.as_slice())?;

        add_chat_message_rich(
            LineBuilder::new()
                .time(message.timestamp)
                .colored(
                    color_data.name.clone(),
                    Color::from_u32(color_data.new_color as u32),
                )
                .plain(" 的聊天颜色从 ")
                .colored(
                    color_code_to_hex(color_data.old_color),
                    Color::from_u32(color_data.old_color as u32),
                )
                .plain(" 更改至 ")
                .colored(
                    color_code_to_hex(color_data.new_color),
                    Color::from_u32(color_data.new_color as u32),
                )
                .build(),
            if context.is_history { Some(0) } else { None },
        );

        ClientManager::update_color(color_data.id.clone(), color_data.new_color);

        Ok(())
    }
}

use prost::Message as ProstMessage;
use ratatui::style::Color;
