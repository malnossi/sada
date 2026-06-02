use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn generate_uuid() -> String {
    use std::time::SystemTime;
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("srv_{}", ms)
}

fn default_server_name() -> String {
    "My Server".to_string()
}

fn default_username() -> String {
    "source".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
    #[serde(default)]
    pub selected_server_id: String,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub recording: RecordingConfig,
    #[serde(default)]
    pub metadata: MetadataConfig,
}

impl AppConfig {
    pub fn selected_server(&self) -> &ServerConfig {
        self.servers
            .iter()
            .find(|s| s.id == self.selected_server_id)
            .unwrap_or_else(|| {
                self.servers.first().expect("Config must have at least one server")
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "generate_uuid")]
    pub id: String,
    #[serde(default = "default_server_name")]
    pub name: String,
    pub server_type: ServerType,
    pub host: String,
    pub port: u16,
    #[serde(rename = "mount_point")]
    pub mount: String,
    #[serde(rename = "password")]
    pub source_password: String,
    #[serde(default)]
    pub admin_password: String,
    #[serde(default = "default_username")]
    pub username: String,
    #[serde(default)]
    pub legacy_icecast: bool,
    #[serde(default)]
    pub custom_listener_url: String,
    #[serde(default)]
    pub custom_listener_mount: String,
    pub stream_name: String,
    pub stream_description: String,
    pub stream_genre: String,
    pub stream_url: String,
    #[serde(rename = "public_server")]
    pub public: bool,
    #[serde(default)]
    pub tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerType {
    Icecast,
    Shoutcast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub device_name: Option<String>,
    pub sample_rate: u32,
    pub channels: u16,
    pub codec: Codec,
    #[serde(rename = "bitrate")]
    pub bitrate_kbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    #[serde(rename = "mp3")]
    MP3,
    Opus,
    #[serde(rename = "ogg_vorbis")]
    OggVorbis,
    #[serde(rename = "aac")]
    AAC,
    #[serde(rename = "aac_plus")]
    AACPlus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub enabled: bool,
    #[serde(rename = "output_path")]
    pub output_dir: String,
    pub format: RecordingFormat,
    #[serde(default)]
    pub split_hours: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingFormat {
    Wav,
    Mp3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    pub update_from_file: bool,
    pub file_path: String,
    pub poll_interval_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_server = ServerConfig::default();
        let selected_id = default_server.id.clone();
        Self {
            servers: vec![default_server],
            selected_server_id: selected_id,
            audio: AudioConfig::default(),
            recording: RecordingConfig::default(),
            metadata: MetadataConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            name: default_server_name(),
            server_type: ServerType::Icecast,
            host: "localhost".into(),
            port: 8000,
            mount: "/stream".into(),
            source_password: "hackme".into(),
            admin_password: "admin".into(),
            username: "source".into(),
            legacy_icecast: false,
            custom_listener_url: String::new(),
            custom_listener_mount: String::new(),
            stream_name: "My Stream".into(),
            stream_description: String::new(),
            stream_genre: String::new(),
            stream_url: String::new(),
            public: false,
            tls: false,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            device_name: None,
            sample_rate: 44100,
            channels: 2,
            codec: Codec::MP3,
            bitrate_kbps: 128,
        }
    }
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            output_dir: String::new(),
            format: RecordingFormat::Wav,
            split_hours: None,
        }
    }
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            update_from_file: false,
            file_path: String::new(),
            poll_interval_secs: 5,
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sada")
        .join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        // First try to deserialize the new format
        if let Ok(mut config) = serde_json::from_str::<AppConfig>(&data) {
            // Ensure there is at least one server
            if config.servers.is_empty() {
                config.servers.push(ServerConfig::default());
            }
            if config.selected_server_id.is_empty() {
                config.selected_server_id = config.servers[0].id.clone();
            }
            return config;
        }
        
        // Fallback: try to deserialize the old format
        #[derive(Deserialize)]
        struct OldAppConfig {
            server: Option<ServerConfig>,
            audio: Option<AudioConfig>,
            recording: Option<RecordingConfig>,
            metadata: Option<MetadataConfig>,
        }
        
        if let Ok(old) = serde_json::from_str::<OldAppConfig>(&data) {
            let mut config = AppConfig::default();
            if let Some(old_server) = old.server {
                let mut server = old_server;
                if server.name.is_empty() || server.name == "My Server" {
                    server.name = "Default Server".to_string();
                }
                config.selected_server_id = server.id.clone();
                config.servers = vec![server];
            }
            if let Some(audio) = old.audio {
                config.audio = audio;
            }
            if let Some(rec) = old.recording {
                config.recording = rec;
            }
            if let Some(meta) = old.metadata {
                config.metadata = meta;
            }
            return config;
        }
        
        AppConfig::default()
    } else {
        AppConfig::default()
    }
}

pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    let path = config_path();
    std::fs::create_dir_all(
        path.parent()
            .expect("Config path must have a parent directory"),
    )?;
    std::fs::write(&path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}
