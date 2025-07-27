use lazy_static::lazy_static;
use orwell::shared::config::{Config, ConfigError};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: Option<u16>,
    pub cert_key_path: Option<String>,
    pub cert_fullchain_path: Option<String>,
}

impl Config for ServerConfig {
    fn config_file_name() -> &'static str {
        "./orwell-server.toml"
    }
}

impl ServerConfig {
    pub fn port_or_default(&self) -> u16 {
        self.port.unwrap_or(1337)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: Some(1337),
            cert_key_path: Some(String::new()),
            cert_fullchain_path: Some(String::new()),
        }
    }
}

lazy_static! {
    static ref CONFIG: RwLock<ServerConfig> = RwLock::new(ServerConfig::load().unwrap_or_default());
}

/// Get a copy of current configuration
pub fn get_config() -> ServerConfig {
    CONFIG.read().unwrap().clone()
}

pub fn get_port() -> u16 {
    get_config().port_or_default()
}

pub fn get_cert_key_path() -> Option<String> {
    get_config().cert_key_path.clone()
}

pub fn get_cert_fullchain_path() -> Option<String> {
    get_config().cert_fullchain_path.clone()
}

/// Reload configuration from file
pub fn reload_config() -> Result<(), ConfigError> {
    let new_config = ServerConfig::load()?;
    *CONFIG.write().unwrap() = new_config;
    Ok(())
}

/// Save current configuration to file
pub fn save_config(config: &ServerConfig) -> Result<(), ConfigError> {
    config.save()?;
    *CONFIG.write().unwrap() = config.clone();
    Ok(())
}
