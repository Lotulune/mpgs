#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Minimal Tauri shell. The UI talks to the MPGS server over HTTP directly;
// this process exposes no custom commands beyond the opener plugin, which the
// capabilities file restricts to https/steam URLs.

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
