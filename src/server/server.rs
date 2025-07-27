use anyhow::Result;
use crystals_dilithium::dilithium5;
use diesel::{Connection, SqliteConnection};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use lazy_static::lazy_static;
use orwell::{
    pb::orwell::{
        ClientHello, ClientHello2, Key, MessageType, OrwellRatchetPacket, OrwellRatchetStep,
        OrwellSignedPacket, PacketType, ServerBroadcastMessage, ServerHeartbeat, ServerHello,
    },
    shared::{
        encryption::{Encryption, KyberDoubleRatchet, RatchetState},
        helper::get_now_timestamp,
    },
};
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SharedSecret};
use prost::Message as ProstMessage;
use rand::Rng;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::{
    collections::HashMap,
    fs,
    io::BufReader,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
    sync::RwLock,
};
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{info, warn};

use crate::{
    adapters::create_registry,
    client::ClientManager,
    config::{get_cert_fullchain_path, get_cert_key_path, get_port},
    message::MessageManager,
    packet_adapter::PacketContext,
    service::Service,
};

use packet_adapter::PacketAdapterRegistry;

mod adapters;
mod client;
mod config;
mod message;
mod packet_adapter;
mod service;
mod token;

pub type WsSender = SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>;

pub struct State {
    dilithium_sk: dilithium5::SecretKey,
    dilithium_pk: dilithium5::PublicKey,
}

impl State {
    pub fn new() -> Self {
        let keys = dilithium5::Keypair::generate(None);
        Self {
            dilithium_sk: keys.secret,
            dilithium_pk: keys.public,
        }
    }
}

lazy_static! {
    static ref STATE: Arc<RwLock<State>> = Arc::new(RwLock::new(State::new()));
    static ref CONNECTIONS: Arc<RwLock<HashMap<u32, KyberDoubleRatchet>>> =
        Arc::new(RwLock::new(HashMap::new()));
    static ref SENDERS: Arc<RwLock<HashMap<u32, Arc<Mutex<WsSender>>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

async fn broadcast_message_from_server(
    message_type: MessageType,
    msg_data: &[u8],
    sender_id: Option<String>,
    sender_name: Option<String>,
    color: Option<i32>,
    except_sender: bool,
) -> Result<()> {
    let sender_id = sender_id.unwrap_or_default();
    let sender_name = sender_name.unwrap_or_default();
    let color = color.unwrap_or_default();

    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    let mut msg_data = msg_data.to_vec();
    msg_data.insert(0, message_type as u8);
    let encrypted_data = Encryption::aes_encrypt(&msg_data, &key);

    let packet = ServerBroadcastMessage {
        key: None,
        sender_id: sender_id.clone(),
        sender_name: sender_name.clone(),
        color: color.clone(),
        data: encrypted_data.clone(),
        timestamp: get_now_timestamp(),
    };

    let mut keys = vec![];

    for client_info in ClientManager::get_all_clients().await {
        let client = client_info.client;
        let mut p = packet.clone();
        let k = Key {
            receiver_id: client.id_.clone(),
            ciphertext: Encryption::kyber_encrypt(&key, &client.kyber_pk_)?,
        };
        p.key = Some(k.clone());
        keys.push(k);

        if except_sender && client.id_ == sender_id {
            continue;
        }
        if let Some(conn_id) = ClientManager::get_client_connection_by_id(&client.id_).await {
            send_packet(conn_id, PacketType::ServerBroadcastMessage, p.clone()).await?;
        }
    }

    MessageManager::add_message(sender_id.clone(), encrypted_data.clone(), keys).await;

    Ok(())
}

async fn send_packet_internal<T>(
    conn_id: u32,
    packet_type: PacketType,
    packet: T,
    ratchet: &mut KyberDoubleRatchet,
) -> Result<()>
where
    T: prost::Message,
{
    let state = STATE.read().await;

    let start_time = Instant::now();
    let encrypted =
        Encryption::encrypt_packet(packet_type, packet, &state.dilithium_sk.to_bytes(), ratchet)?;
    let end_time: Instant = Instant::now();
    info!("加密用时 {}ms", (end_time - start_time).as_millis());

    let target_sender = {
        let senders = SENDERS.read().await;
        senders.get(&conn_id).cloned()
    };

    let sender = target_sender.unwrap();
    let mut sender = sender.lock().await;
    sender.send(Message::Binary(encrypted.into())).await?;

    Ok(())
}

async fn send_packet<T>(conn_id: u32, packet_type: PacketType, packet: T) -> Result<()>
where
    T: prost::Message,
{
    let mut connections = CONNECTIONS.write().await;
    let ratchet = connections
        .get_mut(&conn_id)
        .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;
    let ratchet_state = ratchet.ratchet_state.clone();
    send_packet_internal(conn_id, packet_type, packet, ratchet).await?;
    drop(connections);

    let rand = rand::thread_rng().gen_ratio(3, 10);
    if rand && ratchet_state == RatchetState::HandshakeFinished {
        info!("ratchet step");
        let mut connections = CONNECTIONS.write().await;
        let ratchet = connections.get_mut(&conn_id).unwrap();
        let mut old_ratchet = ratchet.clone();
        let ct = ratchet.step_send_chain()?;
        let packet = OrwellRatchetStep {
            ct: ct.as_bytes().to_vec(),
        };
        drop(connections);
        send_packet_internal(
            conn_id,
            PacketType::ServerOrwellRatchetStep,
            packet,
            &mut old_ratchet,
        )
        .await?;
    }

    Ok(())
}

lazy_static! {
    static ref ADAPTER_REGISTRY: tokio::sync::OnceCell<PacketAdapterRegistry> =
        tokio::sync::OnceCell::const_new();
}

async fn get_adapter_registry() -> &'static PacketAdapterRegistry {
    ADAPTER_REGISTRY
        .get_or_init(|| async { create_registry() })
        .await
}

