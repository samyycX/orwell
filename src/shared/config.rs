use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug)]
pub enum ConfigError {
    IoError(io::Error),
    TomlError(toml::de::Error),
    TomlSerializeError(toml::ser::Error),
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::TomlError(err)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(err: toml::ser::Error) -> Self {
        ConfigError::TomlSerializeError(err)
    }
}

pub trait Config: Serialize + for<'de> Deserialize<'de> + Default {
    fn config_file_name() -> &'static str;

    fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_file_name();

        if !Path::new(config_path).exists() {
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(config_path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_file_name();
        let toml_string = toml::to_string_pretty(self)?;

        if let Some(parent) = Path::new(config_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(config_path)?;
        file.write_all(toml_string.as_bytes())?;
        Ok(())
    }
}
