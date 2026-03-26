pub mod commands;
pub mod converter;
pub mod identifier;
pub mod scanner;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan_folder,
            commands::get_libreoffice_path,
            commands::get_default_concurrency,
            commands::start_conversion,
            commands::cancel_conversion,
            commands::export_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
