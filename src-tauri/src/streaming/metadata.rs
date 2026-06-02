use base64::Engine as _;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

/// Update the "Now Playing" metadata on an Icecast server via the admin HTTP API.
/// This is separate from the source connection — it's a regular HTTP GET request
/// to the admin endpoint, authenticated with the admin password (not source password).
pub async fn update_metadata_http(
    client: &reqwest::Client,
    cfg: &crate::config::ServerConfig,
    song_title: &str,
) -> anyhow::Result<()> {
    let encoded_title = utf8_percent_encode(song_title, NON_ALPHANUMERIC).to_string();
    let mount = if cfg.mount.starts_with('/') {
        cfg.mount.as_str().to_string()
    } else {
        format!("/{}", cfg.mount)
    };

    let (url, credentials) = match cfg.server_type {
        crate::config::ServerType::Icecast => {
            let url = format!(
                "http://{}:{}/admin/metadata?mount={}&mode=updinfo&song={}",
                cfg.host, cfg.port, mount, encoded_title
            );
            let credentials = base64::engine::general_purpose::STANDARD
                .encode(format!("admin:{}", cfg.admin_password));
            (url, credentials)
        }
        crate::config::ServerType::Shoutcast => {
            // Shoutcast 1 & 2 admin.cgi metadata update format
            let sid = if mount.starts_with('/') && mount.len() > 1 && mount[1..].chars().all(|c| c.is_ascii_digit()) {
                mount[1..].to_string()
            } else if mount.chars().all(|c| c.is_ascii_digit()) {
                mount.clone()
            } else {
                "1".to_string()
            };
            let url = format!(
                "http://{}:{}/admin.cgi?sid={}&mode=updinfo&pass={}&song={}",
                cfg.host, cfg.port, sid, cfg.source_password, encoded_title
            );
            // Support Shoutcast 2 Basic Auth (admin:password) with fallback to source_password
            let shoutcast_password = if cfg.admin_password.is_empty() {
                &cfg.source_password
            } else {
                &cfg.admin_password
            };
            let credentials = base64::engine::general_purpose::STANDARD
                .encode(format!("admin:{}", shoutcast_password));
            (url, credentials)
        }
    };

    let mut request = client
        .get(&url)
        .header("Authorization", format!("Basic {credentials}"))
        .timeout(std::time::Duration::from_secs(5));

    // Some Shoutcast server firewalls block reqwest requests with empty/default User-Agents
    if cfg.server_type == crate::config::ServerType::Shoutcast {
        request = request.header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Metadata update HTTP {}: {}", response.status(), url);
    }
    Ok(())
}

use tauri::Emitter;

/// Auto-metadata from file: polls a file path every N seconds.
/// When the content changes, sends the new metadata via the provided channel.
/// Emits 'metadata-changed' event to the frontend Svelte application.
pub async fn watch_metadata_file(
    path: String,
    tx: tokio::sync::mpsc::Sender<String>,
    mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    interval_secs: u64,
    app: tauri::AppHandle,
) {
    let mut last_content = String::new();
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let trimmed = content.trim().to_string();
                    if trimmed != last_content && !trimmed.is_empty() {
                        last_content = trimmed.clone();
                        let _ = tx.send(trimmed.clone()).await;
                        let _ = app.emit("metadata-changed", serde_json::json!({
                            "current": trimmed
                        }));
                    }
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }
}
