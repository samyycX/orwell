use crystals_dilithium::dilithium5::SecretKey;
use lazy_static::lazy_static;
use orwell::pb::orwell::{OrwellPacket, PacketType};
use orwell::shared::encryption::Encryption;
use prost::Message;

lazy_static! {
    static ref HANDLER: Handler = Handler::new();
}

pub struct Handler {}

impl Handler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_packet(&self, data: &[u8]) {
        if !Encryption::validate(data) {
            println!("Invalid packet");
            return;
        }

        let packet = OrwellPacket::decode(data).unwrap();

        let type_ = PacketType::from_i32(packet.r#type as i32);
        if type_.is_none() {
            println!("Invalid packet type");
            return;
        }

        let type_ = type_.unwrap();

        match type_ {
            PacketType::ClientHello => {
                println!("Client hello");
            }
            _ => {
                println!("Unknown packet");
            }
        }
    }
}
