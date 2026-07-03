use crate::auth::{offline_access_token, offline_uuid};
use crate::config;
use crate::download::{DownloadProgress, DownloadTask, ParallelDownloader};
use crate::java::ensure_java;
use crate::mojang::{
    fetch_asset_index, library_applies, maven_to_path, native_classifier_key,
    native_classifier_key_alt, resolve_argument_values, AssetIndexJson, VersionJson,
};
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::Path;
use std::process::{Command, Stdio};
use zip::ZipArchive;

pub struct LaunchOptions {
    pub username: String,
    pub ram_mb: u32,
}

pub fn prepare_and_launch(
    version: &VersionJson,
    options: &LaunchOptions,
    downloader: &ParallelDownloader,
    progress_tx: Option<std::sync::mpsc::Sender<DownloadProgress>>,
) -> Result<(), String> {
    let mc_dir = config::minecraft_dir();
    fs::create_dir_all(&mc_dir).map_err(|e| format!("No se pudo crear .minecraft: {e}"))?;
    fs::create_dir_all(mc_dir.join("versions")).map_err(|e| e.to_string())?;
    fs::create_dir_all(mc_dir.join("libraries")).map_err(|e| e.to_string())?;
    fs::create_dir_all(mc_dir.join("assets")).map_err(|e| e.to_string())?;
    fs::create_dir_all(mc_dir.join("natives")).map_err(|e| e.to_string())?;

    let version_dir = mc_dir.join("versions").join(&version.id);
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;

    let client_jar = version_dir.join(format!("{}.jar", version.id));
    let version_json_path = version_dir.join(format!("{}.json", version.id));

    let mut tasks = vec![DownloadTask {
        url: version.downloads.client.url.clone(),
        dest: client_jar.clone(),
        expected_sha1: Some(version.downloads.client.sha1.clone()),
        expected_size: Some(version.downloads.client.size),
        label: format!("Cliente {}", version.id),
    }];

    let version_json_str =
        serde_json::to_string_pretty(version).map_err(|e| format!("Serializar versión: {e}"))?;
    fs::write(&version_json_path, &version_json_str).map_err(|e| e.to_string())?;

    collect_library_tasks(version, &mc_dir, &mut tasks);

    let asset_index_path = mc_dir
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", version.asset_index.id));

    tasks.push(DownloadTask {
        url: version.asset_index.url.clone(),
        dest: asset_index_path.clone(),
        expected_sha1: Some(version.asset_index.sha1.clone()),
        expected_size: Some(version.asset_index.size),
        label: format!("Assets index {}", version.asset_index.id),
    });

    downloader.run(tasks, progress_tx.clone())?;

    let asset_index = if asset_index_path.exists() {
        let content = fs::read_to_string(&asset_index_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| format!("Parsear assets: {e}"))?
    } else {
        fetch_asset_index(&version.asset_index.url)?
    };

    let asset_tasks = collect_asset_tasks(&asset_index, &mc_dir);
    downloader.run(asset_tasks, progress_tx.clone())?;

    let java_exe = ensure_java(version, downloader, progress_tx.clone())?;

    let natives_dir = mc_dir.join("natives").join(&version.id);
    if natives_dir.exists() {
        fs::remove_dir_all(&natives_dir).ok();
    }
    fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;
    extract_natives(version, &mc_dir, &natives_dir)?;

    let cmd = build_launch_command(version, &java_exe, &client_jar, &natives_dir, options)?;
    spawn_game(cmd)?;

    Ok(())
}

