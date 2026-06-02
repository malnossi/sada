use lame::Lame;
use opus::{Application, Channels, Encoder as LibOpusEncoder};

/// A codec takes raw PCM frames and returns encoded bytes
pub trait AudioEncoder: Send + 'static {
    fn encode_frame(&mut self, pcm: &[f32]) -> anyhow::Result<Vec<u8>>;
    fn content_type(&self) -> &'static str;
    fn flush(&mut self) -> anyhow::Result<Vec<u8>> {
        Ok(vec![])
    }
}

/// Create an encoder based on the configured codec
pub fn create_encoder(
    codec: &crate::config::Codec,
    sample_rate: u32,
    channels: u16,
    bitrate_kbps: u32,
) -> anyhow::Result<Box<dyn AudioEncoder>> {
    use crate::config::Codec;
    match codec {
        Codec::MP3 => Ok(Box::new(Mp3Encoder::new(
            sample_rate,
            channels,
            bitrate_kbps,
        )?)),
        Codec::Opus => Ok(Box::new(OpusEncoder::new(
            sample_rate,
            channels,
            bitrate_kbps,
        )?)),
        Codec::OggVorbis => Ok(Box::new(VorbisEncoderImpl::new(
            sample_rate,
            channels,
            bitrate_kbps,
        )?)),
        Codec::AAC => Ok(Box::new(AacEncoder::new(
            sample_rate,
            channels,
            bitrate_kbps,
            false,
        )?)),
        Codec::AACPlus => Ok(Box::new(AacEncoder::new(
            sample_rate,
            channels,
            bitrate_kbps,
            true,
        )?)),
    }
}

// ---------------------------------------------------------------------------
// Ogg Vorbis Encoder
// ---------------------------------------------------------------------------

pub struct VorbisEncoderImpl {
    encoder: vorbis_encoder::Encoder,
    pcm_buffer: Vec<f32>,
    i16_buffer: Vec<i16>,
    channels: u16,
}

// SAFETY: The underlying libvorbis encoder is only accessed from the encoder thread.
unsafe impl Send for VorbisEncoderImpl {}

impl VorbisEncoderImpl {
    pub fn new(sample_rate: u32, channels: u16, bitrate_kbps: u32) -> anyhow::Result<Self> {
        let quality = ((bitrate_kbps as f32) / 320.0).clamp(-0.1, 1.0);
        let encoder = vorbis_encoder::Encoder::new(channels as u32, sample_rate as u64, quality)
            .map_err(|e| anyhow::anyhow!("Vorbis init error: {:?}", e))?;
        Ok(Self {
            encoder,
            pcm_buffer: Vec::new(),
            i16_buffer: Vec::new(),
            channels,
        })
    }
}

impl AudioEncoder for VorbisEncoderImpl {
    fn encode_frame(&mut self, pcm: &[f32]) -> anyhow::Result<Vec<u8>> {
        self.pcm_buffer.extend_from_slice(pcm);

        let len = self.pcm_buffer.len();
        let rem = len % self.channels as usize;
        let process_len = len - rem;

        if process_len == 0 {
            return Ok(vec![]);
        }

        self.i16_buffer.clear();
        self.i16_buffer.extend(
            self.pcm_buffer.drain(..process_len)
                .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        );

        let bytes = self
            .encoder
            .encode(&self.i16_buffer)
            .map_err(|e| anyhow::anyhow!("Vorbis encode error: {:?}", e))?;
        Ok(bytes)
    }

    fn flush(&mut self) -> anyhow::Result<Vec<u8>> {
        if !self.pcm_buffer.is_empty() {
            let rem = self.pcm_buffer.len() % self.channels as usize;
            if rem > 0 {
                let pad = self.channels as usize - rem;
                self.pcm_buffer.resize(self.pcm_buffer.len() + pad, 0.0);
            }
            self.i16_buffer.clear();
            self.i16_buffer.extend(
                self.pcm_buffer.drain(..)
                    .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            );
            let _ = self.encoder.encode(&self.i16_buffer);
        }
        let bytes = self
            .encoder
            .flush()
            .map_err(|e| anyhow::anyhow!("Vorbis flush error: {:?}", e))?;
        Ok(bytes)
    }

    fn content_type(&self) -> &'static str {
        "audio/ogg"
    }
}


// ---------------------------------------------------------------------------
// MP3 Encoder (LAME)
// ---------------------------------------------------------------------------

pub struct Mp3Encoder {
    lame: Lame,
    channels: u16,
    /// LAME works on fixed-size frames. Buffer PCM until we have enough.
    pcm_buffer: Vec<f32>,
    /// 1152 samples per channel per LAME frame
    frame_size: usize,
}

// SAFETY: Lame contains a raw pointer (*mut c_void) to the lame_global_flags struct.
// The LAME C library is safe to use from a single thread at a time. Since Mp3Encoder
// is only accessed from the encoder thread (never shared), this is safe.
unsafe impl Send for Mp3Encoder {}

