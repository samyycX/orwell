use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use lazy_static::lazy_static;
use orwell::shared::encryption::Encryption;
use rand::Rng;
use tokio::sync::RwLock;

lazy_static! {
    static ref TOKEN_MANAGER: Arc<RwLock<TokenManager>> =
        Arc::new(RwLock::new(TokenManager::new()));
}
#[derive(Clone)]
pub struct TokenAndPk(pub Vec<u8>, pub Vec<u8>);
pub struct TokenManager {
    tokens: HashMap<u32, TokenAndPk>,
}

impl TokenManager {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    pub async fn has_token(conn_id: u32) -> bool {
        let token_manager = TOKEN_MANAGER.read().await;
        token_manager.tokens.contains_key(&conn_id)
    }

    pub async fn generate_token(conn_id: u32, dilithium_pk: &[u8]) -> Result<Vec<u8>> {
        if Self::has_token(conn_id).await {
            return Err(anyhow::anyhow!("Token already exists"));
        }
        let mut token_manager = TOKEN_MANAGER.write().await;
        let mut rng = rand::thread_rng();
        let mut token = [0u8; 128];
        rng.fill(&mut token);
        token_manager
            .tokens
            .insert(conn_id, TokenAndPk(token.to_vec(), dilithium_pk.to_vec()));
        Ok(token.to_vec())
    }

    pub async fn validate_token(conn_id: u32, signed_token: &[u8]) -> Option<TokenAndPk> {
        let mut token_manager = TOKEN_MANAGER.write().await;
        let token = token_manager.tokens.get(&conn_id).cloned();
        if token.is_none() {
            return None;
        }
        let token_and_pk = token.unwrap();
        let result = Encryption::dilithium_verify(&token_and_pk.0, &token_and_pk.1, signed_token);
        token_manager.tokens.remove(&conn_id);
        if result.is_err() || !result.unwrap() {
            None
        } else {
            Some(token_and_pk)
        }
    }
}
