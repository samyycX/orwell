use configparser::ini::Ini;
use lazy_static::lazy_static;
use std::path::Path;
use std::sync::RwLock;


const CONFIG_FILE: &str = "./orwell.ini";

#[derive(Clone, Default)]
pub struct ClientConfig {
    pub server_url: Option<String>,
}

impl ClientConfig {
    fn load_from_file() -> Self {
        if !Path::new(CONFIG_FILE).exists() {
            let mut ini = Ini::new();
            ini.set("config", "server_url", Some(String::new()));
            let _ = ini.write(CONFIG_FILE); // ignore errors
            return ClientConfig::default();
        }

        let mut ini = Ini::new();
        if ini.load(CONFIG_FILE).is_err() {
            return ClientConfig::default();
        }

        let server_url = ini
            .get("config", "server_url")
            .map(|v| v.trim().to_string())
            .filter(|s| !s.is_empty());

        ClientConfig { server_url }
    }
}

lazy_static! {
    static ref CONFIG: RwLock<ClientConfig> = RwLock::new(ClientConfig::load_from_file());
}

/// Get a copy of current configuration
pub fn get_config() -> ClientConfig {
    CONFIG.read().unwrap().clone()
}

/// Convenience helper to fetch the auto-connect URL if present.
pub fn get_server_url() -> Option<String> {
    CONFIG.read().unwrap().server_url.clone()
}
