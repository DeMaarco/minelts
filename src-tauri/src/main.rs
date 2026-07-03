#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod commands;
mod config;
mod download;
mod installed;
mod java;
mod launch;
mod mojang;
mod system_ram;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_versions,
            commands::get_installed,
            commands::get_minecraft_dir,
            commands::get_system_ram_mb,
            commands::open_folder,
            commands::install_version,
            commands::launch_game,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar minelts");
}
