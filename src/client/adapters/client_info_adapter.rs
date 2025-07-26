use anyhow::Result;
use orwell::{
    decode_packet,
    pb::orwell::{OrwellPacket, PacketType, ServerClientInfo},
};
use prost::Message;

use crate::{
    packet_adapter::{ClientPacketAdapter, ClientPacketContext},
    service::{ClientInfo, ClientManager},
    ClientStatus,
};

pub struct ClientInfoAdapter;

impl ClientPacketAdapter for ClientInfoAdapter {
    fn packet_type(&self) -> PacketType {
        PacketType::ServerClientInfo
    }

    fn process(&self, packet: OrwellPacket, _context: ClientPacketContext<'_>) -> Result<()> {
        let packet = decode_packet!(packet, ServerClientInfo);
        let new_clients = packet.data;
        for client in new_clients {
            ClientManager::add_client(ClientInfo {
                id: client.id.clone(),
                name: client.name.clone(),
                color: client.color as i32,
                kyber_pk: client.kyber_pk,
                status: ClientStatus::try_from(client.status as i32).unwrap(),
            });
        }

        Ok(())
    }
}
