use std::{fs, sync::RwLock, thread};

use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use crystals_dilithium::dilithium5;
use lazy_static::lazy_static;
use orwell::{pb::orwell::Profile, shared::encryption::Encryption};
use pqcrypto_kyber::kyber1024_keypair;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret};
use prost::Message;
use rand::Rng;

use crate::{
    config::get_server_url,
    message::{add_chat_message, add_debug_message, MessageLevel},
    network::Network,
    STATE,
};

const PROFILE_FOLDER: &str = "./profiles";

lazy_static! {
    static ref KEY_MANAGER: RwLock<Option<KeyManager>> = RwLock::new(None);
}

pub struct KeyManager {
    profile: Option<Profile>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self { profile: None }
    }

    pub fn get_profile_path(name: &str) -> String {
        format!("{}/{}.orwell", PROFILE_FOLDER, name)
    }

    pub fn create_key(name: &str, password: &str) {
        if !fs::exists(PROFILE_FOLDER).unwrap() {
            fs::create_dir(PROFILE_FOLDER).unwrap();
        }

        let password = password.to_string();
        let name = name.to_string();
        let mut state = STATE.write().unwrap();
        state.processing = true;
        state.logged = false;
        drop(state);
        let mut rng = rand::thread_rng();

        add_debug_message(MessageLevel::Debug, "正在生成Kyber密钥对");
        let (pk, sk) = kyber1024_keypair();

        add_debug_message(MessageLevel::Debug, "正在生成Dilithium密钥对");
        let keys = dilithium5::Keypair::generate(None);

        add_debug_message(MessageLevel::Debug, "正在生成数据");
        let mut data = Profile {
            name: name.to_string(),
            kyber_pk: pk.as_bytes().to_vec(),
            kyber_sk: sk.as_bytes().to_vec(),
            dilithium_pk: keys.public.bytes.to_vec(),
            dilithium_sk: keys.secret.bytes.to_vec(),
        }
        .encode_to_vec();

        add_debug_message(MessageLevel::Debug, "正在进行AES-256-GCM加密");
        let mut salt = [0u8; 32];
        rng.fill(&mut salt);

        let mut nonce = [0u8; 12];
        rng.fill(&mut nonce);

        add_debug_message(MessageLevel::Debug, "正在保存盐和非对称加密密钥");
        let mut saved = vec![];
        saved.extend_from_slice(&salt);
        saved.extend_from_slice(&nonce);

        let key = Encryption::argon2id_derive_key(password.as_bytes(), &salt);
        let cipher = Aes256Gcm::new(&key);
        let nonce = Nonce::from_slice(&nonce);
        data.splice(0..0, "0RW3LL".as_bytes().to_vec());
        let ciphertext = cipher.encrypt(nonce, data.as_slice()).unwrap();
        saved.extend_from_slice(&ciphertext);

        fs::write(Self::get_profile_path(&name), saved).unwrap();

        let keymanager = Self {
            profile: Some(Profile {
                name,
                kyber_pk: pk.as_bytes().to_vec(),
                kyber_sk: sk.as_bytes().to_vec(),
                dilithium_pk: keys.public.bytes.to_vec(),
                dilithium_sk: keys.secret.bytes.to_vec(),
            }),
        };

        let mut key_manager = KEY_MANAGER.write().unwrap();
        key_manager.replace(keymanager);

        add_chat_message("密钥创建成功！您已登录，请使用/connect <服务器地址> 以连接到服务器。");

        {
            let mut state = STATE.write().unwrap();
            state.processing = false;
            state.logged = true;
        }

        // Auto connect if configured
        if let Some(url) = get_server_url() {
            add_debug_message(
                MessageLevel::Info,
                format!("检测到配置的服务器地址 {}，尝试自动连接", url),
            );

            Network::start(url);
        }
    }

    pub fn load_key(name: &str, password: &str) {
        if !fs::exists(Self::get_profile_path(name)).unwrap() {
            add_chat_message("身份不存在！");
            return;
        }

        fn fail() {
            let mut state = STATE.write().unwrap();
            state.processing = false;
            drop(state);
            add_chat_message("密码错误！");
        }
        let name_clone = name.to_string();
        let password_clone = password.to_string();
        let mut state = STATE.write().unwrap();
        state.processing = true;
        drop(state);

        let data = fs::read(Self::get_profile_path(&name_clone)).unwrap();
        let key = Encryption::argon2id_derive_key(password_clone.as_bytes(), &data[0..32]);
        let cipher = Aes256Gcm::new(&key);
        let nonce = Nonce::from_slice(&data[32..44]);
        let plaintext = match cipher.decrypt(nonce, &data[44..]) {
            Ok(result) => result,
            Err(e) => {
                add_debug_message(MessageLevel::Error, format!("解密失败: {}", e));
                fail();
                return;
            }
        };

        let mut plaintext = plaintext.as_slice().to_vec();
        if !plaintext.starts_with("0RW3LL".as_bytes()) {
            fail();
            return;
        }

        plaintext.splice(0..6, vec![]);

        let key_pair = Profile::decode(plaintext.as_slice()).unwrap();

        let mut key_manager = KEY_MANAGER.write().unwrap();
        key_manager.replace(Self {
            profile: Some(key_pair),
        });

        add_debug_message(MessageLevel::Info, "密钥加载成功");
        add_chat_message("登录成功！请使用/connect <服务器地址> 以连接到服务器。");

        {
            let mut state = STATE.write().unwrap();
            state.processing = false;
            state.logged = true;
        }

        // Auto connect if configured
        if let Some(url) = get_server_url() {
            add_debug_message(
                MessageLevel::Info,
                format!("检测到配置的服务器地址 {}，尝试自动连接", url),
            );
            Network::start(url);
        }
    }

    pub fn get_profile() -> Option<Profile> {
        let key_manager = KEY_MANAGER.read().unwrap();
        key_manager.as_ref().unwrap().profile.clone()
    }
}
