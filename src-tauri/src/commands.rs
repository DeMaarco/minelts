use crate::config::{self, validate_ram_range, Config};
use crate::download::{DownloadProgress, ParallelDownloader};
use crate::installed;
use crate::launch::{parse_jvm_args, prepare_and_launch, prepare_version, LaunchOptions};
use crate::mojang::{fetch_version_json, fetch_version_manifest, VersionEntry};
use serde::Serialize;
use std::fs;
use std::sync::mpsc;
use std::thread;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
pub struct ProgressEvent {
    pub completed: usize,
    pub total: usize,
    pub current: String,
}

#[derive(Serialize, Clone)]
pub struct DoneEvent {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SystemRamInfo {
    pub total_mb: u32,
}

#[derive(Serialize)]
pub struct InstalledVersionDto {
    pub id: String,
    pub jar_path: String,
}

fn progress_to_event(progress: DownloadProgress) -> ProgressEvent {
    match progress {
        DownloadProgress::Started { label } => ProgressEvent {
            completed: 0,
            total: 0,
            current: label,
        },
        DownloadProgress::Progress { completed, total } => ProgressEvent {
            completed,
            total,
            current: String::new(),
        },
        DownloadProgress::Failed { label, error } => ProgressEvent {
            completed: 0,
            total: 0,
            current: format!("{label}: {error}"),
        },
        DownloadProgress::Completed => ProgressEvent {
            completed: 0,
            total: 0,
            current: String::new(),
        },
    }
}

#[tauri::command]
pub fn get_config() -> Config {
    config::load()
}

#[tauri::command]
pub fn save_config(cfg: Config) -> Result<(), String> {
    cfg.validate_ram()?;
    config::save(&cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_versions() -> Result<Vec<VersionEntry>, String> {
    fetch_version_manifest().map(|m| m.versions)
}

#[tauri::command]
pub fn get_installed() -> Vec<InstalledVersionDto> {
    installed::list_installed()
        .into_iter()
        .map(|v| InstalledVersionDto {
            id: v.id,
            jar_path: v.jar_path.display().to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn get_minecraft_dir() -> String {
    config::minecraft_dir().display().to_string()
}

#[tauri::command]
pub fn get_system_ram_mb() -> SystemRamInfo {
    const MIN_MB: u32 = 512;
    const MAX_MB: u32 = 16384;

    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let total_mb = (system.total_memory() / 1024 / 1024) as u32;
    let total_mb = total_mb.clamp(MIN_MB, MAX_MB);

    SystemRamInfo { total_mb }
}

#[tauri::command]
pub fn open_folder(which: String) -> Result<(), String> {
    let path = match which.as_str() {
        "root" => config::minecraft_dir(),
        "versions" => config::minecraft_dir().join("versions"),
        "logs" => config::minecraft_dir().join("logs"),
        "crashes" => config::minecraft_dir().join("crash-reports"),
        _ => return Err(format!("Carpeta desconocida: {which}")),
    };
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    installed::open_folder(&path);
    Ok(())
}

#[tauri::command]
pub fn install_version(app: AppHandle, version_id: String, url: String) -> Result<(), String> {
    if version_id.is_empty() {
        return Err("Selecciona una versión".into());
    }

    thread::spawn(move || {
        let (dl_tx, dl_rx) = mpsc::channel();
        let app_progress = app.clone();
        thread::spawn(move || {
            while let Ok(progress) = dl_rx.recv() {
                let event = progress_to_event(progress);
                let _ = app_progress.emit("download-progress", event);
            }
        });

        let result = (|| {
            let version = fetch_version_json(&url)?;
            let downloader = ParallelDownloader::new(8);
            prepare_version(&version, &downloader, Some(dl_tx)).map(|_| ())
        })();

        let payload = match result {
            Ok(()) => DoneEvent {
                ok: true,
                error: None,
            },
            Err(e) => DoneEvent {
                ok: false,
                error: Some(e),
            },
        };
        let _ = app.emit("install-done", payload);
    });

    Ok(())
}

#[tauri::command]
pub fn launch_game(
    app: AppHandle,
    username: String,
    version_id: String,
    url: String,
    ram_min_mb: u32,
    ram_max_mb: u32,
    jvm_args: String,
) -> Result<(), String> {
    let username = username.trim().to_string();
    if username.is_empty() {
        return Err("Introduce un nombre de usuario".into());
    }
    if username.len() > 16 {
        return Err("El nombre no puede superar 16 caracteres".into());
    }
    if version_id.is_empty() {
        return Err("Selecciona una versión".into());
    }
    validate_ram_range(ram_min_mb, ram_max_mb)?;

    let jvm_args_vec = parse_jvm_args(&jvm_args);

    thread::spawn(move || {
        let (dl_tx, dl_rx) = mpsc::channel();
        let app_progress = app.clone();
        thread::spawn(move || {
            while let Ok(progress) = dl_rx.recv() {
                let event = progress_to_event(progress);
                let _ = app_progress.emit("download-progress", event);
            }
        });

        let result = (|| {
            let version = fetch_version_json(&url)?;
            let downloader = ParallelDownloader::new(8);
            prepare_and_launch(
                &version,
                &LaunchOptions {
                    username,
                    ram_min_mb,
                    ram_max_mb,
                    jvm_args: jvm_args_vec,
                },
                &downloader,
                Some(dl_tx),
            )
        })();

        let payload = match result {
            Ok(()) => DoneEvent {
                ok: true,
                error: None,
            },
            Err(e) => DoneEvent {
                ok: false,
                error: Some(e),
            },
        };
        let _ = app.emit("launch-done", payload);
    });

    Ok(())
}
