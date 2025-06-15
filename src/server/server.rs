use anyhow::Result;
use crystals_dilithium::dilithium5;
use diesel::{Connection, SqliteConnection};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use lazy_static::lazy_static;
use orwell::{
    decode_packet,
    pb::orwell::{
        ClientChangeColor, ClientHello, ClientLogin, ClientMessage, ClientPreLogin, ClientRegister,
        Message as PbMessage, MessageType, OrwellSignedPacket, PacketType,
        ServerBroadcastChangeColor, ServerBroadcastMessage, ServerChangeColorResponse, ServerHello,
        ServerInformation, ServerLoginResponse, ServerOtherClientPk, ServerPreLogin,
        ServerRegisterResponse,
    },
    shared::{encryption::Encryption, helper::get_now_timestamp},
};
use pqcrypto_kyber::{kyber1024, kyber1024_encapsulate};
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SharedSecret};
use prost::Message as ProstMessage;
use rand::Rng;
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
    sync::RwLock,
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    client::ClientManager, message::MessageManager, service::Service, token::TokenManager,
};

mod client;
mod message;
mod service;
mod token;

pub type WsSender = SplitSink<WebSocketStream<TcpStream>, Message>;

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
    static ref CONNECTIONS: Arc<RwLock<HashMap<u32, kyber1024::SharedSecret>>> =
        Arc::new(RwLock::new(HashMap::new()));
    static ref SENDERS: Arc<RwLock<HashMap<u32, Arc<Mutex<WsSender>>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

async fn broadcast_message_from_server(
    msg_type: MessageType,
    msg_data: &[u8],
    sender_id: Option<String>,
    sender_name: Option<String>,
    color: Option<i32>,
    ws_sender: Arc<Mutex<WsSender>>,
    except_sender: bool,
) -> Result<()> {
    let sender_id = sender_id.unwrap_or_default();
    let sender_name = sender_name.unwrap_or_default();
    let color = color.unwrap_or_default();
    let packet = ServerBroadcastMessage {
        sender_id: sender_id.clone(),
        sender_name: sender_name.clone(),
        color: color.clone(),
        data: None,
    };

    let msg_id = Uuid::now_v7().to_string();

    for client in ClientManager::get_all_clients().await {
        let data = PbMessage {
            id: client.id_.clone(),
            msg_type: msg_type as i32,
            ciphertext: Encryption::kyber_encrypt(msg_data, &client.kyber_pk_)?,
            timestamp: get_now_timestamp(),
        };

        let mut p = packet.clone();
        p.data = Some(data.clone());

        MessageManager::add_message(
            sender_id.clone(),
            msg_id.clone(),
            msg_type as i32,
            client.id_.clone(),
            data.ciphertext.clone(),
        )
        .await;

        if except_sender && client.id_ == sender_id {
            continue;
        }

        if let Some(conn_id) = ClientManager::get_client_connection_by_id(&client.id_).await {
            send_packet(
                conn_id,
                ws_sender.clone(),
                PacketType::ServerBroadcastMessage,
                p,
            )
            .await?;
        }
    }

    Ok(())
}