async fn handle_packet(
    packet: OrwellSignedPacket,
    ws_sender: Arc<Mutex<WsSender>>,
    conn_id: u32,
) -> Result<()> {
    let client = ClientManager::get_client_by_connection(conn_id).await;

    let validated_packet = match &client {
        None => Encryption::validate(packet.clone(), None)?,
        Some(client_info) => Encryption::validate(
            packet.clone(),
            Some(&dilithium5::PublicKey::from_bytes(
                &client_info.client.dilithium_pk_,
            )),
        )?,
    };

    let packet_type = PacketType::try_from(validated_packet.packet_type)?;

    let registry = get_adapter_registry().await;
    if let Some(adapter) = registry.get(packet_type) {
        let context = PacketContext {
            conn_id,
            ws_sender,
            client_info: client,
        };

        adapter.process(validated_packet, context).await?;
    } else {
        tracing::warn!("No adapter found for packet type: {:?}", packet_type);
    }

    Ok(())
}

async fn handle_connection_with_error(
    stream: WebSocketStream<TlsStream<TcpStream>>,
    addr: std::net::SocketAddr,
) -> Result<()> {
    handle_connection(stream, addr)
        .await
        .inspect_err(|e| warn!("Error when handling connection: {:?}", e))
}

async fn handle_connection(
    stream: WebSocketStream<TlsStream<TcpStream>>,
    addr: std::net::SocketAddr,
) -> Result<()> {
    let (ws_sender_raw, mut ws_receiver) = stream.split();
    let ws_sender = Arc::new(Mutex::new(ws_sender_raw));
    let conn_id = addr.port() as u32;
    info!("New connection: {}", conn_id);
    // Store sender for global access
    let mut senders = SENDERS.write().await;
    senders.insert(conn_id, ws_sender.clone());
    drop(senders);
    info!("Sender stored: {}", conn_id);

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                let connections = CONNECTIONS.read().await;
                let ratchet: Option<&KyberDoubleRatchet> = connections.get(&conn_id);
                let state = match ratchet {
                    Some(ratchet) => ratchet.ratchet_state.clone(),
                    None => RatchetState::HandshakePhase1,
                };
                drop(connections);
                match state {
                    RatchetState::HandshakePhase1 => {
                        info!("客户端已连接");
                        let state = STATE.read().await;
                        let packet = ClientHello::decode(data).unwrap();
                        let mut ratchet = KyberDoubleRatchet::new();
                        ratchet.ratchet_state = RatchetState::HandshakePhase2;
                        let response = ratchet.initialize_session(&packet.pk)?;
                        let packet = ServerHello {
                            ciphertext: response,
                            pk: ratchet.kyber_pk.as_bytes().to_vec(),
                            dilithium_pk: state.dilithium_pk.to_bytes().to_vec(),
                        };
                        drop(state);
                        let mut connections = CONNECTIONS.write().await;
                        connections.insert(conn_id, ratchet);
                        drop(connections);

                        let response = packet.encode_to_vec();
                        let mut sender = ws_sender.lock().await;
                        sender.send(Message::Binary(response.into())).await.unwrap();
                        drop(sender);
                        info!("已回应客户端");
                    }
                    RatchetState::HandshakePhase2 => {
                        let packet = ClientHello2::decode(data).unwrap();
                        let mut connections = CONNECTIONS.write().await;
                        let ratchet: Option<&mut KyberDoubleRatchet> =
                            connections.get_mut(&conn_id);
                        if ratchet.is_none() {
                            return Err(anyhow::anyhow!("Connection not found"));
                        }
                        let ratchet = ratchet.unwrap();
                        ratchet.finalize_session(&packet.ciphertext)?;
                        ratchet.ratchet_state = RatchetState::HandshakeFinished;
                        drop(connections);

                        let mut random_data = vec![];
                        let mut rng = rand::rngs::OsRng::default();
                        for _ in 0..rng.gen_range(1024..4096) {
                            random_data.push(rng.gen_range(0..=255));
                        }
                        let mut sender = ws_sender.lock().await;
                        sender
                            .send(Message::Binary(random_data.to_vec().into()))
                            .await
                            .unwrap();
                        drop(sender);
                    }
                    RatchetState::HandshakeFinished => {
                        let mut connections: tokio::sync::RwLockWriteGuard<
                            '_,
                            HashMap<u32, KyberDoubleRatchet>,
                        > = CONNECTIONS.write().await;
                        let ratchet: Option<&mut KyberDoubleRatchet> =
                            connections.get_mut(&conn_id);
                        if ratchet.is_none() {
                            return Err(anyhow::anyhow!("Connection not found"));
                        }
                        let ratchet = ratchet.unwrap();
                        let data = OrwellRatchetPacket::decode(data)?;
                        let data = ratchet.decrypt(data)?;
                        drop(connections);

                        handle_packet(data, ws_sender.clone(), conn_id).await?;
                    }
                }
            }
            Ok(Message::Text(_)) => {
                let mut sender = ws_sender.lock().await;
                sender
                    .send(Message::Text("Invalid packet".to_string().into()))
                    .await
                    .unwrap();
                drop(sender);
            }
            Ok(Message::Close(_)) => {
                CONNECTIONS.write().await.remove(&conn_id);
                SENDERS.write().await.remove(&conn_id);
                Service::logout_client(conn_id).await?;
                break;
            }
            Err(e) => {
                warn!("WebSocket 错误: {:?}", e);
                CONNECTIONS.write().await.remove(&conn_id);
                SENDERS.write().await.remove(&conn_id);
                Service::logout_client(conn_id).await?;
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn get_db_connection() -> SqliteConnection {
    SqliteConnection::establish("server.db").unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = format!("0.0.0.0:{}", get_port());
    let listener = TcpListener::bind(addr.clone()).await?;
    println!("Listening on: {}", addr);

    let fullchain_path = get_cert_fullchain_path();
    let key_path = get_cert_key_path();
    if fullchain_path.is_none() || key_path.is_none() {
        return Err(anyhow::anyhow!("TLS certificate not found"));
    }
    let fullchain_path = fullchain_path.unwrap();
    let key_path = key_path.unwrap();
    let fullchain: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(fs::File::open(fullchain_path)?))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
    let key: PrivateKeyDer<'static> =
        rustls_pemfile::ec_private_keys(&mut BufReader::new(fs::File::open(key_path)?))
            .next()
            .unwrap()
            .map(Into::into)?;

    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(fullchain, key)
        .unwrap();

    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

    tracing_subscriber::fmt::init();

    // heartbeat
    tokio::spawn(async move {
        loop {
            let senders_clone = {
                let senders = SENDERS.read().await;
                let result = ClientManager::get_all_connections()
                    .await
                    .into_iter()
                    .filter_map(|conn_id| {
                        senders
                            .get(&conn_id)
                            .map(|sender| (conn_id, sender.clone()))
                    })
                    .collect::<Vec<_>>();
                drop(senders);
                result
            };

            for (conn_id, sender) in senders_clone {
                if let Err(e) =
                    send_packet(conn_id, PacketType::ServerHeartbeat, ServerHeartbeat {}).await
                {
                    warn!("Failed to send heartbeat to {}: {:?}", conn_id, e);
                }
            }

            let random_sleep_time = rand::thread_rng().gen_range(15000..40000);
            tokio::time::sleep(Duration::from_millis(random_sleep_time)).await;
        }
    });

    while let Ok((stream, addr)) = listener.accept().await {
        let stream_tcp_tls = acceptor.accept(stream).await;
        if stream_tcp_tls.is_err() {
            warn!(
                "Failed to accept connection from {}: {:?}",
                addr,
                stream_tcp_tls.err()
            );
            continue;
        }
        let stream_tcp_tls = stream_tcp_tls.unwrap();
        match tokio_tungstenite::accept_async(stream_tcp_tls).await {
            Ok(ws_stream) => {
                tokio::spawn(handle_connection_with_error(ws_stream, addr));
            }
            Err(e) => {
                warn!("Failed to accept connection from {}: {:?}", addr, e);
            }
        };
    }

    Ok(())
}
