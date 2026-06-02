use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;

/// A basic WAV file writer for recording raw PCM audio.
/// Writes a standard 44-byte WAV header, then appends PCM data.
/// The header is finalized when `finalize()` is called with the total data size.
pub struct WavWriter {
    file: File,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    data_bytes_written: u32,
}

impl WavWriter {
    /// Create a new WAV file at the given path.
    /// Writes a placeholder header that will be updated on finalize.
    pub fn new(path: &Path, sample_rate: u32, channels: u16) -> io::Result<Self> {
        let mut file = File::create(path)?;
        let bits_per_sample: u16 = 16; // i16 PCM

        // Write a placeholder header — will be updated in finalize()
        let header = build_wav_header(sample_rate, channels, bits_per_sample, 0);
        file.write_all(&header)?;

        Ok(Self {
            file,
            sample_rate,
            channels,
            bits_per_sample,
            data_bytes_written: 0,
        })
    }

    /// Write f32 PCM samples (normalized -1.0 to 1.0) as i16 to the WAV file.
    pub fn write_samples(&mut self, samples: &[f32]) -> io::Result<()> {
        // Convert f32 → i16 and write as little-endian bytes
        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let i16_val = (clamped * 32767.0) as i16;
            self.file.write_all(&i16_val.to_le_bytes())?;
            self.data_bytes_written += 2;
        }
        Ok(())
    }

    /// Finalize the WAV file by seeking back and updating the header
    /// with the correct data size.
    pub fn finalize(mut self) -> io::Result<()> {
        let header = build_wav_header(
            self.sample_rate,
            self.channels,
            self.bits_per_sample,
            self.data_bytes_written,
        );
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&header)?;
        self.file.flush()?;
        Ok(())
    }
}

/// Build a 44-byte WAV (RIFF) header.
fn build_wav_header(
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    data_size: u32,
) -> Vec<u8> {
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let chunk_size = 36 + data_size; // total file size - 8

    let mut header = Vec::with_capacity(44);

    // RIFF header
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&chunk_size.to_le_bytes());
    header.extend_from_slice(b"WAVE");

    // fmt sub-chunk
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes()); // sub-chunk size (PCM = 16)
    header.extend_from_slice(&1u16.to_le_bytes()); // audio format (PCM = 1)
    header.extend_from_slice(&channels.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&byte_rate.to_le_bytes());
    header.extend_from_slice(&block_align.to_le_bytes());
    header.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data sub-chunk
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_size.to_le_bytes());

    header
}