async fn send_packet<T>(
    conn_id: u32,
    fallback_sender: Arc<Mutex<WsSender>>,
    packet_type: PacketType,
    packet: T,
) -> Result<()>
where
    T: prost::Message,
{
    let ss = {
        let connections = CONNECTIONS.read().await;
        connections
            .get(&conn_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?
    };

    let state = STATE.read().await;

    let start_time = Instant::now();
    let encrypted = Encryption::encrypt_packet(
        packet_type,
        packet,
        &state.dilithium_sk.to_bytes(),
        &ss.as_bytes(),
    )?;
    let end_time = Instant::now();
    info!("加密用时 {}ms", (end_time - start_time).as_millis());

    let target_sender = {
        let senders = SENDERS.read().await;
        senders.get(&conn_id).cloned()
    };

    let sender_arc = target_sender.unwrap_or_else(|| fallback_sender);
    let mut sender_guard = sender_arc.lock().await;
    sender_guard
        .send(Message::Binary(encrypted.into()))
        .await
        .map_err(|e| anyhow::anyhow!("Send error: {:?}", e))?;

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
            let type_ = PacketType::try_from(packet.packet_type);
            if type_.is_err() {
                return Err(anyhow::anyhow!("Illegal data sent by {}", conn_id));
            }
            let type_ = type_.unwrap();
            match type_ {
                PacketType::ClientPreLogin => {
                    let packet = decode_packet!(packet, ClientPreLogin);
                    let client = ClientManager::find_client(&packet.dilithium_pk);
                    let response = if client.is_none() {
                        ServerPreLogin {
                            registered: false,
                            can_register: true,
                            token: None,
                        }
                    } else {
                        let token =
                            TokenManager::generate_token(conn_id, &packet.dilithium_pk).await;
                        if token.is_err() {
                            return Err(anyhow::anyhow!("Failed to generate token"));
                        }
                        let token = token.unwrap();
                        let token = Encryption::kyber_encrypt(&token, &client.unwrap().kyber_pk_)?;
                        ServerPreLogin {
                            registered: true,
                            can_register: false,
                            token: Some(token),
                        }
                    };
                    send_packet(
                        conn_id,
                        ws_sender.clone(),
                        PacketType::ServerPreLogin,
                        response,
                    )
                    .await?;
                }
                PacketType::ClientRegister => {
                    let packet = decode_packet!(packet, ClientRegister);
                    let client = ClientManager::find_client(&packet.dilithium_pk);
                    let mut registered_client = None;
                    let response = if client.is_some() {
                        ServerRegisterResponse {
                            success: false,
                            color: None,
                            information: None,
                            message: Some("您已经注册过了".to_string()),
                        }
                    } else if ClientManager::is_name_taken(&packet.name) {
                        ServerRegisterResponse {
                            success: false,
                            color: None,
                            information: None,
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
                        let clients = ClientManager::get_all_clients().await;
                        ServerRegisterResponse {
                            success: true,
                            color: Some(color),
                            information: Some(ServerInformation {
                                other_clients_pk: clients
                                    .iter()
                                    .map(|client| ServerOtherClientPk {
                                        id: client.id_.clone(),
                                        kyber_pk: client.kyber_pk_.clone(),
                                    })
                                    .collect(),
                            }),
                            message: Some("注册成功".to_string()),
                        }
                    };
                    send_packet(
                        conn_id,
                        ws_sender.clone(),
                        PacketType::ServerRegisterResponse,
                        response,
                    )
                    .await?;

                    if let Some(client) = registered_client {
                        Service::login_client(conn_id, client, ws_sender.clone()).await?;
                    }
                }
                PacketType::ClientLogin => {
                    let packet = decode_packet!(packet, ClientLogin);
                    let token = TokenManager::validate_token(conn_id, &packet.token_sign).await;
                    let mut login_client = None;
                    let response = if token.is_none() {
                        ServerLoginResponse {
                            success: false,
                            information: None,
                            message: Some("身份校验失败".to_string()),
                        }
                    } else {
                        let pk = token.clone().unwrap().1;
                        let client = ClientManager::find_client(&pk).unwrap();
                        login_client.replace(client);
                        let clients = ClientManager::get_all_clients().await;
                        ServerLoginResponse {
                            success: true,
                            information: Some(ServerInformation {
                                other_clients_pk: clients
                                    .iter()
                                    .map(|client| ServerOtherClientPk {
                                        id: client.id_.clone(),
                                        kyber_pk: client.kyber_pk_.clone(),
                                    })
                                    .collect(),
                            }),
                            message: Some("登录成功".to_string()),
                        }
                    };

                    send_packet(
                        conn_id,
                        ws_sender.clone(),
                        PacketType::ServerLoginResponse,
                        response,
                    )
                    .await?;

                    if let Some(client) = login_client {
                        Service::login_client(conn_id, client, ws_sender.clone()).await?;
                    }
                }

                _ => {}
            }
        }
        Some(client) => {
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
                    let msg_id = Uuid::now_v7().to_string();
                    for message in &data {
                        let client = ClientManager::get_client_by_id(&message.id).await;
                        if client.is_none() {
                            continue;
                        }
                        let client = client.unwrap();
                        MessageManager::add_message(
                            sender.id_.clone(),
                            msg_id.clone(),
                            message.msg_type,
                            client.id_.clone(),
                            message.ciphertext.clone(),
                        )
                        .await;
                        if let Some(conn_id) =
                            ClientManager::get_client_connection_by_id(&client.id_.clone()).await
                        {
                            info!("Receiver: {:?}", conn_id);
                            send_packet(
                                conn_id,
                                ws_sender.clone(),
                                PacketType::ServerBroadcastMessage,
                                ServerBroadcastMessage {
                                    sender_id: sender.id_.clone(),
                                    sender_name: sender.name_.clone(),
                                    data: Some(message.clone()),
                                    color: sender.color_,
                                },
                            )
                            .await?;
                        }
                    }
                }
                PacketType::ClientChangeColor => {
                    let packet = decode_packet!(packet, ClientChangeColor);
                    let clients = ClientManager::get_all_clients().await;
                    if clients
                        .iter()
                        .map(|c| c.color_ == packet.color && c.id_ != client.id_)
                        .any(|x| x)
                    {
                        send_packet(
                            conn_id,
                            ws_sender.clone(),
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
                                name: client.name_,
                                old_color: client.color_,
                                new_color: packet.color,
                            }
                            .encode_to_vec(),
                            None,
                            None,
                            None,
                            ws_sender.clone(),
                            true,
                        )
                        .await?;

                        ClientManager::update_color(&client.id_, packet.color).await;

                        send_packet(
                            conn_id,
                            ws_sender.clone(),
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
                _ => {}
            }
        }
    };

    Ok(())
}

