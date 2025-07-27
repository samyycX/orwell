use anyhow::Result;
use orwell::shared::helper::color_code_to_hex;
use orwell::{
    decode_packet,
    pb::orwell::{OrwellPacket, PacketType, ServerChangeColorResponse},
};
use prost::Message;

use crate::{
    message::LineBuilder,
    message::{add_chat_message, add_chat_message_rich},
    packet_adapter::{ClientPacketAdapter, ClientPacketContext},
};
use ratatui::style::Color;

pub struct ColorResponseAdapter;

impl ClientPacketAdapter for ColorResponseAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerChangeColorResponse
    }

    fn process(&self, packet: OrwellPacket, _context: ClientPacketContext<'_>) -> Result<()> {
        let packet = decode_packet!(packet, ServerChangeColorResponse);

        match packet.success {
            true => {
                add_chat_message_rich(
                    LineBuilder::new()
                        .plain("更改颜色至 ")
                        .colored(
                            color_code_to_hex(packet.color.unwrap()).to_string(),
                            Color::from_u32(packet.color.unwrap() as u32),
                        )
                        .plain(" 成功")
                        .build(),
                    None,
                );
            }
            false => {
                add_chat_message(format!("更改颜色失败: {}", packet.message.unwrap()));
            }
        }

        Ok(())
    }
}
