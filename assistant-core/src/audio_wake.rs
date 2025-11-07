use crate::{WakeDetector, EngineError, audio::{AudioCapture, AudioConfig}};
use async_trait::async_trait;
use tracing::{debug, info};

/// Real audio-based wake detection using microphone input
pub struct AudioWake {
    energy_threshold: f32,
    trigger_duration_ms: u64,
    audio_config: AudioConfig,
}

impl AudioWake {
    pub fn new() -> Self {
        Self {
            energy_threshold: 0.02,
            trigger_duration_ms: 500,
            audio_config: AudioConfig::default(),
        }
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.energy_threshold = threshold;
        self
    }

    pub fn with_trigger_duration(mut self, duration_ms: u64) -> Self {
        self.trigger_duration_ms = duration_ms;
        self
    }

    fn calculate_energy(&self, audio_data: &[f32]) -> f32 {
        if audio_data.is_empty() {
            return 0.0;
        }
        
        let sum_squares: f32 = audio_data.iter().map(|&x| x * x).sum();
        (sum_squares / audio_data.len() as f32).sqrt()
    }
}

#[async_trait]
impl WakeDetector for AudioWake {
    async fn wait_for_wake(&self) -> Result<(), EngineError> {
        info!("🎤 Starting REAL audio-based wake detection...");
        info!("📊 Energy threshold: {:.4}", self.energy_threshold);
        info!("⏱️  Trigger duration: {}ms", self.trigger_duration_ms);
        info!("");
        info!("💡 Speak loudly or make a sound to wake Friday!");
        info!("   (The louder, the better - try clapping or saying 'HEY FRIDAY' loudly)");
        info!("");
        
        // Create audio capture
        let capture = AudioCapture::new(self.audio_config.clone())
            .map_err(|e| EngineError::Audio(e.to_string()))?;
        
        capture.start()
            .map_err(|e| EngineError::Audio(e.to_string()))?;
        
        let mut energy_start: Option<std::time::Instant> = None;
        let mut max_energy_seen = 0.0f32;
        
        info!("🔊 Listening... (monitoring audio levels)");
        
        loop {
            // Try to get audio chunk
            if let Some(chunk) = capture.try_next_chunk() {
                let audio_data = chunk.to_mono();
                let energy = self.calculate_energy(&audio_data);
                
                // Track maximum energy for debugging
                if energy > max_energy_seen {
                    max_energy_seen = energy;
                    debug!("📈 New max energy: {:.4}", max_energy_seen);
                }
                
                // Show periodic energy readings
                if rand::random::<f32>() < 0.01 { // 1% of the time
                    info!("📊 Current audio level: {:.4} (threshold: {:.4}, max seen: {:.4})", 
                          energy, self.energy_threshold, max_energy_seen);
                }
                
                if energy > self.energy_threshold {
                    // Energy detected!
                    if energy_start.is_none() {
                        energy_start = Some(std::time::Instant::now());
                        info!("🔊 Sound detected! Energy: {:.4} (above threshold {:.4})", 
                              energy, self.energy_threshold);
                        info!("⏳ Waiting for sustained sound...");
                    } else if let Some(start) = energy_start {
                        let duration = start.elapsed();
                        if duration.as_millis() >= self.trigger_duration_ms as u128 {
                            info!("✅ WAKE WORD DETECTED! (sustained for {}ms)", duration.as_millis());
                            info!("🎤 Friday is now listening to your command...");
                            return Ok(());
                        } else {
                            // Show progress
                            if duration.as_millis() % 100 == 0 {
                                debug!("⏱️  Sustained for {}ms / {}ms", 
                                      duration.as_millis(), self.trigger_duration_ms);
                            }
                        }
                    }
                } else {
                    // Energy dropped below threshold
                    if energy_start.is_some() {
                        debug!("🔇 Sound stopped, resetting (energy: {:.4})", energy);
                        energy_start = None;
                    }
                }
            } else {
                // No audio data yet, wait a bit
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_calculation() {
        let wake = AudioWake::new();
        
        // Silent audio
        let silent = vec![0.0f32; 1000];
        assert!(wake.calculate_energy(&silent) < 0.001);
        
        // Loud audio
        let loud = vec![0.5f32; 1000];
        assert!(wake.calculate_energy(&loud) > 0.4);
        
        // Mixed audio
        let mixed = vec![0.1, -0.1, 0.2, -0.2, 0.1];
        let energy = wake.calculate_energy(&mixed);
        assert!(energy > 0.0 && energy < 0.3);
    }
}
