use crate::config;
use crate::download::{DownloadTask, ParallelDownloader};
use crate::mojang::{
    fetch_java_runtime_detail, fetch_java_runtime_manifest, fallback_java_component,
    resolve_java_runtime_entry, JavaVersionSpec, VersionJson,
};
use std::path::PathBuf;

const PLATFORM_KEY: &str = "windows-x64";

pub fn resolve_java_spec(version: &VersionJson) -> (String, u32) {
    if let Some(JavaVersionSpec {
        component,
        major_version,
    }) = &version.java_version
    {
        return (component.clone(), *major_version);
    }
    let (component, major) = fallback_java_component(&version.id);
    (component.to_string(), major)
}

pub fn java_executable_path(component: &str) -> PathBuf {
    config::minecraft_dir()
        .join("runtime")
        .join(component)
        .join(PLATFORM_KEY)
        .join("bin")
        .join("java.exe")
}

pub fn ensure_java(
    version: &VersionJson,
    downloader: &ParallelDownloader,
    progress_tx: Option<std::sync::mpsc::Sender<crate::download::DownloadProgress>>,
) -> Result<PathBuf, String> {
    let (component, _major) = resolve_java_spec(version);
    let java_exe = java_executable_path(&component);

    if java_exe.exists() {
        return Ok(java_exe);
    }

    let manifest = fetch_java_runtime_manifest()?;
    let runtime_entry = resolve_java_runtime_entry(&manifest, PLATFORM_KEY, &component)?;

    let detail = fetch_java_runtime_detail(&runtime_entry.manifest.url)?;

    let runtime_root = config::minecraft_dir()
        .join("runtime")
        .join(&component)
        .join(PLATFORM_KEY);

    let mut tasks = Vec::new();

    for (rel_path, file) in &detail.files {
        if file.file_type != "file" && file.file_type != "directory" {
            continue;
        }

        if file.file_type == "directory" {
            let dir_path = runtime_root.join(rel_path);
            std::fs::create_dir_all(&dir_path).map_err(|e| {
                format!("No se pudo crear directorio {}: {e}", dir_path.display())
            })?;
            continue;
        }

        let Some(downloads) = &file.downloads else {
            continue;
        };

        let entry = downloads
            .raw
            .as_ref()
            .or(downloads.lzma.as_ref())
            .ok_or_else(|| format!("Sin URL de descarga para {rel_path}"))?;

        let dest = runtime_root.join(rel_path);
        tasks.push(DownloadTask {
            url: entry.url.clone(),
            dest,
            expected_sha1: Some(entry.sha1.clone()),
            expected_size: Some(entry.size),
            label: format!("Java: {rel_path}"),
        });
    }

    downloader.run(tasks, progress_tx)?;

    let java_exe = find_java_in_runtime(&component).unwrap_or(java_exe);
    if !java_exe.exists() {
        return Err(format!(
            "Java no encontrado tras la descarga: {}",
            java_exe.display()
        ));
    }

    Ok(java_exe)
}

pub fn find_java_in_runtime(component: &str) -> Option<PathBuf> {
    let base = config::minecraft_dir()
        .join("runtime")
        .join(component)
        .join(PLATFORM_KEY);

    let candidates = [
        base.join("bin").join("java.exe"),
        base.join("jre").join("bin").join("java.exe"),
        base.join("bin").join("javaw.exe"),
    ];

    candidates.into_iter().find(|p| p.exists())
}
