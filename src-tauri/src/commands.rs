use crate::audio::capture::list_input_devices;
use crate::config::{save_config, AppConfig};
use crate::state::AppState;
use crate::streaming;
use std::sync::{Arc, Mutex};
use tauri::State;

pub fn start_monitor_internal(
    app: tauri::AppHandle,
    state: &Arc<Mutex<AppState>>,
    device_name: Option<String>,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();

    // Stop any existing monitor stream
    if let Some(h) = s.monitor.take() {
        let _ = h.audio_shutdown.send(());
    }

    // Do not start monitor if connected or recording
    if s.is_connected() || s.recording.is_some() {
        return Ok(());
    }

    let device = crate::audio::capture::get_default_or_named(device_name.as_deref())
        .map_err(|e| e.to_string())?;

    // Create sync channel for VU levels
    let (vu_tx, vu_rx) = std::sync::mpsc::sync_channel::<crate::audio::VuLevel>(10);

    // Start VU emitter thread (exits when vu_tx drops)
    crate::audio::start_vu_emitter(app.clone(), vu_rx);

    let (audio_shutdown_tx, audio_shutdown_rx) = std::sync::mpsc::channel::<()>();
    let app_clone = app.clone();

    std::thread::spawn(move || {
        let _stream = match crate::audio::capture::start_monitor_capture(&device, vu_tx, app_clone)
        {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to start monitor capture: {e}");
                return;
            }
        };
        // Wait for shutdown signal
        let _ = audio_shutdown_rx.recv();
        // _stream is dropped here, stopping cpal capture
    });

    s.monitor = Some(crate::state::MonitorHandle {
        audio_shutdown: audio_shutdown_tx,
    });

    Ok(())
}

pub fn stop_monitor_internal(state: &Arc<Mutex<AppState>>) {
    let mut s = state.lock().unwrap();
    if let Some(h) = s.monitor.take() {
        let _ = h.audio_shutdown.send(());
    }
}

#[tauri::command]
pub async fn start_monitor(
    device_name: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    start_monitor_internal(app, &state, device_name)
}

#[tauri::command]
pub async fn stop_monitor(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    stop_monitor_internal(&state);
    Ok(())
}

#[tauri::command]
pub async fn get_config(state: State<'_, Arc<Mutex<AppState>>>) -> Result<AppConfig, String> {
    Ok(state.lock().unwrap().config.clone())
}

#[tauri::command]
pub async fn save_config_cmd(
    config: AppConfig,
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    save_config(&config).map_err(|e| e.to_string())?;

    // Update config in state
    {
        let mut s = state.lock().unwrap();
        s.config = config.clone();
    }

    // Automatically restart monitor on saved device (if idle)
    let _ = start_monitor_internal(app, &state, config.audio.device_name);
    Ok(())
}

#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<String>, String> {
    list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect(
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let config = {
        let s = state.lock().unwrap();
        if s.is_connected() {
            return Err("Already connected".into());
        }
        s.config.clone()
    };

    stop_monitor_internal(&state);

    match streaming::start(&app, &state, config.clone()).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = start_monitor_internal(app.clone(), &state, config.audio.device_name.clone());
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn disconnect(
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let handle = state.lock().unwrap().connection.take();
    if let Some(h) = handle {
        // Signal audio thread to stop
        let _ = h.audio_shutdown.send(());
        // Signal network/encoder tasks to stop
        let _ = h.shutdown_tx.send(()).await;
        let _ = h.encoder_shutdown.send(());
        if let Some(w_tx) = h.watcher_shutdown {
            let _ = w_tx.send(()).await;
        }
    }

    // Resume monitoring
    let device_name = {
        let s = state.lock().unwrap();
        s.config.audio.device_name.clone()
    };
    let _ = start_monitor_internal(app, &state, device_name);
    Ok(())
}

#[tauri::command]
pub async fn update_metadata(
    song: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let tx = {
        let s = state.lock().unwrap();
        s.connection.as_ref().map(|c| c.metadata_tx.clone())
    };
    if let Some(tx) = tx {
        tx.send(song).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_connection_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<crate::state::ConnectionStatus, String> {
    Ok(state.lock().unwrap().status.clone())
}

#[tauri::command]
pub async fn start_recording(
    path: String,
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let audio_config = {
        let s = state.lock().unwrap();
        if s.recording.is_some() {
            return Err("Recording is already active".into());
        }
        s.config.audio.clone()
    };

    stop_monitor_internal(&state);

    match crate::recording::start_recording_session(&path, &audio_config) {
        Ok(session) => {
            let mut s = state.lock().unwrap();
            s.recording = Some(crate::state::RecordingHandle {
                shutdown_tx: session.shutdown_tx,
                audio_shutdown: session.audio_shutdown_tx,
            });
            Ok(())
        }
        Err(e) => {
            let _ = start_monitor_internal(app, &state, audio_config.device_name.clone());
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn stop_recording(
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let handle = state.lock().unwrap().recording.take();
    if let Some(h) = handle {
        let _ = h.audio_shutdown.send(());
        let _ = h.shutdown_tx.send(());
    }

    // Resume monitoring
    let (device_name, is_connected) = {
        let s = state.lock().unwrap();
        (s.config.audio.device_name.clone(), s.is_connected())
    };
    if !is_connected {
        let _ = start_monitor_internal(app, &state, device_name);
    }
    Ok(())
}
