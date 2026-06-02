pub mod capture;
pub mod encoder;
pub mod vu_meter;

use ringbuf::{traits::Split, HeapCons, HeapProd, HeapRb};
use tokio::sync::mpsc;

pub use capture::{AudioStream, VuLevel};
pub use encoder::{create_encoder, AudioEncoder};
pub use vu_meter::{compute_rms, linear_to_dbfs, start_vu_emitter, PeakHold};

/// Ring buffer sized for 200ms at 48kHz stereo = 19200 f32 samples.
/// Double it to be safe: 38400.
pub const RING_BUFFER_SIZE: usize = 38_400;

/// Create a lock-free SPSC ring buffer.
/// The HeapProd goes to the audio thread (via start_capture).
/// The HeapCons goes to the encoder thread.
pub fn create_ring_buffer() -> (HeapProd<f32>, HeapCons<f32>) {
    HeapRb::<f32>::new(RING_BUFFER_SIZE).split()
}

/// Encoder thread: reads PCM from ring buffer, encodes, sends to network channel.
/// Runs as a dedicated OS thread. Exits when shutdown signal received or network channel closes.
pub fn run_encoder_thread(
    mut consumer: HeapCons<f32>,
    mut encoder: Box<dyn AudioEncoder>,
    network_tx: mpsc::Sender<bytes::Bytes>,
    shutdown_rx: std::sync::mpsc::Receiver<()>,
) {
    use ringbuf::traits::Consumer;

    let mut frame_buf: Vec<f32> = Vec::with_capacity(4096);
    let read_chunk = 4096usize;

    loop {
        // Check shutdown
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        // Drain ring buffer
        frame_buf.resize(read_chunk, 0.0);
        let read = consumer.pop_slice(&mut frame_buf);

        if read == 0 {
            // Buffer empty — park thread until unparked by audio capture callback
            std::thread::park_timeout(std::time::Duration::from_millis(100));
            continue;
        }

        let pcm = &frame_buf[..read];

        match encoder.encode_frame(pcm) {
            Ok(encoded) if !encoded.is_empty() => {
                let bytes = bytes::Bytes::from(encoded);
                if network_tx.blocking_send(bytes).is_err() {
                    break; // network thread shut down
                }
            }
            Ok(_) => {} // no output yet (buffering)
            Err(e) => {
                log::error!("Encoder error: {e}");
                break;
            }
        }
    }

    // Flush remaining encoder state
    if let Ok(tail) = encoder.flush() {
        if !tail.is_empty() {
            let _ = network_tx.blocking_send(bytes::Bytes::from(tail));
        }
    }
}
