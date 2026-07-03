#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod commands;
mod config;
mod download;
mod installed;
mod java;
mod launch;
mod mojang;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_versions,
            commands::get_installed,
            commands::get_minecraft_dir,
            commands::open_folder,
            commands::launch_game,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar minelts");
}
