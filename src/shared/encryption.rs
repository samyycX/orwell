use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::Result;
use argon2::Argon2;
use crystals_dilithium::dilithium5;
use hkdf::{
    hmac::{Hmac, Mac},
    Hkdf,
};
use lazy_static::lazy_static;
use pqcrypto_kyber::{kyber1024, kyber1024_decapsulate, kyber1024_encapsulate, kyber1024_keypair};
use pqcrypto_traits::{
    kem::{Ciphertext, PublicKey, SecretKey, SharedSecret},
    sign::PublicKey as SignPublicKey,
};
use prost::Message;
use sha2::Sha256;
use sha3::{Digest, Sha3_512};

use crate::{
    pb::orwell::{OrwellPacket, OrwellRatchetPacket, OrwellSignedPacket, PacketType},
    shared::helper::get_now_timestamp,
};
use rand::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

const TIME_LIMIT: u64 = 10000;
const KYBER1024_CIPHERTEXTBYTES: usize = 1568;

#[derive(Clone, PartialEq)]
pub enum RatchetState {
    HandshakePhase1,
    HandshakePhase2,
    HandshakeFinished,
}

#[derive(Clone)]
pub struct KyberDoubleRatchet {
    pub kyber_sk: kyber1024::SecretKey,
    pub kyber_pk: kyber1024::PublicKey,

    root_key: Vec<u8>,
    send_chain_key: Vec<u8>,
    recv_chain_key: Vec<u8>,

    send_chain_counter: u64,
    recv_chain_counter: u64,

    pub ratchet_state: RatchetState,
    remote_pk: Option<kyber1024::PublicKey>,
    skipped_keys: HashMap<(Vec<u8>, u64), Vec<u8>>,
}

impl KyberDoubleRatchet {
    pub fn new() -> Self {
        let (pk, sk) = kyber1024_keypair();
        Self {
            kyber_sk: sk,
            kyber_pk: pk,
            root_key: vec![0u8; 32],
            send_chain_key: vec![0u8; 32],
            recv_chain_key: vec![0u8; 32],
            send_chain_counter: 0,
            recv_chain_counter: 0,
            ratchet_state: RatchetState::HandshakePhase1,
            remote_pk: None,
            skipped_keys: HashMap::new(),
        }
    }

    fn hkdf_derive_key(ikm: &[u8], salt: &[u8], info: String, length: usize) -> Vec<u8> {
        let hk = Hkdf::<Sha256>::new(Some(&salt[..]), &ikm);
        let mut okm = vec![0u8; length];
        hk.expand(&info.as_bytes(), &mut okm).unwrap();
        okm
    }

    pub fn hmac_sha256(data: &[u8], key: &[u8]) -> Vec<u8> {
        let mut hmac = <Hmac<Sha256> as KeyInit>::new_from_slice(&key).unwrap();
        hmac.update(data);
        hmac.finalize().into_bytes().to_vec()
    }

    fn derive_root_key(&mut self, shared_secret: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let new = Self::hkdf_derive_key(
            shared_secret,
            &self.root_key,
            "OrwellKDRDerive".to_string(),
            64,
        );
        let (root_key, other_key) = new.split_at(32);
        Ok((root_key.to_vec(), other_key.to_vec()))
    }

    pub fn initialize_session(&mut self, remote_pk: &[u8]) -> Result<Vec<u8>> {
        let remote_pk = PublicKey::from_bytes(remote_pk)
            .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;
        let (shared_secret, ct) = kyber1024_encapsulate(&remote_pk);
        let mut salt = [0u8; 64];
        let mut rng = rand::thread_rng();
        rng.fill(&mut salt);
        let root_key = Self::hkdf_derive_key(
            shared_secret.as_bytes(),
            &salt,
            "OrwellKDRRootKey".to_string(),
            32,
        );
        self.root_key = root_key;
        self.remote_pk = Some(remote_pk);
        let send_ct = self.step_send_chain()?;

        let mut result = vec![];
        result.extend_from_slice(&salt);
        result.extend_from_slice(ct.as_bytes());
        result.extend_from_slice(send_ct.as_bytes());
        Ok(result)
    }

