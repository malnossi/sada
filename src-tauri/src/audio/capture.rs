use crate::audio::vu_meter::compute_rms;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample};
use ringbuf::traits::Producer;
use ringbuf::HeapProd;
use std::sync::mpsc;
use tauri::Emitter;

/// VU meter value sent from audio thread to UI emitter
#[derive(Clone, Copy)]
pub struct VuLevel {
    pub left: f32,
    pub right: f32,
}

/// Holds the cpal stream alive — dropping this stops capture
pub struct AudioStream {
    _stream: cpal::Stream,
    pub actual_rate: u32,
    pub actual_channels: u16,
}

/// List all available input device names
pub fn list_input_devices() -> anyhow::Result<Vec<String>> {
    let host = cpal::default_host();
    let devices = host.input_devices()?;
    let names: Vec<String> = devices.filter_map(|d| d.name().ok()).collect();
    Ok(names)
}

/// Get a specific device by name
pub fn get_device_by_name(name: &str) -> anyhow::Result<cpal::Device> {
    let host = cpal::default_host();
    host.input_devices()?
        .find(|d| d.name().map(|n| n == name).unwrap_or(false))
        .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", name))
}

/// Get default device, or a named one if specified
pub fn get_default_or_named(name: Option<&str>) -> anyhow::Result<cpal::Device> {
    match name {
        Some(n) if !n.is_empty() => get_device_by_name(n),
        _ => cpal::default_host()
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device")),
    }
}

/// Get the best supported config for our target sample rate.
/// cpal may not support exactly 44100 or 48000; we request and fall back.
pub fn get_input_config(
    device: &cpal::Device,
    target_rate: u32,
    channels: u16,
) -> anyhow::Result<cpal::SupportedStreamConfig> {
    let supported = device.supported_input_configs()?;
    for config in supported {
        if config.channels() == channels
            && config.min_sample_rate().0 <= target_rate
            && config.max_sample_rate().0 >= target_rate
        {
            return Ok(config.with_sample_rate(cpal::SampleRate(target_rate)));
        }
    }
    // Fall back to device default
    Ok(device.default_input_config()?)
}

/// Start capturing audio from the given device.
/// Producer end of ring buffer goes to encoder thread.
/// VU sender goes to VU emitter thread.
pub fn start_capture(
    device: &cpal::Device,
    target_rate: u32,
    target_channels: u16,
    producer: HeapProd<f32>,
    vu_tx: mpsc::SyncSender<VuLevel>,
    app: tauri::AppHandle,
    encoder_thread: std::thread::Thread,
) -> anyhow::Result<AudioStream> {
    let config = get_input_config(device, target_rate, target_channels)?;
    let actual_rate = config.sample_rate().0;
    let actual_channels = config.channels();
    let sample_format = config.sample_format();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => build_stream::<f32>(
            device,
            &config.into(),
            producer,
            vu_tx,
            target_channels,
            target_rate,
            app,
            encoder_thread,
        )?,
        cpal::SampleFormat::I16 => build_stream::<i16>(
            device,
            &config.into(),
            producer,
            vu_tx,
            target_channels,
            target_rate,
            app,
            encoder_thread,
        )?,
        cpal::SampleFormat::U16 => build_stream::<u16>(
            device,
            &config.into(),
            producer,
            vu_tx,
            target_channels,
            target_rate,
            app,
            encoder_thread,
        )?,
        cpal::SampleFormat::U8 => build_stream::<u8>(
            device,
            &config.into(),
            producer,
            vu_tx,
            target_channels,
            target_rate,
            app,
            encoder_thread,
        )?,
        cpal::SampleFormat::I8 => build_stream::<i8>(
            device,
            &config.into(),
            producer,
            vu_tx,
            target_channels,
            target_rate,
            app,
            encoder_thread,
        )?,
        cpal::SampleFormat::I32 => build_stream::<i32>(
            device,
            &config.into(),
            producer,
            vu_tx,
            target_channels,
            target_rate,
            app,
            encoder_thread,
        )?,
        cpal::SampleFormat::U32 => build_stream::<u32>(
            device,
            &config.into(),
            producer,
            vu_tx,
            target_channels,
            target_rate,
            app,
            encoder_thread,
        )?,
        cpal::SampleFormat::F64 => build_stream::<f64>(
            device,
            &config.into(),
            producer,
            vu_tx,
            target_channels,
            target_rate,
            app,
            encoder_thread,
        )?,
        _ => anyhow::bail!("Unsupported sample format: {:?}", sample_format),
    };

    stream.play()?;
    Ok(AudioStream {
        _stream: stream,
        actual_rate,
        actual_channels,
    })
}