impl Mp3Encoder {
    pub fn new(sample_rate: u32, channels: u16, bitrate_kbps: u32) -> anyhow::Result<Self> {
        let mut lame = Lame::new().ok_or_else(|| anyhow::anyhow!("Failed to init LAME"))?;

        lame.set_sample_rate(sample_rate)
            .map_err(|e| anyhow::anyhow!("LAME set_sample_rate: {e:?}"))?;
        lame.set_channels(channels as u8)
            .map_err(|e| anyhow::anyhow!("LAME set_channels: {e:?}"))?;
        lame.set_kilobitrate(bitrate_kbps as i32)
            .map_err(|e| anyhow::anyhow!("LAME set_kilobitrate: {e:?}"))?;
        lame.set_quality(5)
            .map_err(|e| anyhow::anyhow!("LAME set_quality: {e:?}"))?;
        lame.init_params()
            .map_err(|e| anyhow::anyhow!("LAME init_params: {e:?}"))?;

        // LAME processes 1152 PCM samples per channel per frame
        let frame_size = 1152 * channels as usize;

        Ok(Self {
            lame,
            channels,
            pcm_buffer: Vec::new(),
            frame_size,
        })
    }
}

impl AudioEncoder for Mp3Encoder {
    fn encode_frame(&mut self, pcm: &[f32]) -> anyhow::Result<Vec<u8>> {
        self.pcm_buffer.extend_from_slice(pcm);
        let mut output = Vec::new();

        // Process full LAME frames
        while self.pcm_buffer.len() >= self.frame_size {
            let frame: Vec<f32> = self.pcm_buffer.drain(..self.frame_size).collect();

            // Convert f32 to i16 — LAME expects i16 PCM
            let i16_samples: Vec<i16> = frame
                .iter()
                .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect();

            let (left, right): (Vec<i16>, Vec<i16>) = if self.channels == 2 {
                (
                    i16_samples.iter().step_by(2).copied().collect(),
                    i16_samples.iter().skip(1).step_by(2).copied().collect(),
                )
            } else {
                (i16_samples.clone(), i16_samples)
            };

            // Output buffer: LAME recommends 1.25 * samples + 7200 bytes
            let buf_size = (left.len() as f32 * 1.25) as usize + 7200;
            let mut mp3_buf = vec![0u8; buf_size];

            match self.lame.encode(&left, &right, &mut mp3_buf) {
                Ok(n) => output.extend_from_slice(&mp3_buf[..n]),
                Err(e) => anyhow::bail!("LAME encode error: {e:?}"),
            }
        }
        Ok(output)
    }

    fn flush(&mut self) -> anyhow::Result<Vec<u8>> {
        // Encode any remaining PCM in the buffer as a partial frame
        if !self.pcm_buffer.is_empty() {
            // Pad to frame_size with silence
            self.pcm_buffer.resize(self.frame_size, 0.0);
            let frame: Vec<f32> = self.pcm_buffer.drain(..).collect();
            let i16_samples: Vec<i16> = frame
                .iter()
                .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect();
            let (left, right): (Vec<i16>, Vec<i16>) = if self.channels == 2 {
                (
                    i16_samples.iter().step_by(2).copied().collect(),
                    i16_samples.iter().skip(1).step_by(2).copied().collect(),
                )
            } else {
                (i16_samples.clone(), i16_samples)
            };
            let buf_size = (left.len() as f32 * 1.25) as usize + 7200;
            let mut mp3_buf = vec![0u8; buf_size];
            if let Ok(n) = self.lame.encode(&left, &right, &mut mp3_buf) {
                return Ok(mp3_buf[..n].to_vec());
            }
        }
        Ok(vec![])
    }

    fn content_type(&self) -> &'static str {
        "audio/mpeg"
    }
}

// ---------------------------------------------------------------------------
// Opus Encoder
// ---------------------------------------------------------------------------

pub struct OpusEncoder {
    encoder: LibOpusEncoder,
    /// Opus frame size in total samples (all channels).
    /// For 48kHz stereo with 20ms frames: 48000 * 0.020 * 2 = 1920
    frame_samples: usize,
    pcm_buffer: Vec<i16>,
}

impl OpusEncoder {
    pub fn new(sample_rate: u32, channels: u16, bitrate_kbps: u32) -> anyhow::Result<Self> {
        let opus_channels = if channels == 1 {
            Channels::Mono
        } else {
            Channels::Stereo
        };

        let mut encoder = LibOpusEncoder::new(sample_rate, opus_channels, Application::Audio)
            .map_err(|e| anyhow::anyhow!("Opus init: {e}"))?;

        encoder
            .set_bitrate(opus::Bitrate::Bits((bitrate_kbps * 1000) as i32))
            .map_err(|e| anyhow::anyhow!("Opus set_bitrate: {e}"))?;

        // 20ms frame at given sample rate
        let frame_samples = (sample_rate as f32 * 0.020) as usize * channels as usize;

        Ok(Self {
            encoder,
            frame_samples,
            pcm_buffer: Vec::new(),
        })
    }
}

