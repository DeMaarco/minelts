use crate::config::{self, Config};
use crate::download::{DownloadProgress, ParallelDownloader};
use crate::installed;
use crate::launch::{prepare_and_launch, LaunchOptions};
use crate::mojang::{fetch_version_json, fetch_version_manifest, VersionEntry};
use serde::Serialize;
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
pub struct LaunchDoneEvent {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
pub fn open_folder(which: String) -> Result<(), String> {
    let path = match which.as_str() {
        "root" => config::minecraft_dir(),
        "versions" => config::minecraft_dir().join("versions"),
        _ => return Err(format!("Carpeta desconocida: {which}")),
    };
    installed::open_folder(&path);
    Ok(())
}

#[tauri::command]
pub fn launch_game(
    app: AppHandle,
    username: String,
    version_id: String,
    url: String,
    ram_mb: u32,
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
                &LaunchOptions { username, ram_mb },
                &downloader,
                Some(dl_tx),
            )
        })();

        let payload = match result {
            Ok(()) => LaunchDoneEvent {
                ok: true,
                error: None,
            },
            Err(e) => LaunchDoneEvent {
                ok: false,
                error: Some(e),
            },
        };
        let _ = app.emit("launch-done", payload);
    });

    Ok(())
}
