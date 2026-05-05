mod db;
mod tracking;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_settings(app_handle: tauri::AppHandle) -> Result<db::Settings, String> {
    db::get_settings(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_settings(app_handle: tauri::AppHandle, settings: db::Settings) -> Result<(), String> {
    db::update_settings(&app_handle, settings).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database
            db::init_db(app.handle()).expect("Failed to initialize database");

            // Start background tracking
            tracking::start_tracking(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
