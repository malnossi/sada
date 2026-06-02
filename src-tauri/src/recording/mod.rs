pub mod writer;

use crate::recording::writer::WavWriter;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::Sample;

pub struct RecordingSession {
    pub shutdown_tx: std::sync::mpsc::Sender<()>,
    pub audio_shutdown_tx: std::sync::mpsc::Sender<()>,
}

/// Start recording session by capturing audio and writing it to a WAV file.
pub fn start_recording_session(
    path_str: &str,
    audio_config: &crate::config::AudioConfig,
) -> anyhow::Result<RecordingSession> {
    let path = std::path::Path::new(path_str);

    // Resolve home directory if path starts with ~
    let resolved_path = if path_str.starts_with('~') {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let stripped = path_str
            .strip_prefix('~')
            .expect("Path verified to start with tilde");
        // Remove leading slash if any
        let relative = stripped.strip_prefix('/').unwrap_or(stripped);
        home.join(relative)
    } else {
        path.to_path_buf()
    };

    // Create parent directories if they don't exist
    if let Some(parent) = resolved_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let device = crate::audio::capture::get_default_or_named(audio_config.device_name.as_deref())?;
    let stream_config = crate::audio::capture::get_input_config(
        &device,
        audio_config.sample_rate,
        audio_config.channels,
    )?;

    let actual_rate = stream_config.sample_rate().0;
    let actual_channels = stream_config.channels();

    let mut writer = WavWriter::new(&resolved_path, actual_rate, actual_channels)?;

    // Channel to send PCM samples to the writer task
    let (pcm_tx, pcm_rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(256);
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();

    // Spawn the dedicated file writing thread
    std::thread::spawn(move || {
        loop {
            match pcm_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(samples) => {
                    if let Err(e) = writer.write_samples(&samples) {
                        log::error!("Failed to write recording samples: {e}");
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if shutdown_rx.try_recv().is_ok() {
                        log::info!("Recording shutdown received");
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    log::info!("Recording channel disconnected");
                    break;
                }
            }
        }
        // Drain any remaining samples
        while let Ok(samples) = pcm_rx.try_recv() {
            let _ = writer.write_samples(&samples);
        }
        // Finalize header with correct sizes
        if let Err(e) = writer.finalize() {
            log::error!("Failed to finalize WAV file: {e}");
        } else {
            log::info!("Recording finalized successfully");
        }
    });

    // Start cpal capture stream in a dedicated thread since cpal::Stream is !Send
    let (audio_shutdown_tx, audio_shutdown_rx) = std::sync::mpsc::channel::<()>();

    std::thread::spawn(move || {
        let sample_format = stream_config.sample_format();

        let build_result = match sample_format {
            cpal::SampleFormat::F32 => {
                build_recording_stream::<f32>(&device, &stream_config.into(), pcm_tx)
            }
            cpal::SampleFormat::I16 => {
                build_recording_stream::<i16>(&device, &stream_config.into(), pcm_tx)
            }
            cpal::SampleFormat::U16 => {
                build_recording_stream::<u16>(&device, &stream_config.into(), pcm_tx)
            }
            cpal::SampleFormat::U8 => {
                build_recording_stream::<u8>(&device, &stream_config.into(), pcm_tx)
            }
            cpal::SampleFormat::I8 => {
                build_recording_stream::<i8>(&device, &stream_config.into(), pcm_tx)
            }
            cpal::SampleFormat::I32 => {
                build_recording_stream::<i32>(&device, &stream_config.into(), pcm_tx)
            }
            cpal::SampleFormat::U32 => {
                build_recording_stream::<u32>(&device, &stream_config.into(), pcm_tx)
            }
            cpal::SampleFormat::F64 => {
                build_recording_stream::<f64>(&device, &stream_config.into(), pcm_tx)
            }
            _ => Err(anyhow::anyhow!("Unsupported sample format")),
        };

        let stream = match build_result {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to build recording stream: {e}");
                return;
            }
        };

        if let Err(e) = stream.play() {
            log::error!("Failed to play recording stream: {e}");
            return;
        }

        // Wait for shutdown
        let _ = audio_shutdown_rx.recv();
        // stream drops here
    });

    Ok(RecordingSession {
        shutdown_tx,
        audio_shutdown_tx,
    })
}

fn build_recording_stream<T: cpal::Sample + cpal::SizedSample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    pcm_tx: std::sync::mpsc::SyncSender<Vec<f32>>,
) -> anyhow::Result<cpal::Stream>
where
    f32: cpal::FromSample<T>,
{
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _info: &cpal::InputCallbackInfo| {
            let samples: Vec<f32> = data.iter().map(|s| f32::from_sample(*s)).collect();
            let _ = pcm_tx.try_send(samples);
        },
        |err| {
            log::error!("cpal recording stream error: {err}");
        },
        None,
    )?;
    Ok(stream)
}