fn build_stream<T: cpal::Sample + cpal::SizedSample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: HeapProd<f32>,
    vu_tx: mpsc::SyncSender<VuLevel>,
    target_channels: u16,
    target_rate: u32,
    app: tauri::AppHandle,
    encoder_thread: std::thread::Thread,
) -> anyhow::Result<cpal::Stream>
where
    f32: FromSample<T>,
{
    let actual_channels = config.channels as usize;
    let actual_rate = config.sample_rate.0;
    let target_channels = target_channels as usize;
    let app_err = app.clone();

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _info: &cpal::InputCallbackInfo| {
            // 1. Convert to normalized f32
            let samples: Vec<f32> = data.iter().map(|s| f32::from_sample(*s)).collect();

            // 2. Channel conversion (actual_channels -> target_channels)
            let channel_converted = convert_channels(samples, actual_channels, target_channels);

            // 3. Sample rate resampling (actual_rate -> target_rate)
            let resampled = resample_samples(channel_converted, actual_rate, target_rate, target_channels);

            // 4. Push into ring buffer
            let _written = producer.push_slice(&resampled);
            encoder_thread.unpark();

            // 5. Compute VU — one RMS per channel using target channels
            let (left, right) = if target_channels >= 2 {
                let l: Vec<f32> = resampled.iter().step_by(2).copied().collect();
                let r: Vec<f32> = resampled.iter().skip(1).step_by(2).copied().collect();
                (compute_rms(&l), compute_rms(&r))
            } else {
                let m = compute_rms(&resampled);
                (m, m)
            };

            // try_send to VU emitter
            let _ = vu_tx.try_send(VuLevel { left, right });
        },
        move |err| {
            let err_msg = format!("cpal audio capture stream error: {err}");
            log::error!("{err_msg}");
            let _ = app_err.emit(
                "app-error",
                serde_json::json!({
                    "message": err_msg,
                    "level": "error"
                }),
            );
        },
        None,
    )?;
    Ok(stream)
}

/// Silent capture stream specifically for real-time VU monitoring.
/// Does not push to ring buffers or encode.
pub fn start_monitor_capture(
    device: &cpal::Device,
    vu_tx: std::sync::mpsc::SyncSender<VuLevel>,
    app: tauri::AppHandle,
) -> anyhow::Result<cpal::Stream> {
    let config = device.default_input_config()?;
    let actual_channels = config.channels() as usize;
    let sample_format = config.sample_format();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            build_monitor_stream::<f32>(device, &config.into(), vu_tx, actual_channels, app)?
        }
        cpal::SampleFormat::I16 => {
            build_monitor_stream::<i16>(device, &config.into(), vu_tx, actual_channels, app)?
        }
        cpal::SampleFormat::U16 => {
            build_monitor_stream::<u16>(device, &config.into(), vu_tx, actual_channels, app)?
        }
        cpal::SampleFormat::U8 => {
            build_monitor_stream::<u8>(device, &config.into(), vu_tx, actual_channels, app)?
        }
        cpal::SampleFormat::I8 => {
            build_monitor_stream::<i8>(device, &config.into(), vu_tx, actual_channels, app)?
        }
        cpal::SampleFormat::I32 => {
            build_monitor_stream::<i32>(device, &config.into(), vu_tx, actual_channels, app)?
        }
        cpal::SampleFormat::U32 => {
            build_monitor_stream::<u32>(device, &config.into(), vu_tx, actual_channels, app)?
        }
        cpal::SampleFormat::F64 => {
            build_monitor_stream::<f64>(device, &config.into(), vu_tx, actual_channels, app)?
        }
        _ => anyhow::bail!("Unsupported sample format: {:?}", sample_format),
    };

    stream.play()?;
    Ok(stream)
}

