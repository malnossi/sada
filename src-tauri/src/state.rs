use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Tauri manages this as: .manage(Arc::new(Mutex::new(AppState::new())))
/// Access in commands: state: tauri::State<'_, Arc<Mutex<AppState>>>
pub struct AppState {
    pub config: AppConfig,
    pub connection: Option<ConnectionHandle>,
    pub status: ConnectionStatus,
    pub recording: Option<RecordingHandle>,
    pub monitor: Option<MonitorHandle>,
    pub start_time: Option<std::time::Instant>,
    pub bytes_sent: u64,
}

pub struct ConnectionHandle {
    pub shutdown_tx: mpsc::Sender<()>,
    pub metadata_tx: mpsc::Sender<String>,
    pub encoder_shutdown: std::sync::mpsc::Sender<()>,
    pub watcher_shutdown: Option<mpsc::Sender<()>>,
    /// Sender to signal the dedicated audio capture std::thread to stop.
    /// cpal::Stream is !Send so it must live on the thread that created it.
    pub audio_shutdown: std::sync::mpsc::Sender<()>,
}

pub struct RecordingHandle {
    pub shutdown_tx: std::sync::mpsc::Sender<()>,
    pub audio_shutdown: std::sync::mpsc::Sender<()>,
}

pub struct MonitorHandle {
    pub audio_shutdown: std::sync::mpsc::Sender<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConnectionStatus {
    Idle,
    Connecting,
    Connected,
    Reconnecting { attempt: u32, delay_secs: u64 },
    Error { message: String },
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: crate::config::load_config(),
            connection: None,
            status: ConnectionStatus::Idle,
            recording: None,
            monitor: None,
            start_time: None,
            bytes_sent: 0,
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected)
    }
}

// CRITICAL: Never hold this Mutex lock across an .await point.
// Pattern for async commands:
//   let config = {
//       let state = state.lock().unwrap();
//       state.config.clone()           // clone what you need
//   };                                 // lock drops HERE before any .await
//   some_async_fn(config).await;