    pub fn establish_session(
        &mut self,
        remote_ciphertext: &[u8],
        remote_pk: &[u8],
    ) -> Result<kyber1024::Ciphertext> {
        if remote_ciphertext.len() != 64 + 2 * KYBER1024_CIPHERTEXTBYTES {
            return Err(anyhow::anyhow!("Invalid ciphertext length"));
        }

        let salt = &remote_ciphertext[..64];
        let ct = &remote_ciphertext[64..64 + KYBER1024_CIPHERTEXTBYTES];
        let send_ct = &remote_ciphertext[64 + KYBER1024_CIPHERTEXTBYTES..];
        let ct = Ciphertext::from_bytes(ct).map_err(|_| anyhow::anyhow!("Invalid ciphertext"))?;
        let send_ct = Ciphertext::from_bytes(send_ct)
            .map_err(|_| anyhow::anyhow!("Invalid send ciphertext"))?;
        let shared_secret = kyber1024_decapsulate(&ct, &self.kyber_sk);
        let send_shared_secret = kyber1024_decapsulate(&send_ct, &self.kyber_sk);
        let root_key = Self::hkdf_derive_key(
            shared_secret.as_bytes(),
            &salt,
            "OrwellKDRRootKey".to_string(),
            32,
        );
        self.root_key = root_key;
        self.remote_pk = Some(PublicKey::from_bytes(remote_pk)?);
        self.step_recv_chain(send_shared_secret.as_bytes())?;

        let ct = self.step_send_chain()?;

        Ok(ct)
    }

    pub fn finalize_session(&mut self, ciphertext: &[u8]) -> Result<()> {
        let ct = Ciphertext::from_bytes(ciphertext)
            .map_err(|_| anyhow::anyhow!("Invalid ciphertext"))?;
        let shared_secret = kyber1024_decapsulate(&ct, &self.kyber_sk);
        self.step_recv_chain(shared_secret.as_bytes())?;
        Ok(())
    }

    pub fn step_send_chain(&mut self) -> Result<kyber1024::Ciphertext> {
        let (shared_secret, ct) = kyber1024_encapsulate(&self.remote_pk.unwrap());
        let (root_key, send_chain_key) = self.derive_root_key(shared_secret.as_bytes())?;
        self.root_key = root_key;
        self.send_chain_key = send_chain_key;
        self.send_chain_counter = 0;
        Ok(ct)
    }

    pub fn step_recv_chain(&mut self, shared_secret: &[u8]) -> Result<()> {
        let (root_key, recv_chain_key) = self.derive_root_key(shared_secret)?;
        self.root_key = root_key;
        self.recv_chain_key = recv_chain_key;
        self.recv_chain_counter = 0;
        Ok(())
    }

    pub fn encrypt(&mut self, data: OrwellSignedPacket) -> Result<OrwellRatchetPacket> {
        let message_key = Self::hmac_sha256(
            self.send_chain_key.as_slice(),
            "OrwellKDRMessageKey".as_bytes(),
        );

        self.send_chain_key = Self::hmac_sha256(
            self.send_chain_key.as_slice(),
            "OrwellKDRChainKey".as_bytes(),
        );

        let packet = OrwellRatchetPacket {
            kyber_pk: self.kyber_pk.as_bytes().to_vec(),
            send_counter: self.send_chain_counter,
            recv_counter: self.recv_chain_counter,
            data: Encryption::aes_encrypt(data.encode_to_vec().as_slice(), message_key.as_slice()),
        };

        self.send_chain_counter += 1;
        Ok(packet)
    }

