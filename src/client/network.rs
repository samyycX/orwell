use anyhow::Result;
use crystals_dilithium::dilithium5;
use lazy_static::lazy_static;
use orwell::pb::orwell::{
    ClientHello, ClientHello2, ClientPreLogin, OrwellRatchetPacket, OrwellSignedPacket, PacketType,
    ServerHello,
};
use orwell::shared::encryption::{Encryption, KyberDoubleRatchet, RatchetState};
use orwell::shared::helper::get_version;
use pqcrypto_kyber::kyber1024::SecretKey;
use pqcrypto_kyber::{kyber1024_decapsulate, kyber1024_keypair};
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SharedSecret};
use prost::Message as ProstMessage;
use std::sync::{mpsc, RwLock};
use std::thread;
use tokio::sync::mpsc as async_mpsc;
use tracing_subscriber::fmt::format;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::key::KeyManager;
use crate::message::{add_chat_message, add_debug_message, MessageLevel};
use crate::service::Service;
use crate::STATE;

lazy_static! {
    pub static ref NETWORK: RwLock<Option<Network>> = RwLock::new(None);
}
#[derive(Debug)]
pub enum NetworkCommand {
    Send(Vec<u8>),
    Close,
}

pub struct Network {
    server_url: String,
    connected: bool,
    msg_tx: mpsc::Sender<Vec<u8>>,
    cmd_tx: async_mpsc::UnboundedSender<NetworkCommand>,
    ws_thread: thread::JoinHandle<()>,
    msg_thread: thread::JoinHandle<()>,

    ratchet: Option<KyberDoubleRatchet>,
    ratchet_remote_pk: Option<Vec<u8>>,
    dilithium_pk: Option<Vec<u8>>,
}

