use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub username: String,
    pub version_id: String,
    pub show_snapshots: bool,
    pub ram_mb: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: String::new(),
            version_id: String::new(),
            show_snapshots: false,
            ram_mb: 2048,
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("minelts")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn minecraft_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft")
}

pub fn load() -> Config {
    let path = config_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Config::default()
    }
}

pub fn save(config: &Config) -> std::io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(config).map_err(std::io::Error::other)?;
    fs::write(config_path(), json)
}