impl AudioEncoder for OpusEncoder {
    fn encode_frame(&mut self, pcm: &[f32]) -> anyhow::Result<Vec<u8>> {
        // Convert f32 to i16
        let i16_input: Vec<i16> = pcm
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();
        self.pcm_buffer.extend_from_slice(&i16_input);

        let mut output = Vec::new();

        while self.pcm_buffer.len() >= self.frame_samples {
            let frame: Vec<i16> = self.pcm_buffer.drain(..self.frame_samples).collect();
            // Max Opus packet: 1275 bytes per channel per frame
            let mut out_buf = vec![0u8; 4000];
            let n = self
                .encoder
                .encode(&frame, &mut out_buf)
                .map_err(|e| anyhow::anyhow!("Opus encode: {e}"))?;
            output.extend_from_slice(&out_buf[..n]);
        }
        Ok(output)
    }

    fn content_type(&self) -> &'static str {
        "audio/opus"
    }
}

// ---------------------------------------------------------------------------
// AAC & AAC+ Encoder (FDK-AAC)
// ---------------------------------------------------------------------------

pub struct AacEncoder {
    encoder: fdk_aac::enc::Encoder,
    frame_samples: usize,
    pcm_buffer: Vec<f32>,
    content_type: &'static str,
}

impl AacEncoder {
    pub fn new(
        sample_rate: u32,
        channels: u16,
        bitrate_kbps: u32,
        is_plus: bool,
    ) -> anyhow::Result<Self> {
        use fdk_aac::enc::{EncoderParams, BitRate, Transport, ChannelMode, AudioObjectType};

        let audio_object_type = if is_plus {
            AudioObjectType::Mpeg4HeAac
        } else {
            AudioObjectType::Mpeg4LowComplexity
        };

        let channel_mode = if channels == 1 {
            ChannelMode::Mono
        } else {
            ChannelMode::Stereo
        };

        let params = EncoderParams {
            bit_rate: BitRate::Cbr(bitrate_kbps * 1000),
            sample_rate,
            transport: Transport::Adts,
            channels: channel_mode,
            audio_object_type,
        };

        let encoder = fdk_aac::enc::Encoder::new(params)
            .map_err(|e| anyhow::anyhow!("Failed to create FDK-AAC encoder: {:?}", e))?;

        let info = encoder.info()
            .map_err(|e| anyhow::anyhow!("Failed to get FDK-AAC info: {:?}", e))?;

        let frame_samples = info.frameLength as usize * channels as usize;
        let content_type = if is_plus {
            "audio/aacp"
        } else {
            "audio/aac"
        };

        Ok(Self {
            encoder,
            frame_samples,
            pcm_buffer: Vec::new(),
            content_type,
        })
    }
}

impl AudioEncoder for AacEncoder {
    fn encode_frame(&mut self, pcm: &[f32]) -> anyhow::Result<Vec<u8>> {
        self.pcm_buffer.extend_from_slice(pcm);
        let mut output = Vec::new();

        while self.pcm_buffer.len() >= self.frame_samples {
            let frame: Vec<f32> = self.pcm_buffer.drain(..self.frame_samples).collect();

            let i16_samples: Vec<i16> = frame
                .iter()
                .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect();

            let mut out_buf = vec![0u8; 8192];
            let info = self.encoder.encode(&i16_samples, &mut out_buf)
                .map_err(|e| anyhow::anyhow!("FDK-AAC encode error: {:?}", e))?;

            if info.output_size > 0 {
                output.extend_from_slice(&out_buf[..info.output_size]);
            }
        }

        Ok(output)
    }

    fn flush(&mut self) -> anyhow::Result<Vec<u8>> {
        if !self.pcm_buffer.is_empty() {
            self.pcm_buffer.resize(self.frame_samples, 0.0);
            let frame: Vec<f32> = self.pcm_buffer.drain(..).collect();
            let i16_samples: Vec<i16> = frame
                .iter()
                .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect();

            let mut out_buf = vec![0u8; 8192];
            if let Ok(info) = self.encoder.encode(&i16_samples, &mut out_buf) {
                if info.output_size > 0 {
                    return Ok(out_buf[..info.output_size].to_vec());
                }
            }
        }
        Ok(vec![])
    }

    fn content_type(&self) -> &'static str {
        self.content_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vorbis_uneven_chunks() {
        let mut encoder = VorbisEncoderImpl::new(44100, 2, 128).unwrap();
        
        // Pass 3 samples (1.5 frames for stereo)
        let frame1 = vec![0.1, -0.1, 0.2];
        let _out1 = encoder.encode_frame(&frame1).unwrap();
        // 2 samples processed, 1 sample kept in buffer
        assert_eq!(encoder.pcm_buffer.len(), 1);

        // Pass 1 sample (0.5 frames for stereo)
        let frame2 = vec![-0.2];
        let _out2 = encoder.encode_frame(&frame2).unwrap();
        // The kept 1 sample + 1 sample = 2 samples processed, 0 kept in buffer
        assert_eq!(encoder.pcm_buffer.len(), 0);

        // Flush remaining
        let _out3 = encoder.flush().unwrap();
    }

}