fn collect_library_tasks(version: &VersionJson, mc_dir: &Path, tasks: &mut Vec<DownloadTask>) {
    for library in &version.libraries {
        if !library_applies(library) {
            continue;
        }

        let Some(downloads) = &library.downloads else {
            continue;
        };

        if let Some(artifact) = &downloads.artifact {
            let rel = maven_to_path(&library.name);
            tasks.push(DownloadTask {
                url: artifact.url.clone(),
                dest: mc_dir.join("libraries").join(&rel),
                expected_sha1: Some(artifact.sha1.clone()),
                expected_size: Some(artifact.size),
                label: library.name.clone(),
            });
        }

        if let Some(classifiers) = &downloads.classifiers {
            for key in [native_classifier_key(), native_classifier_key_alt()] {
                if let Some(native) = classifiers.get(key) {
                    let native_name = format!("{}:{}", library.name, key);
                    let rel = maven_to_path(&native_name);
                    tasks.push(DownloadTask {
                        url: native.url.clone(),
                        dest: mc_dir.join("libraries").join(&rel),
                        expected_sha1: Some(native.sha1.clone()),
                        expected_size: Some(native.size),
                        label: format!("Native {}", library.name),
                    });
                }
            }
        }
    }
}

fn collect_asset_tasks(asset_index: &AssetIndexJson, mc_dir: &Path) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();
    for (_name, obj) in &asset_index.objects {
        let prefix = &obj.hash[..2];
        let dest = mc_dir
            .join("assets")
            .join("objects")
            .join(prefix)
            .join(&obj.hash);

        // Los assets se guardan por hash: si existe con el tamaño correcto, omitir.
        if dest.exists() {
            if let Ok(meta) = fs::metadata(&dest) {
                if meta.len() == obj.size {
                    continue;
                }
            }
        }

        let url = format!(
            "https://resources.download.minecraft.net/{}/{}",
            prefix, obj.hash
        );
        tasks.push(DownloadTask {
            url,
            dest,
            expected_sha1: Some(obj.hash.clone()),
            expected_size: Some(obj.size),
            label: format!("Asset {}", &obj.hash[..8]),
        });
    }
    tasks
}

fn extract_natives(version: &VersionJson, mc_dir: &Path, natives_dir: &Path) -> Result<(), String> {
    for library in &version.libraries {
        if !library_applies(library) {
            continue;
        }

        let Some(downloads) = &library.downloads else {
            continue;
        };

        let Some(classifiers) = &downloads.classifiers else {
            continue;
        };

        let native_entry = classifiers
            .get(native_classifier_key())
            .or_else(|| classifiers.get(native_classifier_key_alt()));

        let Some(_native) = native_entry else {
            continue;
        };

        let native_name = format!(
            "{}:{}",
            library.name,
            if classifiers.contains_key(native_classifier_key()) {
                native_classifier_key()
            } else {
                native_classifier_key_alt()
            }
        );
        let rel = maven_to_path(&native_name);
        let jar_path = mc_dir.join("libraries").join(&rel);

        if !jar_path.exists() {
            continue;
        }

        let exclude: Vec<String> = library
            .extract
            .as_ref()
            .map(|e| e.exclude.clone())
            .unwrap_or_default();

        extract_native_jar(&jar_path, natives_dir, &exclude)?;
    }
    Ok(())
}