async fn handle_connection(stream: TcpStream, addr: std::net::SocketAddr) {
    let ws_stream = tokio_tungstenite::accept_async(stream).await;
    if ws_stream.is_err() {
        info!("Failed to accept connection from {}", addr);
        return;
    }
    let ws_stream = ws_stream.unwrap();
    let (ws_sender_raw, mut ws_receiver) = ws_stream.split();
    let ws_sender = Arc::new(Mutex::new(ws_sender_raw));
    let conn_id = addr.port() as u32;

    // Store sender for global access
    SENDERS.write().await.insert(conn_id, ws_sender.clone());

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                if data.starts_with(b"0RW3LL@HANDSHAKE") {
                    info!("客户端已连接");
                    let state = STATE.read().await;
                    let packet = ClientHello::decode(&data[16..]).unwrap();
                    let (ss, ct) =
                        kyber1024_encapsulate(&PublicKey::from_bytes(&packet.pk).unwrap());
                    let packet = ServerHello {
                        ciphertext: ct.as_bytes().to_vec(),
                        dilithium_pk: state.dilithium_pk.to_bytes().to_vec(),
                    };
                    CONNECTIONS.write().await.insert(conn_id, ss);
                    let mut response = packet.encode_to_vec();
                    response.splice(0..0, b"0RW3LL@HANDSHAKE".to_vec());
                    {
                        let mut sender = ws_sender.lock().await;
                        sender.send(Message::Binary(response.into())).await.unwrap();
                    }
                    info!("已回应客户端");
                } else {
                    let connections = CONNECTIONS.read().await;
                    let ss = connections.get(&conn_id);
                    if ss.is_none() {
                        {
                            let mut sender = ws_sender.lock().await;
                            sender.close().await.unwrap();
                        }
                        break;
                    }
                    let ss = ss.unwrap();
                    let data = Encryption::aes_decrypt(&data, &ss.as_bytes());
                    if data.is_err() {
                        warn!("{}: {:?}", addr, data.err());
                        {
                            let mut sender = ws_sender.lock().await;
                            sender.close().await.unwrap();
                        }
                        break;
                    }
                    let data = data.unwrap();
                    let packet = OrwellSignedPacket::decode(data.as_slice());
                    if packet.is_err() {
                        warn!("{}: {:?}", addr, packet.err());
                        {
                            let mut sender = ws_sender.lock().await;
                            sender.close().await.unwrap();
                        }
                        break;
                    }

                    let result = handle_packet(packet.unwrap(), ws_sender.clone(), conn_id).await;
                    if result.is_err() {
                        warn!("Error: {:?}", result.err());
                        {
                            let mut sender = ws_sender.lock().await;
                            sender.close().await.unwrap();
                        }
                        break;
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
                Service::logout_client(conn_id, ws_sender.clone()).await;
                break;
            }
            Err(e) => {
                warn!("WebSocket 错误: {:?}", e);
                CONNECTIONS.write().await.remove(&conn_id);
                SENDERS.write().await.remove(&conn_id);
                Service::logout_client(conn_id, ws_sender.clone()).await;
                break;
            }
            _ => {}
        }
    }
}

pub fn get_db_connection() -> SqliteConnection {
    SqliteConnection::establish("server.db").unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:1337";
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on: {}", addr);

    tracing_subscriber::fmt::init();

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }

    Ok(())
}
