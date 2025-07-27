use lazy_static::lazy_static;
use orwell::shared::config::{Config, ConfigError};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientConfig {
    pub server_url: Option<String>,
}

impl Config for ClientConfig {
    fn config_file_name() -> &'static str {
        "./orwell-client.toml"
    }
}

lazy_static! {
    static ref CONFIG: RwLock<ClientConfig> = RwLock::new(ClientConfig::load().unwrap_or_default());
}

/// Get a copy of current configuration
pub fn get_config() -> ClientConfig {
    CONFIG.read().unwrap().clone()
}

/// Convenience helper to fetch the auto-connect URL if present.
pub fn get_server_url() -> Option<String> {
    CONFIG.read().unwrap().server_url.clone()
}

/// Reload configuration from file
pub fn reload_config() -> Result<(), ConfigError> {
    let new_config = ClientConfig::load()?;
    *CONFIG.write().unwrap() = new_config;
    Ok(())
}

/// Save current configuration to file
pub fn save_config(config: &ClientConfig) -> Result<(), ConfigError> {
    config.save()?;
    *CONFIG.write().unwrap() = config.clone();
    Ok(())
}
