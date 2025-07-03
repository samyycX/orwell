use anyhow::Result;
use crystals_dilithium::dilithium5;
use diesel::{Connection, SqliteConnection};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use lazy_static::lazy_static;
use orwell::{
    decode_packet,
    pb::orwell::{
        ClientAfk, ClientChangeColor, ClientHello, ClientHello2, ClientInfo, ClientLogin,
        ClientMessage, ClientPreLogin, ClientRegister, ClientStatus, Key, MessageType,
        OrwellRatchetPacket, OrwellRatchetStep, OrwellSignedPacket, PacketType,
        ServerBroadcastChangeColor, ServerBroadcastMessage, ServerChangeColorResponse,
        ServerClientInfo, ServerHeartbeat, ServerHello, ServerLoginResponse, ServerPreLogin,
        ServerRegisterResponse,
    },
    shared::{
        encryption::{Encryption, KyberDoubleRatchet, RatchetState},
        helper::{get_now_timestamp, get_version},
    },
};
use pqcrypto_kyber::{kyber1024, kyber1024_encapsulate};
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SharedSecret};
use prost::Message as ProstMessage;
use rand::Rng;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
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
    client::ClientManager,
    config::{get_cert_fullchain_path, get_cert_key_path, get_port},
    message::MessageManager,
    service::Service,
    token::TokenManager,
};

mod client;
mod config;
mod message;
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