impl Network {
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(NetworkCommand::Close);
    }

    pub fn start(server_url: String) {
        let mut state = STATE.write().unwrap();
        state.server_url = server_url.clone();
        drop(state);

        let server_url = format!("wss://{}", server_url);
        let (msg_tx, msg_rx) = mpsc::channel();
        let (cmd_tx, mut cmd_rx) = async_mpsc::unbounded_channel();

        let server_url_cloned = server_url.clone();
        let msg_tx_cloned = msg_tx.clone();

        let ws_thread = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build Tokio runtime");

            rt.block_on(async move {
                let (ws_stream, _) = match connect_async(&server_url_cloned).await {
                    Ok(v) => v,
                    Err(e) => {
                        add_debug_message(
                            MessageLevel::Error,
                            format!("Websocket 连接失败: {}", e),
                        );
                        return;
                    }
                };

                let (mut write, mut read) = ws_stream.split();

                let writer = tokio::spawn(async move {
                    while let Some(cmd) = cmd_rx.recv().await {
                        add_debug_message(MessageLevel::Info, format!("WS收到命令"));
                        match cmd {
                            NetworkCommand::Send(raw) => {
                                let result = write.send(Message::Binary(raw.clone().into())).await;

                                if result.is_err() {
                                    add_debug_message(
                                        MessageLevel::Error,
                                        format!("发送数据失败: {}", result.err().unwrap()),
                                    );
                                    let mut network_guard = NETWORK.write().unwrap();
                                    if let Some(network) = network_guard.as_mut() {
                                        network.shutdown();
                                    }
                                    break;
                                }

                                if let Ok(_) = result {
                                    let mut state = STATE.write().unwrap();
                                    state.processed_bytes += raw.len() as u64;
                                    drop(state);
                                    add_debug_message(
                                        MessageLevel::Info,
                                        format!("↑ {:?} Bytes", raw.len()),
                                    );
                                }
                            }
                            NetworkCommand::Close => {
                                let _ = write.close().await;
                                break;
                            }
                        }
                    }
                });

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Binary(data)) => {
                            let _ = msg_tx_cloned.send(data.to_vec());
                        }
                        Ok(_) => {}
                        Err(e) => {
                            add_debug_message(
                                MessageLevel::Error,
                                format!("Websocket 读取错误: {}", e),
                            );
                            return;
                        }
                    };
                }

                let _ = writer.await;
            });
        });

        let msg_thread = thread::spawn(move || {
            while let Ok(data) = msg_rx.recv() {
                add_debug_message(MessageLevel::Info, format!("↓ {:?} Bytes", data.len()));
                let mut state = STATE.write().unwrap();
                state.processed_bytes += data.len() as u64;
                drop(state);
                let mut network_guard = NETWORK.write().unwrap();
                if let Some(network) = network_guard.as_mut() {
                    if let Err(e) = network.on_message(data) {
                        add_debug_message(MessageLevel::Error, e.to_string());
                        network.shutdown();
                        return;
                    }
                }
            }
        });

        let mut network_lock = NETWORK.write().unwrap();
        if let Some(old) = network_lock.as_ref() {
            old.shutdown();
        }

        *network_lock = Some(Network {
            server_url,
            connected: false,
            msg_tx,
            cmd_tx: cmd_tx.clone(),
            ws_thread,
            msg_thread,
            ratchet: Some(KyberDoubleRatchet::new()),
            ratchet_remote_pk: None,
            dilithium_pk: None,
        });

        let ratchet = network_lock.as_ref().unwrap().ratchet.as_ref().unwrap();
        let client_hello = ClientHello {
            pk: ratchet.kyber_pk.as_bytes().to_vec(),
        }
        .encode_to_vec();
        network_lock
            .as_ref()
            .unwrap()
            .cmd_tx
            .send(NetworkCommand::Send(client_hello))
            .unwrap();
    }

    pub fn on_message(&mut self, data: Vec<u8>) -> Result<()> {
        let state = self.ratchet.as_ref().unwrap().ratchet_state.clone();
        match state {
            RatchetState::HandshakePhase1 => {
                let server_hello = ServerHello::decode(data.as_slice())?;

                let result = self
                    .ratchet
                    .as_mut()
                    .unwrap()
                    .establish_session(&server_hello.ciphertext, &server_hello.pk)?;

                self.connected = true;
                self.ratchet_remote_pk = Some(server_hello.pk);
                self.dilithium_pk = Some(server_hello.dilithium_pk);

                add_chat_message("已协调棘轮，连接至服务器成功");
                add_debug_message(MessageLevel::Info, "连接至服务器成功");

                let profile = KeyManager::get_profile().unwrap();

                let client_hello2 = ClientHello2 {
                    ciphertext: result.as_bytes().to_vec(),
                };

                let client_hello2 = client_hello2.encode_to_vec();
                self.cmd_tx.send(NetworkCommand::Send(client_hello2))?;

                //self.send_packet(PacketType::ClientPreLogin, packet);
                self.ratchet.as_mut().unwrap().ratchet_state = RatchetState::HandshakePhase2;
            }
            RatchetState::HandshakePhase2 => {
                let profile = KeyManager::get_profile().unwrap();
                let packet = ClientPreLogin {
                    dilithium_pk: profile.dilithium_pk.to_vec(),
                    version: get_version(),
                };
                self.send_packet(PacketType::ClientPreLogin, packet);
                self.ratchet.as_mut().unwrap().ratchet_state = RatchetState::HandshakeFinished;
            }
            RatchetState::HandshakeFinished => {
                let data = OrwellRatchetPacket::decode(data.as_slice())?;
                let packet = self.ratchet.as_mut().unwrap().decrypt(data)?;

                let packet = Encryption::validate(
                    packet,
                    Some(&dilithium5::PublicKey::from_bytes(
                        self.dilithium_pk.as_ref().unwrap(),
                    )),
                )?;

                Service::handle_packet(packet, self)?;
            }
        }

        Ok(())
    }

    pub fn send(data: Vec<u8>) {
        if let Some(net) = NETWORK.read().unwrap().as_ref() {
            let _ = net.cmd_tx.send(NetworkCommand::Send(data));
        }
    }

    pub fn send_packet<T: prost::Message>(&mut self, packet_type: PacketType, packet: T) {
        let mut state = STATE.write().unwrap();
        state.ratchet_roll_time += 1;
        drop(state);
        match Encryption::encrypt_packet(
            packet_type,
            packet,
            &KeyManager::get_profile().unwrap().dilithium_sk,
            self.ratchet.as_mut().unwrap(),
        ) {
            Ok(encrypted) => {
                let _ = self.cmd_tx.send(NetworkCommand::Send(encrypted));
            }
            Err(e) => add_debug_message(MessageLevel::Error, e.to_string()),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn ratchet_step(&mut self, ct: Vec<u8>) -> Result<()> {
        let ct = Ciphertext::from_bytes(&ct)?;
        let shared_secret = kyber1024_decapsulate(&ct, &self.ratchet.as_ref().unwrap().kyber_sk);
        self.ratchet
            .as_mut()
            .unwrap()
            .step_recv_chain(shared_secret.as_bytes())?;
        Ok(())
    }
}
