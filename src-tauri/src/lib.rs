pub mod audio;
pub mod commands;
pub mod config;
pub mod recording;
pub mod state;
pub mod streaming;

use state::AppState;
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(AppState::new())))
        .setup(|app| {
            use tauri::Manager;
            let state = app.state::<Arc<Mutex<AppState>>>();
            let device_name = {
                let s = state.lock().unwrap();
                s.config.audio.device_name.clone()
            };
            let _ = commands::start_monitor_internal(app.handle().clone(), &state, device_name);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config_cmd,
            commands::get_audio_devices,
            commands::connect,
            commands::disconnect,
            commands::update_metadata,
            commands::get_connection_status,
            commands::start_recording,
            commands::stop_recording,
            commands::start_monitor,
            commands::stop_monitor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sada");
}