async fn handle_packet(
    packet: OrwellSignedPacket,
    ws_sender: Arc<Mutex<WsSender>>,
    conn_id: u32,
) -> Result<()> {
    let client = ClientManager::get_client_by_connection(conn_id).await;
    match client {
        None => {
            let packet = Encryption::validate(packet, None)?;
            let type_ = PacketType::try_from(packet.packet_type)?;
            match type_ {
                PacketType::ClientPreLogin => {
                    let packet = decode_packet!(packet, ClientPreLogin);
                    let client = ClientManager::find_client(&packet.dilithium_pk);

                    if get_version() != packet.version {
                        let response = ServerPreLogin {
                            registered: false,
                            can_register: false,
                            token: None,
                            version_mismatch: true,
                        };
                        send_packet(conn_id, PacketType::ServerPreLogin, response).await?;
                        ws_sender.lock().await.close().await?;
                    }

                    let response = if client.is_none() {
                        ServerPreLogin {
                            registered: false,
                            can_register: true,
                            token: None,
                            version_mismatch: false,
                        }
                    } else {
                        let token =
                            TokenManager::generate_token(conn_id, &packet.dilithium_pk).await?;
                        let token = Encryption::kyber_encrypt(&token, &client.unwrap().kyber_pk_)?;
                        ServerPreLogin {
                            registered: true,
                            can_register: false,
                            token: Some(token),
                            version_mismatch: false,
                        }
                    };
                    send_packet(conn_id, PacketType::ServerPreLogin, response).await?;
                }
                PacketType::ClientRegister => {
                    let packet = decode_packet!(packet, ClientRegister);
                    let client = ClientManager::find_client(&packet.dilithium_pk);
                    let mut registered_client = None;
                    let response = if client.is_some() {
                        ServerRegisterResponse {
                            success: false,
                            color: None,
                            message: Some("您已经注册过了".to_string()),
                        }
                    } else if ClientManager::is_name_taken(&packet.name) {
                        ServerRegisterResponse {
                            success: false,
                            color: None,
                            message: Some("该用户名已被占用".to_string()),
                        }
                    } else {
                        let color = rand::thread_rng().gen_range(0..0x00FFFFFF);
                        let client = ClientManager::register_client(
                            &packet.name,
                            &packet.kyber_pk,
                            &packet.dilithium_pk,
                            color,
                        );
                        registered_client.replace(client);
                        ServerRegisterResponse {
                            success: true,
                            color: Some(color),
                            message: Some("注册成功".to_string()),
                        }
                    };
                    send_packet(conn_id, PacketType::ServerRegisterResponse, response).await?;

                    if let Some(client) = registered_client {
                        Service::login_client(conn_id, client.clone()).await?;
                    }
                }
                PacketType::ClientLogin => {
                    let packet = decode_packet!(packet, ClientLogin);
                    let token = TokenManager::validate_token(conn_id, &packet.token_sign).await;
                    let mut login_client = None;
                    let response = if token.is_none() {
                        ServerLoginResponse {
                            success: false,
                            message: Some("身份校验失败".to_string()),
                        }
                    } else {
                        let pk = token.clone().unwrap().1;
                        let client = ClientManager::find_client(&pk).unwrap();
                        login_client.replace(client);
                        ServerLoginResponse {
                            success: true,
                            message: Some("登录成功".to_string()),
                        }
                    };

                    send_packet(conn_id, PacketType::ServerLoginResponse, response).await?;

                    if let Some(client) = login_client {
                        Service::login_client(conn_id, client).await?;
                    }
                }

                _ => {}
            }
        }
        Some(client_info) => {
            let client = client_info.client;
            let packet = Encryption::validate(
                packet,
                Some(&dilithium5::PublicKey::from_bytes(&client.dilithium_pk_)),
            )?;
            let type_ = PacketType::try_from(packet.packet_type);
            if type_.is_err() {
                return Err(anyhow::anyhow!("Illegal data sent by {}", client.name_));
            }
            let type_ = type_.unwrap();
            match type_ {
                PacketType::ClientMessage => {
                    info!("收到消息");
                    let packet = decode_packet!(packet, ClientMessage);
                    let sender = client;
                    let data = packet.data;
                    for key in &packet.keys {
                        let client = ClientManager::get_client_by_id(&key.receiver_id).await;
                        if client.is_none() {
                            continue;
                        }
                        let client = client.unwrap();
                        if let Some(conn_id) =
                            ClientManager::get_client_connection_by_id(&client.id_.clone()).await
                        {
                            info!("Receiver: {:?}", conn_id);
                            send_packet(
                                conn_id,
                                PacketType::ServerBroadcastMessage,
                                ServerBroadcastMessage {
                                    sender_id: sender.id_.clone(),
                                    sender_name: sender.name_.clone(),
                                    key: Some(key.clone()),
                                    data: data.clone(),
                                    color: sender.color_,
                                    timestamp: get_now_timestamp(),
                                },
                            )
                            .await?;
                        }
                    }
                    MessageManager::add_message(sender.id_.clone(), data.clone(), packet.keys)
                        .await;
                }
                PacketType::ClientChangeColor => {
                    let packet = decode_packet!(packet, ClientChangeColor);
                    let clients = ClientManager::get_all_clients().await;
                    if clients
                        .iter()
                        .map(|c| c.client.color_ == packet.color && c.client.id_ != client.id_)
                        .any(|x| x)
                    {
                        send_packet(
                            conn_id,
                            PacketType::ServerChangeColorResponse,
                            ServerChangeColorResponse {
                                success: false,
                                color: None,
                                message: Some(format!("已经有用户在使用此颜色")),
                            },
                        )
                        .await?;
                    } else {
                        broadcast_message_from_server(
                            MessageType::ChangeColor,
                            &ServerBroadcastChangeColor {
                                id: client.id_.clone(),
                                name: client.name_.clone(),
                                old_color: client.color_,
                                new_color: packet.color,
                            }
                            .encode_to_vec(),
                            None,
                            None,
                            None,
                            true,
                        )
                        .await?;

                        ClientManager::update_color(&client.id_, packet.color).await;

                        send_packet(
                            conn_id,
                            PacketType::ServerChangeColorResponse,
                            ServerChangeColorResponse {
                                success: true,
                                color: Some(packet.color),
                                message: None,
                            },
                        )
                        .await?;
                    }
                }

                PacketType::ClientAfk => {
                    let packet = decode_packet!(packet, ClientAfk);
                    let status = ClientManager::get_status(conn_id).await;
                    if status == ClientStatus::Afk {
                        ClientManager::update_status(conn_id, ClientStatus::Online).await;
                        broadcast_message_from_server(
                            MessageType::LeftAfk,
                            &[],
                            Some(client.id_.clone()),
                            Some(client.name_.clone()),
                            Some(client.color_),
                            false,
                        )
                        .await?;
                    } else {
                        ClientManager::update_status(conn_id, ClientStatus::Afk).await;
                        broadcast_message_from_server(
                            MessageType::EnterAfk,
                            &[],
                            Some(client.id_.clone()),
                            Some(client.name_.clone()),
                            Some(client.color_),
                            false,
                        )
                        .await?;
                    }
                    Service::broadcast_resync_client().await?;
                }
                _ => {}
            }
        }
    };

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
    SENDERS.write().await.insert(conn_id, ws_sender.clone());
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
                        let mut connections = CONNECTIONS.write().await;
                        connections.insert(conn_id, ratchet);
                        drop(connections);

                        let response = packet.encode_to_vec();
                        let mut sender = ws_sender.lock().await;
                        sender.send(Message::Binary(response.into())).await.unwrap();
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
                    }
                    RatchetState::HandshakeFinished => {
                        let mut connections = CONNECTIONS.write().await;
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

    while let Ok((stream, addr)) = listener.accept().await {
        let stream_tcp_tls = acceptor.accept(stream).await?;
        match tokio_tungstenite::accept_async(stream_tcp_tls).await {
            Ok(ws_stream) => {
                tokio::spawn(handle_connection_with_error(ws_stream, addr));
            }
            Err(e) => {
                warn!("Failed to accept connection from {}: {:?}", addr, e);
            }
        };
    }

    tracing_subscriber::fmt::init();

    // heartbeat
    tokio::spawn(async move {
        loop {
            let senders_clone = {
                let senders = SENDERS.read().await;
                ClientManager::get_all_connections()
                    .await
                    .into_iter()
                    .filter_map(|conn_id| {
                        senders
                            .get(&conn_id)
                            .map(|sender| (conn_id, sender.clone()))
                    })
                    .collect::<Vec<_>>()
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

    Ok(())
}