    pub fn decrypt(&mut self, packet: OrwellRatchetPacket) -> Result<OrwellSignedPacket> {
        if packet.send_counter > self.recv_chain_counter {
            for i in self.recv_chain_counter..packet.send_counter {
                let skipped_key = Self::hmac_sha256(
                    self.recv_chain_key.as_slice(),
                    "OrwellKDRMessageKey".as_bytes(),
                );
                self.skipped_keys
                    .insert((packet.kyber_pk.clone(), i), skipped_key);
                self.recv_chain_key = Self::hmac_sha256(
                    self.recv_chain_key.as_slice(),
                    "OrwellKDRChainKey".as_bytes(),
                );
            }
        };

        let key_id = (packet.kyber_pk.clone(), packet.send_counter);
        let message_key = if let Some(key) = self.skipped_keys.get(&key_id) {
            key.clone()
        } else {
            let message_key = Self::hmac_sha256(
                self.recv_chain_key.as_slice(),
                "OrwellKDRMessageKey".as_bytes(),
            );
            self.recv_chain_key = Self::hmac_sha256(
                self.recv_chain_key.as_slice(),
                "OrwellKDRChainKey".as_bytes(),
            );
            message_key
        };
        self.skipped_keys.remove(&key_id);

        let plaintext = Encryption::aes_decrypt(packet.data.as_slice(), message_key.as_slice())?;
        let result = OrwellSignedPacket::decode(plaintext.as_slice())?;
        self.recv_chain_counter = packet.send_counter + 1;
        Ok(result)
    }
}

struct Salt {
    timestamp: u64,
    salt: Vec<u8>,
}

lazy_static! {
    static ref SALTS: Mutex<VecDeque<Salt>> = Mutex::new(VecDeque::new());
}

pub struct Encryption {}

