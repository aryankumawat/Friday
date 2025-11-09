use crate::{AsrEngine, EngineEvent, EngineError, TranscriptFragment};
use crate::audio::{AudioCapture, AudioConfig};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::fs;
use tracing::{debug, info};
use std::path::Path;

/// Real recording-based ASR that captures audio after wake detection
pub struct RecordingAsr {
    audio_config: AudioConfig,
    recording_duration_ms: u64,
    silence_threshold: f32,
    silence_duration_ms: u64,
    output_path: String,
}

impl RecordingAsr {
    pub fn new() -> Self {
        Self {
            audio_config: AudioConfig::default(),
            recording_duration_ms: 5000, // 5 seconds max
            silence_threshold: 0.01,
            silence_duration_ms: 1500, // 1.5 seconds of silence to stop
            output_path: "/tmp/friday_recording.wav".to_string(),
        }
    }

    pub fn with_audio_config(mut self, config: AudioConfig) -> Self {
        self.audio_config = config;
        self
    }

    pub fn with_max_duration(mut self, duration_ms: u64) -> Self {
        self.recording_duration_ms = duration_ms;
        self
    }

    pub fn with_silence_detection(mut self, threshold: f32, duration_ms: u64) -> Self {
        self.silence_threshold = threshold;
        self.silence_duration_ms = duration_ms;
        self
    }

    pub fn with_output_path(mut self, path: String) -> Self {
        self.output_path = path;
        self
    }

    fn calculate_energy(&self, audio_data: &[f32]) -> f32 {
        if audio_data.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = audio_data.iter().map(|&x| x * x).sum();
        (sum_squares / audio_data.len() as f32).sqrt()
    }

