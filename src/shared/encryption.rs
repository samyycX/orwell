use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::Result;
use crystals_dilithium::dilithium5;
use hkdf::Hkdf;
use lazy_static::lazy_static;
use pbkdf2::pbkdf2_hmac;
use pqcrypto_kyber::{kyber1024_decapsulate, kyber1024_encapsulate};
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret};
use prost::Message;
use sha2::Sha256;
use sha3::{Digest, Sha3_512};

use crate::{
    pb::orwell::{OrwellPacket, OrwellSignedPacket, PacketType},
    shared::helper::get_now_timestamp,
};
use rand::prelude::*;
use std::{collections::VecDeque, sync::Mutex};

const TIME_LIMIT: u64 = 10;
const KYBER1024_CIPHERTEXTBYTES: usize = 1568;

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

        if now_timestamp - data.timestamp > TIME_LIMIT {
            return Err(anyhow::anyhow!("时间戳过期"));
        }

        if !Encryption::check_and_put_salt(&data.salt) {
            return Err(anyhow::anyhow!("盐值重复"));
        }

        Ok(data)
    }

    pub fn pbkdf2_derive_key(original_key: &[u8], salt: &[u8]) -> Key<Aes256Gcm> {
        let mut key = [0u8; 32];

        pbkdf2_hmac::<Sha256>(&original_key, salt, 100_000, &mut key);

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
            return Err(anyhow::anyhow!("Invalid data length"));
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
            return Err(anyhow::anyhow!("Invalid data length"));
        }

        let ct_bytes = &data[..KYBER1024_CIPHERTEXTBYTES];
        let encrypted = &data[KYBER1024_CIPHERTEXTBYTES..];

        let ciphertext =
            Ciphertext::from_bytes(ct_bytes).map_err(|_| anyhow::anyhow!("Invalid ciphertext"))?;
        let secret_key =
            SecretKey::from_bytes(sk).map_err(|_| anyhow::anyhow!("Invalid secret key length"))?;
        let shared_secret = kyber1024_decapsulate(&ciphertext, &secret_key);

        let decrypted = Self::aes_decrypt(encrypted, shared_secret.as_bytes())?;
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
        shared_secret: &[u8],
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
        let data = Self::aes_encrypt(packet.encode_to_vec().as_slice(), shared_secret);
        Ok(data)
    }
}