impl Encryption {
    pub fn generate_salt() -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut salt = [0u8; 128];
        rng.fill(&mut salt);
        salt.to_vec()
    }

    pub fn check_and_put_salt(salt: &[u8]) -> bool {
        if salt.len() != 128 {
            return false;
        }

        let mut salts = SALTS.lock().unwrap();
        let now_timestamp = get_now_timestamp();

        while salts.len() > 0 && (now_timestamp - salts.front().unwrap().timestamp > TIME_LIMIT) {
            salts.pop_front();
        }

        for s in salts.iter() {
            if s.salt == salt {
                return false;
            }
        }

        salts.push_back(Salt {
            timestamp: now_timestamp,
            salt: salt.to_vec(),
        });

        true
    }

    pub fn hash_packet(packet: &OrwellPacket) -> Vec<u8> {
        let mut hasher = <Sha3_512 as Digest>::new();
        hasher.update(packet.encode_to_vec());
        hasher.finalize().to_vec()
    }

    pub fn validate(
        packet: OrwellSignedPacket,
        dilithium_pk: Option<&dilithium5::PublicKey>,
    ) -> Result<OrwellPacket> {
        if packet.data.is_none() {
            return Err(anyhow::anyhow!("空数据包"));
        }

        let data = packet.data.unwrap();

        if let Some(dilithium_pk) = dilithium_pk {
            let hash = Self::hash_packet(&data);
            if !dilithium_pk.verify(&hash, &packet.sign) {
                return Err(anyhow::anyhow!("数字签名校验失败"));
            }
        }

        let now_timestamp = get_now_timestamp();

        if now_timestamp.abs_diff(data.timestamp) > TIME_LIMIT {
            return Err(anyhow::anyhow!(
                "时间戳过期: 当前={}, 数据={}, 差值={}",
                now_timestamp,
                data.timestamp,
                now_timestamp - data.timestamp
            ));
        }

        if !Encryption::check_and_put_salt(&data.salt) {
            return Err(anyhow::anyhow!("盐值重复"));
        }

        Ok(data)
    }

    pub fn argon2id_derive_key(original_key: &[u8], salt: &[u8]) -> Key<Aes256Gcm> {
        let mut key = [0u8; 32];

        let argon2 = Argon2::default();
        argon2
            .hash_password_into(original_key, salt, &mut key)
            .unwrap();

        Key::<Aes256Gcm>::clone_from_slice(&key)
    }

    pub fn hkdf_derive_key(ikm: &[u8], salt: &[u8]) -> Key<Aes256Gcm> {
        let hk = Hkdf::<Sha256>::new(Some(&salt[..]), &ikm);
        let mut okm = [0u8; 32];
        hk.expand(&[0u8; 0], &mut okm).unwrap();

        Key::<Aes256Gcm>::clone_from_slice(&okm)
    }

    pub fn aes_encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut salt = [0u8; 32];
        let mut nonce = [0u8; 12];

        rng.fill(&mut salt);
        rng.fill(&mut nonce);

        let nonce = Nonce::from_slice(&nonce);
        let key = Self::hkdf_derive_key(key, &salt);
        let cipher = Aes256Gcm::new(&key);
        let encrypted = cipher.encrypt(nonce, data).unwrap();

        let mut result = Vec::new();
        result.extend_from_slice(&salt);
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&encrypted);
        result
    }

    pub fn aes_decrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 44 {
            return Err(anyhow::anyhow!("AES Invalid data length"));
        }

        let salt = &data[..32];
        let nonce = &data[32..44];
        let encrypted = &data[44..];

        let nonce = Nonce::from_slice(nonce);
        let key = Self::hkdf_derive_key(key, &salt);
        let cipher = Aes256Gcm::new(&key);
        let decrypted = cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        Ok(decrypted.to_vec())
    }

    pub fn kyber_encrypt(plaintext: &[u8], pk: &[u8]) -> Result<Vec<u8>> {
        let public_key =
            PublicKey::from_bytes(pk).map_err(|_| anyhow::anyhow!("Invalid public key length"))?;

        let (shared_secret, ct) = kyber1024_encapsulate(&public_key);

        let encrypted = Self::aes_encrypt(plaintext, shared_secret.as_bytes());
        let mut result = Vec::new();
        result.extend_from_slice(ct.as_bytes());
        result.extend_from_slice(&encrypted);
        Ok(result)
    }

    pub fn kyber_decrypt(data: &[u8], sk: &[u8]) -> Result<Vec<u8>> {
        if data.len() < KYBER1024_CIPHERTEXTBYTES + 32 + 12 {
            return Err(anyhow::anyhow!("Kyber Invalid data length"));
        }

        let ct_bytes = &data[..KYBER1024_CIPHERTEXTBYTES];
        let encrypted = &data[KYBER1024_CIPHERTEXTBYTES..];

        let ciphertext =
            Ciphertext::from_bytes(ct_bytes).map_err(|_| anyhow::anyhow!("Invalid ciphertext"))?;
        let secret_key =
            SecretKey::from_bytes(sk).map_err(|_| anyhow::anyhow!("Invalid secret key length"))?;
        let shared_secret = kyber1024_decapsulate(&ciphertext, &secret_key);

        let decrypted = Self::aes_decrypt(encrypted, shared_secret.as_bytes())
            .map_err(|e| anyhow::anyhow!("Kyber Decryption failed: {}", e))?;
        Ok(decrypted)
    }

    pub fn dilithium_sign(data: &[u8], sk: &[u8]) -> Result<Vec<u8>> {
        let mut sk_buf = [0u8; dilithium5::SECRETKEYBYTES];
        if sk.len() != dilithium5::SECRETKEYBYTES {
            return Err(anyhow::anyhow!("Invalid secret key length"));
        }
        sk_buf.copy_from_slice(sk);
        let sk = dilithium5::SecretKey::from_bytes(&sk_buf);
        let sign = sk.sign(data);
        Ok(sign.to_vec())
    }

    pub fn dilithium_verify(data: &[u8], pk: &[u8], sign: &[u8]) -> Result<bool> {
        let mut pk_buf = [0u8; dilithium5::PUBLICKEYBYTES];
        if pk.len() != dilithium5::PUBLICKEYBYTES {
            return Err(anyhow::anyhow!("Invalid public key length"));
        }
        pk_buf.copy_from_slice(pk);
        let pk = dilithium5::PublicKey::from_bytes(&pk_buf);
        let result = pk.verify(data, &sign);
        Ok(result)
    }

    pub fn encrypt_packet<T>(
        packet_type: PacketType,
        packet: T,
        dilithium_sk: &[u8],
        ratchet: &mut KyberDoubleRatchet,
    ) -> Result<Vec<u8>>
    where
        T: prost::Message,
    {
        let packet = OrwellPacket {
            timestamp: get_now_timestamp(),
            salt: Self::generate_salt(),
            packet_type: packet_type as i32,
            data: packet.encode_to_vec(),
        };
        let hash = Self::hash_packet(&packet);
        let sign = Self::dilithium_sign(hash.as_slice(), dilithium_sk)?;
        let packet = OrwellSignedPacket {
            data: Some(packet),
            sign: sign.to_vec(),
        };
        let data = ratchet.encrypt(packet)?;
        Ok(data.encode_to_vec())
    }
}
