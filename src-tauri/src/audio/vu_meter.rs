use crate::audio::capture::VuLevel;
use tauri::Emitter;

/// Compute Root Mean Square of a slice of normalized f32 samples [-1.0, 1.0].
/// Returns value in dBFS.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -60.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| if s.is_finite() { s * s } else { 0.0 }).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();
    linear_to_dbfs(rms)
}

/// Convert linear amplitude (0.0–1.0) to dBFS (-inf to 0.0).
/// Clamp to -60.0 as practical silence floor.
pub fn linear_to_dbfs(linear: f32) -> f32 {
    if linear <= 0.0 {
        return -60.0;
    }
    let db = 20.0 * linear.log10();
    db.max(-60.0)
}

/// Peak hold helper — call every frame, returns held peak in dBFS.
pub struct PeakHold {
    pub peak_db: f32,
    hold_samples: u32,
    decay_per_frame: f32,
}

impl Default for PeakHold {
    fn default() -> Self {
        Self::new()
    }
}

impl PeakHold {
    pub fn new() -> Self {
        Self {
            peak_db: -60.0,
            hold_samples: 0,
            decay_per_frame: 0.5,
        }
    }

    pub fn update(&mut self, current_db: f32) -> f32 {
        if current_db >= self.peak_db {
            self.peak_db = current_db;
            self.hold_samples = 90; // ~3 seconds at 30fps
        } else if self.hold_samples > 0 {
            self.hold_samples -= 1;
        } else {
            self.peak_db = (self.peak_db - self.decay_per_frame).max(-60.0);
        }
        self.peak_db
    }
}

/// VU event loop — runs on a dedicated std::thread (NOT the audio callback).
/// Receives VuLevel from the audio callback via std::sync::mpsc and emits
/// Tauri events to the frontend at ~30fps.
pub fn start_vu_emitter(app: tauri::AppHandle, vu_rx: std::sync::mpsc::Receiver<VuLevel>) {
    std::thread::spawn(move || {
        let mut peak_l = PeakHold::new();
        let mut peak_r = PeakHold::new();
        let mut last_emit = std::time::Instant::now();
        loop {
            match vu_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(mut level) => {
                    // Drain all pending messages to get the latest one
                    while let Ok(next) = vu_rx.try_recv() {
                        level = next;
                    }
                    let now = std::time::Instant::now();
                    if now.duration_since(last_emit) >= std::time::Duration::from_millis(33) {
                        let _ = app.emit(
                            "vu-meter",
                            serde_json::json!({
                                "left": level.left,
                                "right": level.right,
                                "peak_left": peak_l.update(level.left),
                                "peak_right": peak_r.update(level.right),
                            }),
                        );
                        last_emit = now;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // No audio for 100ms — emit silence
                    let _ = app.emit(
                        "vu-meter",
                        serde_json::json!({
                            "left": -60.0, "right": -60.0,
                            "peak_left": -60.0, "peak_right": -60.0,
                        }),
                    );
                    last_emit = std::time::Instant::now();
                }
                Err(_) => break, // channel closed — audio stopped
            }
        }
    });
}