fn build_monitor_stream<T: cpal::Sample + cpal::SizedSample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    vu_tx: std::sync::mpsc::SyncSender<VuLevel>,
    actual_channels: usize,
    _app: tauri::AppHandle,
) -> anyhow::Result<cpal::Stream>
where
    f32: FromSample<T>,
{
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _info: &cpal::InputCallbackInfo| {
            let samples: Vec<f32> = data.iter().map(|s| f32::from_sample(*s)).collect();
            let (left, right) = if actual_channels >= 2 {
                let l: Vec<f32> = samples.iter().step_by(actual_channels).copied().collect();
                let r: Vec<f32> = samples
                    .iter()
                    .skip(1)
                    .step_by(actual_channels)
                    .copied()
                    .collect();
                (compute_rms(&l), compute_rms(&r))
            } else {
                let m = compute_rms(&samples);
                (m, m)
            };
            let _ = vu_tx.try_send(VuLevel { left, right });
        },
        move |err| {
            log::error!("cpal monitor stream error: {err}");
        },
        None,
    )?;
    Ok(stream)
}

pub fn convert_channels(
    samples: Vec<f32>,
    actual_channels: usize,
    target_channels: usize,
) -> Vec<f32> {
    if actual_channels == 0 || target_channels == 0 {
        return samples;
    }
    if actual_channels == target_channels {
        samples
    } else if actual_channels == 2 && target_channels == 1 {
        // Stereo to Mono
        let mut mono = Vec::with_capacity(samples.len() / 2);
        for chunk in samples.chunks_exact(2) {
            mono.push((chunk[0] + chunk[1]) * 0.5);
        }
        mono
    } else if actual_channels == 1 && target_channels == 2 {
        // Mono to Stereo
        let mut stereo = Vec::with_capacity(samples.len() * 2);
        for &s in &samples {
            stereo.push(s);
            stereo.push(s);
        }
        stereo
    } else {
        // Multi-channel fallback
        let mut result = Vec::with_capacity((samples.len() / actual_channels) * target_channels);
        for chunk in samples.chunks(actual_channels) {
            if target_channels == 1 {
                let avg: f32 = if chunk.is_empty() { 0.0 } else { chunk.iter().sum::<f32>() / chunk.len() as f32 };
                result.push(avg);
            } else if target_channels == 2 {
                let avg: f32 = if chunk.is_empty() { 0.0 } else { chunk.iter().sum::<f32>() / chunk.len() as f32 };
                result.push(avg);
                result.push(avg);
            }
        }
        result
    }
}

pub fn resample_samples(
    channel_converted: Vec<f32>,
    actual_rate: u32,
    target_rate: u32,
    target_channels: usize,
) -> Vec<f32> {
    if actual_rate == 0 || target_rate == 0 || target_channels == 0 {
        return channel_converted;
    }
    if actual_rate == target_rate {
        channel_converted
    } else {
        let ratio = actual_rate as f32 / target_rate as f32;
        let num_input_frames = channel_converted.len() / target_channels;
        if num_input_frames == 0 {
            Vec::new()
        } else {
            let num_output_frames = (num_input_frames as f32 / ratio).round() as usize;
            let mut output = Vec::with_capacity(num_output_frames * target_channels);

            for i in 0..num_output_frames {
                let input_pos = i as f32 * ratio;
                let index = std::cmp::min(input_pos.floor() as usize, num_input_frames - 1);
                let next_index = std::cmp::min(index + 1, num_input_frames - 1);
                let frac = input_pos - index as f32;

                for ch in 0..target_channels {
                    let s0 = channel_converted[index * target_channels + ch];
                    let s1 = channel_converted[next_index * target_channels + ch];
                    let interpolated = s0 + (s1 - s0) * frac;
                    output.push(interpolated);
                }
            }
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_zero_input() {
        let res = resample_samples(vec![], 48000, 44100, 2);
        assert!(res.is_empty());
    }

    #[test]
    fn test_resampler_normal() {
        let input = vec![0.0, 0.0, 1.0, 1.0, 0.5, 0.5, 0.0, 0.0]; // 4 frames of stereo
        let res = resample_samples(input.clone(), 44100, 44100, 2);
        assert_eq!(res, input);

        let res_down = resample_samples(input, 48000, 24000, 2);
        assert_eq!(res_down.len(), 4); // 2 frames
    }

    #[test]
    fn test_channel_conversion() {
        let stereo = vec![1.0, 0.0, 0.5, 0.5];
        let mono = convert_channels(stereo.clone(), 2, 1);
        assert_eq!(mono, vec![0.5, 0.5]);

        let mono_in = vec![1.0, 0.5];
        let stereo_out = convert_channels(mono_in, 1, 2);
        assert_eq!(stereo_out, vec![1.0, 1.0, 0.5, 0.5]);
    }
}