fn extract_native_jar(jar_path: &Path, dest: &Path, exclude: &[String]) -> Result<(), String> {
    let file = File::open(jar_path).map_err(|e| format!("Abrir native jar: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("ZIP native: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();

        if entry.is_dir() {
            continue;
        }

        if exclude.iter().any(|ex| name.starts_with(ex)) {
            continue;
        }

        let out_path = dest.join(
            Path::new(&name)
                .file_name()
                .ok_or_else(|| format!("Nombre inválido en jar: {name}"))?,
        );

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut out = File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn build_classpath(version: &VersionJson, mc_dir: &Path, client_jar: &Path) -> Result<String, String> {
    let mut paths = Vec::new();

    for library in &version.libraries {
        if !library_applies(library) {
            continue;
        }

        if let Some(downloads) = &library.downloads {
            if let Some(_artifact) = &downloads.artifact {
                let rel = maven_to_path(&library.name);
                let path = mc_dir.join("libraries").join(rel);
                if path.exists() {
                    paths.push(path);
                }
            }
        }
    }

    paths.push(client_jar.to_path_buf());

    let separator = ";";
    Ok(paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(separator))
}

fn substitution_map(
    version: &VersionJson,
    mc_dir: &Path,
    client_jar: &Path,
    natives_dir: &Path,
    options: &LaunchOptions,
) -> HashMap<String, String> {
    let uuid = offline_uuid(&options.username);
    let mut map = HashMap::new();

    map.insert(
        "${natives_directory}".to_string(),
        natives_dir.to_string_lossy().into_owned(),
    );
    map.insert(
        "${launcher_name}".to_string(),
        "minelts".to_string(),
    );
    map.insert("${launcher_version}".to_string(), "0.1.0".to_string());
    map.insert(
        "${classpath}".to_string(),
        build_classpath(version, mc_dir, client_jar).unwrap_or_default(),
    );
    map.insert(
        "${library_directory}".to_string(),
        mc_dir.join("libraries").to_string_lossy().into_owned(),
    );
    map.insert(
        "${version_name}".to_string(),
        version.id.clone(),
    );
    map.insert(
        "${game_directory}".to_string(),
        mc_dir.to_string_lossy().into_owned(),
    );
    map.insert(
        "${assets_root}".to_string(),
        mc_dir.join("assets").to_string_lossy().into_owned(),
    );
    map.insert(
        "${assets_index_name}".to_string(),
        version.asset_index.id.clone(),
    );
    map.insert(
        "${auth_player_name}".to_string(),
        options.username.clone(),
    );
    map.insert(
        "${auth_uuid}".to_string(),
        uuid.hyphenated().to_string(),
    );
    map.insert(
        "${auth_access_token}".to_string(),
        offline_access_token().to_string(),
    );
    map.insert("${clientid}".to_string(), String::new());
    map.insert("${auth_xuid}".to_string(), String::new());
    map.insert("${user_type}".to_string(), "msa".to_string());
    map.insert("${version_type}".to_string(), version.version_type.clone());

    map
}

fn substitute(template: &str, map: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in map {
        result = result.replace(key, value);
    }
    result
}

fn build_launch_command(
    version: &VersionJson,
    java_exe: &Path,
    client_jar: &Path,
    natives_dir: &Path,
    options: &LaunchOptions,
) -> Result<Vec<String>, String> {
    let mc_dir = config::minecraft_dir();
    let subs = substitution_map(version, &mc_dir, client_jar, natives_dir, options);

    let mut args = vec![java_exe.to_string_lossy().into_owned()];

    let max_ram = options.ram_mb.to_string();
    let min_ram = (options.ram_mb / 2).max(512).to_string();
    args.push(format!("-Xmx{max_ram}M"));
    args.push(format!("-Xms{min_ram}M"));

    if let Some(arg_block) = &version.arguments {
        for jvm_arg in resolve_argument_values(&arg_block.jvm) {
            if jvm_arg.starts_with("-Xmx") || jvm_arg.starts_with("-Xms") {
                continue;
            }
            args.push(substitute(&jvm_arg, &subs));
        }
    } else {
        args.push(format!(
            "-Djava.library.path={}",
            natives_dir.to_string_lossy()
        ));
    }

    args.push(version.main_class.clone());

    if let Some(arg_block) = &version.arguments {
        for game_arg in resolve_argument_values(&arg_block.game) {
            args.push(substitute(&game_arg, &subs));
        }
    } else if let Some(mc_args) = &version.minecraft_arguments {
        for part in mc_args.split_whitespace() {
            args.push(substitute(part, &subs));
        }
    }

    Ok(args)
}

fn spawn_game(mut cmd_parts: Vec<String>) -> Result<(), String> {
    if cmd_parts.is_empty() {
        return Err("Comando vacío".to_string());
    }

    let program = cmd_parts.remove(0);
    let mut cmd = Command::new(&program);
    cmd.args(&cmd_parts)
        .current_dir(config::minecraft_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn()
        .map_err(|e| format!("No se pudo lanzar Minecraft: {e}"))?;

    Ok(())
}
