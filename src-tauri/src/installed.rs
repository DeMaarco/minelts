use crate::config;
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct InstalledVersion {
    pub id: String,
    pub jar_path: PathBuf,
}

/// Escanea `.minecraft/versions` y devuelve versiones con client jar presente.
pub fn list_installed() -> Vec<InstalledVersion> {
    let versions_dir = config::minecraft_dir().join("versions");
    let mut installed = Vec::new();

    let Ok(entries) = fs::read_dir(&versions_dir) else {
        return installed;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = entry.file_name().to_string_lossy().into_owned();
        let jar_path = path.join(format!("{id}.jar"));
        if jar_path.exists() {
            installed.push(InstalledVersion { id, jar_path });
        }
    }

    installed.sort_by(|a, b| compare_versions(&b.id, &a.id));
    installed
}

#[allow(dead_code)]
pub fn is_installed(version_id: &str) -> bool {
    let jar = config::minecraft_dir()
        .join("versions")
        .join(version_id)
        .join(format!("{version_id}.jar"));
    jar.exists()
}

fn compare_versions(a: &str, b: &str) -> Ordering {
    let pa = version_parts(a);
    let pb = version_parts(b);
    pa.cmp(&pb)
}

fn version_parts(v: &str) -> Vec<u32> {
    v.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

#[cfg(windows)]
pub fn open_folder(path: &std::path::Path) {
    let _ = std::process::Command::new("explorer")
        .arg(path.as_os_str())
        .spawn();
}

#[cfg(not(windows))]
pub fn open_folder(path: &std::path::Path) {
    let _ = path;
}
