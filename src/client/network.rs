use crystals_dilithium::dilithium5;
use lazy_static::lazy_static;
use orwell::pb::orwell::{
    ClientHello, ClientPreLogin, OrwellSignedPacket, PacketType, ServerHello,
};
use orwell::shared::encryption::Encryption;
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

    sk: Option<SecretKey>,
    shared_secret: Option<Vec<u8>>,
    dilithium_pk: Option<Vec<u8>>,
}

impl Network {
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(NetworkCommand::Close);
    }

    pub fn start(server_url: String) {
        let server_url = format!("ws://{}", server_url);
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
                                    break;
                                }

                                if let Ok(_) = result {
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
                let mut network_guard = NETWORK.write().unwrap();
                if let Some(network) = network_guard.as_mut() {
                    network.on_message(data);
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
            sk: None,
            shared_secret: None,
            dilithium_pk: None,
        });

        // Kyber handshake (client -> server)
        let (pk, sk) = kyber1024_keypair();
        if let Some(net) = network_lock.as_mut() {
            net.sk = Some(sk);
        }

        let mut client_hello = ClientHello {
            pk: pk.as_bytes().to_vec(),
        }
        .encode_to_vec();
        client_hello.splice(0..0, b"0RW3LL@HANDSHAKE".to_vec());
        network_lock
            .as_ref()
            .unwrap()
            .cmd_tx
            .send(NetworkCommand::Send(client_hello))
            .unwrap();
    }

    pub fn on_message(&mut self, data: Vec<u8>) {
        if data.starts_with(b"0RW3LL@HANDSHAKE") {
            let payload = &data[16..];
            let server_hello = match ServerHello::decode(payload) {
                Ok(p) => p,
                Err(e) => {
                    add_chat_message("连接至服务器失败");
                    add_debug_message(
                        MessageLevel::Error,
                        format!("服务端握手包校验不通过: {:?}", e),
                    );
                    self.shutdown();
                    return;
                }
            };

            let ct = match Ciphertext::from_bytes(&server_hello.ciphertext) {
                Ok(c) => c,
                Err(_) => {
                    add_chat_message("连接至服务器失败");
                    add_debug_message(
                        MessageLevel::Error,
                        "服务端握手包ciphertext非法".to_string(),
                    );
                    return;
                }
            };

            let shared_secret = kyber1024_decapsulate(&ct, self.sk.as_ref().unwrap());
            self.shared_secret = Some(shared_secret.as_bytes().to_vec());
            self.dilithium_pk = Some(server_hello.dilithium_pk);
            self.connected = true;

            add_chat_message("已协调Kyber密钥和dilithium公钥，连接至服务器成功");
            add_debug_message(MessageLevel::Info, "连接至服务器成功");

            let profile = KeyManager::get_profile().unwrap();
            let packet = ClientPreLogin {
                dilithium_pk: profile.dilithium_pk.to_vec(),
            };
            self.send_packet(PacketType::ClientPreLogin, packet);
        } else {
            let decrypted =
                match Encryption::aes_decrypt(&data, self.shared_secret.as_ref().unwrap()) {
                    Ok(p) => p,
                    Err(e) => {
                        add_debug_message(MessageLevel::Error, format!("AES解密失败 {:?}", e));
                        return;
                    }
                };

            let packet = match OrwellSignedPacket::decode(&decrypted[..]) {
                Ok(p) => p,
                Err(_) => {
                    add_debug_message(MessageLevel::Error, "数据包解码失败");
                    return;
                }
            };

            let packet = match Encryption::validate(
                packet,
                Some(&dilithium5::PublicKey::from_bytes(
                    self.dilithium_pk.as_ref().unwrap(),
                )),
            ) {
                Ok(p) => p,
                Err(e) => {
                    add_debug_message(MessageLevel::Error, e.to_string());
                    return;
                }
            };

            if let Err(e) = Service::handle_packet(packet, self) {
                add_debug_message(MessageLevel::Error, e.to_string());
            }
        }
    }

    pub fn send(data: Vec<u8>) {
        if let Some(net) = NETWORK.read().unwrap().as_ref() {
            let _ = net.cmd_tx.send(NetworkCommand::Send(data));
        }
    }

    pub fn send_packet<T: prost::Message>(&self, packet_type: PacketType, packet: T) {
        match Encryption::encrypt_packet(
            packet_type,
            packet,
            &KeyManager::get_profile().unwrap().dilithium_sk,
            self.shared_secret.as_ref().unwrap(),
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
}