    async fn record_audio(&self, events: mpsc::Sender<EngineEvent>) -> Result<Vec<f32>, EngineError> {
        info!("🎤 Recording audio... (speak now!)");
        
        // Create audio capture
        let capture = AudioCapture::new(self.audio_config.clone())
            .map_err(|e| EngineError::Audio(e.to_string()))?;
        
        capture.start()
            .map_err(|e| EngineError::Audio(e.to_string()))?;
        
        let mut audio_buffer = Vec::new();
        let start_time = std::time::Instant::now();
        let mut silence_start: Option<std::time::Instant> = None;
        
        // Send initial partial
        events.send(EngineEvent::PartialTranscript(TranscriptFragment {
            text: "[Recording...]".to_string(),
            is_final: false,
        })).await.map_err(|e| EngineError::Asr(e.to_string()))?;
        
        loop {
            // Check max duration
            if start_time.elapsed().as_millis() >= self.recording_duration_ms as u128 {
                info!("⏱️  Max recording duration reached");
                break;
            }
            
            // Get audio chunk
            if let Some(chunk) = capture.try_next_chunk() {
                let mono_data = chunk.to_mono();
                let energy = self.calculate_energy(&mono_data);
                
                // Add to buffer
                audio_buffer.extend(&mono_data);
                
                // Check for silence
                if energy < self.silence_threshold {
                    if silence_start.is_none() {
                        silence_start = Some(std::time::Instant::now());
                        debug!("🔇 Silence detected, starting timer");
                    } else if let Some(start) = silence_start {
                        let silence_duration = start.elapsed().as_millis();
                        if silence_duration >= self.silence_duration_ms as u128 {
                            info!("🔇 Silence detected for {}ms, stopping recording", silence_duration);
                            break;
                        }
                    }
                } else {
                    // Reset silence timer
                    if silence_start.is_some() {
                        debug!("🔊 Sound resumed, resetting silence timer");
                        silence_start = None;
                    }
                }
                
                // Send periodic updates
                if audio_buffer.len() % 16000 == 0 { // Every second at 16kHz
                    let duration_secs = audio_buffer.len() / self.audio_config.sample_rate as usize;
                    events.send(EngineEvent::PartialTranscript(TranscriptFragment {
                        text: format!("[Recording... {}s]", duration_secs),
                        is_final: false,
                    })).await.map_err(|e| EngineError::Asr(e.to_string()))?;
                }
            } else {
                // No audio data yet
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }
        
        let duration_secs = audio_buffer.len() as f32 / self.audio_config.sample_rate as f32;
        info!("✅ Recording complete: {:.2}s, {} samples", duration_secs, audio_buffer.len());
        
        Ok(audio_buffer)
    }

    async fn save_to_wav(&self, audio_data: &[f32]) -> Result<(), EngineError> {
        use hound::{WavWriter, WavSpec, SampleFormat};
        
        let spec = WavSpec {
            channels: 1,
            sample_rate: self.audio_config.sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        
        let mut writer = WavWriter::create(&self.output_path, spec)
            .map_err(|e| EngineError::Audio(e.to_string()))?;
        
        for &sample in audio_data {
            let sample_i16 = (sample * i16::MAX as f32) as i16;
            writer.write_sample(sample_i16)
                .map_err(|e| EngineError::Audio(e.to_string()))?;
        }
        
        writer.finalize()
            .map_err(|e| EngineError::Audio(e.to_string()))?;
        
        info!("💾 Audio saved to: {}", self.output_path);
        Ok(())
    }

    async fn transcribe_with_whisper(&self) -> Result<String, EngineError> {
        // Try to use whisper.cpp if available
        let whisper_bin = "whisper.cpp/build/bin/main";
        let model_path = "whisper.cpp/models/ggml-base.bin";
        
        if !Path::new(whisper_bin).exists() {
            info!("⚠️  Whisper binary not found, using fallback");
            return Ok("[Whisper not available - please say your command]".to_string());
        }
        
        if !Path::new(model_path).exists() {
            info!("⚠️  Whisper model not found, using fallback");
            return Ok("[Whisper model not available - please say your command]".to_string());
        }
        
        info!("🧠 Transcribing with Whisper...");
        
        let output = tokio::process::Command::new(whisper_bin)
            .arg("-m").arg(model_path)
            .arg("-f").arg(&self.output_path)
            .arg("--no-timestamps")
            .arg("--language").arg("en")
            .output()
            .await
            .map_err(|e| EngineError::Asr(format!("Failed to run Whisper: {}", e)))?;
        
        if !output.status.success() {
            return Err(EngineError::Asr(format!("Whisper failed: {}", 
                String::from_utf8_lossy(&output.stderr))));
        }
        
        let transcript = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        
        info!("✅ Transcription: '{}'", transcript);
        Ok(transcript)
    }

    async fn transcribe_with_system_asr(&self) -> Result<String, EngineError> {
        // Fallback: Use macOS system speech recognition if available
        #[cfg(target_os = "macos")]
        {
            info!("🍎 Attempting macOS system speech recognition...");
            // This is a placeholder - macOS doesn't have a simple CLI for speech recognition
            // In a real implementation, you'd use the Speech framework via Objective-C bindings
        }
        
        // Ultimate fallback: Return a helpful message
        Ok("I heard you, but speech recognition is not set up yet. Please configure Whisper.".to_string())
    }
}

#[async_trait]
impl AsrEngine for RecordingAsr {
    async fn stream_until_silence(&self, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        info!("🎙️  Starting real audio recording...");
        
        // Record audio
        let audio_data = self.record_audio(events.clone()).await?;
        
        if audio_data.is_empty() {
            return Err(EngineError::Asr("No audio data recorded".to_string()));
        }
        
        // Save to WAV file
        self.save_to_wav(&audio_data).await?;
        
        // Send recording complete event
        events.send(EngineEvent::PartialTranscript(TranscriptFragment {
            text: "[Processing...]".to_string(),
            is_final: false,
        })).await.map_err(|e| EngineError::Asr(e.to_string()))?;
        
        // Try to transcribe with Whisper
        let transcript = match self.transcribe_with_whisper().await {
            Ok(text) if !text.is_empty() && !text.contains("not available") => text,
            _ => {
                // Fallback to system ASR or helpful message
                self.transcribe_with_system_asr().await?
            }
        };
        
        // Send final transcript
        events.send(EngineEvent::PartialTranscript(TranscriptFragment {
            text: transcript.clone(),
            is_final: true,
        })).await.map_err(|e| EngineError::Asr(e.to_string()))?;
        
        Ok(transcript)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_calculation() {
        let asr = RecordingAsr::new();
        
        let silent = vec![0.0f32; 1000];
        assert!(asr.calculate_energy(&silent) < 0.001);
        
        let loud = vec![0.5f32; 1000];
        assert!(asr.calculate_energy(&loud) > 0.4);
    }

    #[test]
    fn test_configuration() {
        let asr = RecordingAsr::new()
            .with_max_duration(10000)
            .with_silence_detection(0.02, 2000)
            .with_output_path("/tmp/test.wav".to_string());
        
        assert_eq!(asr.recording_duration_ms, 10000);
        assert_eq!(asr.silence_threshold, 0.02);
        assert_eq!(asr.silence_duration_ms, 2000);
        assert_eq!(asr.output_path, "/tmp/test.wav");
    }
}
