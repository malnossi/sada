pub mod icecast;
pub mod metadata;
pub mod shoutcast;

use crate::audio;
use crate::config::{AppConfig, ServerConfig, ServerType};
use crate::state::{AppState, ConnectionHandle, ConnectionStatus};
use std::sync::Arc;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

/// Full startup sequence (STREAMING.md section 7):
///
/// 1. Lock AppState, clone config, set status = Connecting, release lock
/// 2. Get audio device from config (or default)
/// 3. Get input stream config from cpal
/// 4. Create ring buffer (RING_BUFFER_SIZE)
/// 5. Create VU mpsc channel (bounded 10)
/// 6. Create encoder based on config.audio.codec
/// 7. Create network mpsc channel (bounded 256)
/// 8. Create shutdown channels (one per task)
/// 9. Start VU emitter thread (std::thread::spawn)
/// 10. Start audio capture (cpal) on DEDICATED std::thread
/// 11. Start encoder tokio task → reads HeapCons, sends to network_tx
/// 12. Start network tokio task → connects to Icecast, streams, reconnects
/// 13. Store ConnectionHandle (shutdown_tx + metadata_tx + audio_shutdown) in AppState
pub async fn start(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, Arc<std::sync::Mutex<AppState>>>,
    config: AppConfig,
) -> anyhow::Result<()> {
    // Channels for shutdown and metadata
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
    let (metadata_tx, mut metadata_rx) = mpsc::channel::<String>(16);
    let (network_tx, network_rx) = mpsc::channel::<bytes::Bytes>(256);

    // Encoder shutdown channel (separate from network shutdown)
    let (encoder_shutdown_tx, encoder_shutdown_rx) = std::sync::mpsc::channel::<()>();

    // --- Step 4: Create ring buffer ---
    let (producer, consumer) = audio::create_ring_buffer();

    // --- Step 5: Create VU channel (bounded 10) ---
    let (vu_tx, vu_rx) = std::sync::mpsc::sync_channel::<audio::VuLevel>(10);

    // --- Step 6: Create encoder ---
    let encoder = audio::create_encoder(
        &config.audio.codec,
        config.audio.sample_rate,
        config.audio.channels,
        config.audio.bitrate_kbps,
    )?;

    // Get content type from encoder before moving it
    let content_type = encoder.content_type().to_string();

    // --- Step 9: Start VU emitter thread ---
    audio::start_vu_emitter(app.clone(), vu_rx);

    // --- Step 9.5: Start metadata HTTP worker task ---
    let cfg_meta = config.selected_server().clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        while let Some(song) = metadata_rx.recv().await {
            if let Err(e) = crate::streaming::metadata::update_metadata_http(&client, &cfg_meta, &song).await {
                log::warn!("Metadata update failed: {e}");
            }
        }
    });

    // --- Step 11: Start encoder dedicated thread ---
    let encoder_thread = std::thread::spawn(move || {
        audio::run_encoder_thread(
            consumer,
            encoder,
            network_tx,
            encoder_shutdown_rx,
        );
    });
    let encoder_thread_handle = encoder_thread.thread().clone();

    // --- Step 10: Start audio capture on DEDICATED std::thread ---
    // cpal::Stream is !Send — it MUST stay on the thread that created it.
    let (audio_shutdown_tx, audio_shutdown_rx) = std::sync::mpsc::channel::<()>();
    let audio_device_name = config.audio.device_name.clone();
    let audio_sample_rate = config.audio.sample_rate;
    let audio_channels = config.audio.channels;
    let app_capture_err = app.clone();

    std::thread::spawn(move || {
        let device = match audio::capture::get_default_or_named(audio_device_name.as_deref()) {
            Ok(d) => d,
            Err(e) => {
                let err_msg = format!("Failed to get audio device: {e}");
                log::error!("{err_msg}");
                let _ = app_capture_err.emit(
                    "app-error",
                    serde_json::json!({
                        "message": err_msg,
                        "level": "error"
                    }),
                );
                return;
            }
        };

        let _audio_stream = match audio::capture::start_capture(
            &device,
            audio_sample_rate,
            audio_channels,
            producer,
            vu_tx,
            app_capture_err.clone(),
            encoder_thread_handle,
        ) {
            Ok(s) => s,
            Err(e) => {
                let err_msg = format!("Failed to start audio capture: {e}");
                log::error!("{err_msg}");
                let _ = app_capture_err.emit(
                    "app-error",
                    serde_json::json!({
                        "message": err_msg,
                        "level": "error"
                    }),
                );
                return;
            }
        };

        // Block this thread until shutdown signal received.
        // The cpal stream stays alive as long as _audio_stream is in scope.
        let _ = audio_shutdown_rx.recv();
        // _audio_stream drops here, stopping capture
    });

    // --- Step 11.5: Spawn metadata file watcher task if enabled ---
    let mut watcher_shutdown_tx = None;
    if config.metadata.update_from_file {
        let (w_shutdown_tx, w_shutdown_rx) = mpsc::channel::<()>(1);
        watcher_shutdown_tx = Some(w_shutdown_tx);

        let path = config.metadata.file_path.clone();
        let m_tx = metadata_tx.clone();
        let interval = config.metadata.poll_interval_secs;

        tokio::spawn(crate::streaming::metadata::watch_metadata_file(
            path,
            m_tx,
            w_shutdown_rx,
            interval,
            app.clone(),
        ));
    }

    // --- Step 12: Start network tokio task ---
    let app_clone = app.clone();
    let cfg_clone = config.selected_server().clone();
    let state_arc = state.inner().clone();

    // --- Step 13: Store ConnectionHandle ---
    {
        let mut s = state.lock().unwrap();
        s.connection = Some(ConnectionHandle {
            shutdown_tx: shutdown_tx.clone(),
            metadata_tx,
            encoder_shutdown: encoder_shutdown_tx,
            audio_shutdown: audio_shutdown_tx,
            watcher_shutdown: watcher_shutdown_tx,
        });
        s.status = ConnectionStatus::Connecting;
        s.start_time = None;
        s.bytes_sent = 0;
    }
    emit_status(app, ConnectionStatus::Connecting);

    let bitrate_kbps = config.audio.bitrate_kbps;
    tokio::spawn(async move {
        run_network_loop(
            &app_clone,
            cfg_clone,
            &content_type,
            network_rx,
            shutdown_rx,
            state_arc.clone(),
            bitrate_kbps,
        )
        .await;

        // Cleanup on exit
        let mut s = state_arc.lock().unwrap();
        // Send audio and watcher shutdown signals
        if let Some(ref conn) = s.connection {
            let _ = conn.audio_shutdown.send(());
            if let Some(ref w_tx) = conn.watcher_shutdown {
                let _ = w_tx.try_send(());
            }
        }
        s.status = ConnectionStatus::Idle;
        s.connection = None;
        s.start_time = None;
    });

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_network_loop(
    app: &tauri::AppHandle,
    cfg: ServerConfig,
    content_type: &str,
    mut network_rx: mpsc::Receiver<bytes::Bytes>,
    mut shutdown_rx: mpsc::Receiver<()>,
    state: Arc<std::sync::Mutex<AppState>>,
    bitrate_kbps: u32,
) {
    let mut attempt = 0u32;

    'reconnect: loop {
        // Exponential backoff: 0s, 1s, 2s, 4s, 8s, 16s, 32s, max 60s
        if attempt > 0 {
            let delay = std::cmp::min(2u64.pow(attempt - 1), 60);
            {
                let mut s = state.lock().unwrap();
                s.status = ConnectionStatus::Reconnecting {
                    attempt,
                    delay_secs: delay,
                };
            }
            emit_status(
                app,
                ConnectionStatus::Reconnecting {
                    attempt,
                    delay_secs: delay,
                },
            );
            log::info!("Reconnecting in {delay}s (attempt {attempt})...");

            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(delay)) => {}
                _ = shutdown_rx.recv() => { break 'reconnect; }
            }
        }

        // Connect
        let connect_result = match cfg.server_type {
            ServerType::Icecast => icecast::connect_icecast(&cfg, content_type, bitrate_kbps).await,
            ServerType::Shoutcast => {
                shoutcast::connect_shoutcast(&cfg, content_type, bitrate_kbps).await
            }
        };

        let stream = match connect_result {
            Ok(s) => s,
            Err(e) => {
                log::error!("Connection failed: {e}");
                let response_str = e.to_string();
                let status_err = ConnectionStatus::Error {
                    message: response_str.clone(),
                };
                {
                    let mut s = state.lock().unwrap();
                    s.status = status_err.clone();
                }
                // Don't reconnect on auth failures
                if !should_reconnect(&response_str) {
                    emit_status(app, status_err);
                    break 'reconnect;
                }
                attempt += 1;
                emit_status(app, status_err);
                continue 'reconnect;
            }
        };

        // Connected! Wrap in BufWriter for efficient TCP writes.
        attempt = 0;
        let start = std::time::Instant::now();
        {
            let mut s = state.lock().unwrap();
            s.status = ConnectionStatus::Connected;
            s.start_time = Some(start);
        }
        emit_status(app, ConnectionStatus::Connected);
        log::info!(
            "Connected to Icecast/Shoutcast at {}:{}{}",
            cfg.host,
            cfg.port,
            cfg.mount
        );

        let mut stream = tokio::io::BufWriter::with_capacity(8192, stream);
        let mut bytes_sent = 0u64;
        let mut stats_tick = tokio::time::interval(tokio::time::Duration::from_secs(1));

        // Main streaming loop
        loop {
            tokio::select! {
                biased;

                // Shutdown signal
                _ = shutdown_rx.recv() => {
                    log::info!("Disconnecting (shutdown signal)");
                    break 'reconnect;
                }

                // Encoded audio packet ready
                packet = network_rx.recv() => {
                    match packet {
                        Some(packet) => {
                            match stream.write_all(&packet).await {
                                Ok(()) => {
                                    bytes_sent += packet.len() as u64;
                                    // Flush when channel is drained
                                    if network_rx.is_empty() {
                                        if let Err(e) = stream.flush().await {
                                            log::error!("Flush error — reconnecting: {e}");
                                            attempt += 1;
                                            break; // inner loop → reconnect
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Write error — reconnecting: {e}");
                                    attempt += 1;
                                    break; // inner loop → reconnect
                                }
                            }
                        }
                        None => {
                            log::info!("Encoder channel closed, exiting network loop");
                            break 'reconnect;
                        }
                    }
                }

                // Stats update every second
                _ = stats_tick.tick() => {
                    let duration = start.elapsed().as_secs();
                    let kbps = if duration > 0 {
                        (bytes_sent * 8) as f64 / duration as f64 / 1000.0
                    } else {
                        0.0
                    };
                    let _ = app.emit("stream-stats", serde_json::json!({
                        "duration_secs": duration,
                        "bytes_sent": bytes_sent,
                        "kbps": kbps,
                    }));
                }
            }
        }
    }

    emit_status(app, ConnectionStatus::Idle);
}

/// Don't reconnect on auth failures — user needs to fix config
fn should_reconnect(response: &str) -> bool {
    if response.contains("401") || response.contains("403") {
        return false;
    }
    true
}

fn emit_status(app: &tauri::AppHandle, status: ConnectionStatus) {
    let _ = app.emit("connection-status", &status);
}

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

pub enum Stream {
    Plain(TcpStream),
    Tls(tokio_native_tls::TlsStream<TcpStream>),
}

impl AsyncRead for Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Stream::Plain(s) => Pin::new(s).poll_read(cx, buf),
            Stream::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Stream::Plain(s) => Pin::new(s).poll_write(cx, buf),
            Stream::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Stream::Plain(s) => Pin::new(s).poll_flush(cx),
            Stream::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Stream::Plain(s) => Pin::new(s).poll_shutdown(cx),
            Stream::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}
