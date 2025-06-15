use orwell::{
    pb::orwell::{ClientHello, ClientRegister, PacketType},
    shared::packet::Packet,
};
use prost::Message;

struct ClientPacket {}

impl ClientPacket {
    pub fn err(err: &str) -> Packet {
        Packet {
            type_id: PacketType::ClientError as u8,
            data: err.as_bytes().to_vec(),
        }
    }

    pub fn hello() -> Packet {
        Packet {
            type_id: PacketType::ClientHello as u8,
            data: ClientHello {
                hello: "0RW3LL".to_string(),
            }
            .encode_to_vec(),
        }
    }
    pub fn register(username: &str, pk: &[u8]) -> Packet {
        Packet {
            type_id: PacketType::ClientRegister as u8,
            data: ClientRegister {
                username: username.to_string(),
                pk: pk.to_vec(),
            }
            .encode_to_vec(),
        }
    }
}
