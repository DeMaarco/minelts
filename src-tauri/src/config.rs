use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub fn app_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFilterPrefs {
    pub show_releases: bool,
    pub show_snapshots: bool,
    pub show_old: bool,
}

impl Default for VersionFilterPrefs {
    fn default() -> Self {
        Self {
            show_releases: true,
            show_snapshots: false,
            show_old: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub username: String,
    pub version_id: String,
    pub ram_min_mb: u32,
    pub ram_max_mb: u32,
    pub jvm_args: String,
    pub filters: VersionFilterPrefs,
}

/// Legacy config shape for migration from older config.json files.
#[derive(Debug, Deserialize)]
struct LegacyConfig {
    username: Option<String>,
    version_id: Option<String>,
    ram_mb: Option<u32>,
    show_snapshots: Option<bool>,
    ram_min_mb: Option<u32>,
    ram_max_mb: Option<u32>,
    jvm_args: Option<String>,
    filters: Option<VersionFilterPrefs>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: String::new(),
            version_id: String::new(),
            ram_min_mb: 1024,
            ram_max_mb: 2048,
            jvm_args: String::new(),
            filters: VersionFilterPrefs::default(),
        }
    }
}

impl Config {
    pub fn validate_ram(&self) -> Result<(), String> {
        validate_ram_range(self.ram_min_mb, self.ram_max_mb)
    }
}

pub fn validate_ram_range(ram_min_mb: u32, ram_max_mb: u32) -> Result<(), String> {
    const MIN: u32 = 512;
    const MAX: u32 = 16384;

    if ram_min_mb < MIN || ram_min_mb > MAX {
        return Err(format!("RAM mínima debe estar entre {MIN} y {MAX} MB"));
    }
    if ram_max_mb < MIN || ram_max_mb > MAX {
        return Err(format!("RAM máxima debe estar entre {MIN} y {MAX} MB"));
    }
    if ram_min_mb > ram_max_mb {
        return Err("RAM mínima no puede superar la máxima".into());
    }
    Ok(())
}

fn migrate_from_legacy(legacy: LegacyConfig) -> Config {
    let mut config = Config::default();

    if let Some(username) = legacy.username {
        config.username = username;
    }
    if let Some(version_id) = legacy.version_id {
        config.version_id = version_id;
    }
    if let Some(jvm_args) = legacy.jvm_args {
        config.jvm_args = jvm_args;
    }
    if let Some(filters) = legacy.filters {
        config.filters = filters;
    }

    if let (Some(ram_min), Some(ram_max)) = (legacy.ram_min_mb, legacy.ram_max_mb) {
        config.ram_min_mb = ram_min;
        config.ram_max_mb = ram_max;
    } else if let Some(ram_mb) = legacy.ram_mb {
        config.ram_max_mb = ram_mb;
        config.ram_min_mb = (ram_mb / 2).max(512);
    }

    if let Some(show_snapshots) = legacy.show_snapshots {
        config.filters.show_snapshots = show_snapshots;
    }

    if config.validate_ram().is_err() {
        config.ram_min_mb = 1024;
        config.ram_max_mb = 2048;
    }

    config
}

pub fn config_dir() -> PathBuf {
    app_dir()
}

pub fn config_path() -> PathBuf {
    app_dir().join("config.json")
}

pub fn minecraft_dir() -> PathBuf {
    app_dir().join(".minecraft")
}

pub fn load() -> Config {
    let path = config_path();
    if !path.exists() {
        return Config::default();
    }

    let content = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Config::default(),
    };

    if let Ok(config) = serde_json::from_str::<Config>(&content) {
        return config;
    }

    serde_json::from_str::<LegacyConfig>(&content)
        .map(migrate_from_legacy)
        .unwrap_or_default()
}

pub fn save(config: &Config) -> std::io::Result<()> {
    config
        .validate_ram()
        .map_err(std::io::Error::other)?;
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string(config).map_err(std::io::Error::other)?;
    fs::write(config_path(), json)
}
