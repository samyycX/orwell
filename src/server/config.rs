use configparser::ini::Ini;
use lazy_static::lazy_static;
use std::fmt::Debug;
use std::path::Path;
use std::str::FromStr;
use std::sync::RwLock;

const CONFIG_FILE: &str = "./orwell.ini";

#[derive(Clone, Default)]
pub struct ServerConfig {
    pub port: Option<u16>,
    pub use_tls: Option<bool>,
    pub cert_key_path: Option<String>,
    pub cert_fullchain_path: Option<String>,
}

impl ServerConfig {
    fn parse<T>(ini: &Ini, section: &str, key: &str) -> Option<T>
    where
        T: FromStr,
        T::Err: Debug,
    {
        ini.get(section, key)
            .map(|v| v.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|v| v.parse::<T>().unwrap())
    }

    fn load_from_file() -> Self {
        if !Path::new(CONFIG_FILE).exists() {
            let mut ini = Ini::new();
            ini.set("config", "port", Some(1337.to_string()));
            ini.set("config", "use_tls", Some(false.to_string()));
            ini.set("config", "cert_key_path", Some(String::new()));
            ini.set("config", "cert_fullchain_path", Some(String::new()));
            let _ = ini.write(CONFIG_FILE); // ignore errors
            return ServerConfig::default();
        }

        let mut ini = Ini::new();
        if ini.load(CONFIG_FILE).is_err() {
            return ServerConfig::default();
        }

        ServerConfig {
            port: Self::parse(&ini, "config", "port"),
            use_tls: Self::parse(&ini, "config", "use_tls"),
            cert_key_path: Self::parse(&ini, "config", "cert_key_path"),
            cert_fullchain_path: Self::parse(&ini, "config", "cert_fullchain_path"),
        }
    }
}

lazy_static! {
    static ref CONFIG: RwLock<ServerConfig> = RwLock::new(ServerConfig::load_from_file());
}

/// Get a copy of current configuration
pub fn get_config() -> ServerConfig {
    CONFIG.read().unwrap().clone()
}

pub fn get_port() -> u16 {
    get_config().port.unwrap_or(1337)
}

pub fn get_use_tls() -> bool {
    get_config().use_tls.unwrap_or(false)
}

pub fn get_cert_key_path() -> Option<String> {
    get_config().cert_key_path.clone()
}

pub fn get_cert_fullchain_path() -> Option<String> {
    get_config().cert_fullchain_path.clone()
}
