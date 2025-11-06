use crate::{AsrEngine, EngineEvent, EngineError, TranscriptFragment};
use crate::audio::{AudioCapture, AudioConfig, AudioChunk};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, timeout};
use tracing::{debug, info, warn};
use std::collections::VecDeque;

/// Voice Activity Detection using energy and zero-crossing rate
pub struct VoiceActivityDetector {
    energy_threshold: f32,
    zcr_threshold: f32,
    min_speech_duration_ms: u64,
    max_silence_duration_ms: u64,
    frame_size: usize,
}

impl VoiceActivityDetector {
    pub fn new() -> Self {
        Self {
            energy_threshold: 0.01,
            zcr_threshold: 0.3,
            min_speech_duration_ms: 300,  // 300ms minimum speech
            max_silence_duration_ms: 1500, // 1.5s silence to end
            frame_size: 480, // 30ms at 16kHz
        }
    }

    pub fn with_energy_threshold(mut self, threshold: f32) -> Self {
        self.energy_threshold = threshold;
        self
    }

    pub fn with_silence_duration(mut self, duration_ms: u64) -> Self {
        self.max_silence_duration_ms = duration_ms;
        self
    }

    /// Detect voice activity in audio frame
    pub fn detect_voice(&self, frame: &[f32]) -> bool {
        if frame.len() < self.frame_size {
            return false;
        }

        // Calculate RMS energy
        let energy = self.calculate_energy(frame);
        
        // Calculate zero-crossing rate
        let zcr = self.calculate_zcr(frame);
        
        // Voice detected if both energy and ZCR are above thresholds
        energy > self.energy_threshold && zcr > self.zcr_threshold
    }

    fn calculate_energy(&self, frame: &[f32]) -> f32 {
        let sum_squares: f32 = frame.iter().map(|&x| x * x).sum();
        (sum_squares / frame.len() as f32).sqrt()
    }

    fn calculate_zcr(&self, frame: &[f32]) -> f32 {
        let mut zero_crossings = 0;
        for i in 1..frame.len() {
            if (frame[i] >= 0.0) != (frame[i-1] >= 0.0) {
                zero_crossings += 1;
            }
        }
        zero_crossings as f32 / frame.len() as f32
    }
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming ASR engine that processes real-time audio
pub struct StreamingAsr {
    vad: VoiceActivityDetector,
    audio_config: AudioConfig,
    whisper_bin: Option<String>,
    whisper_model: Option<String>,
    buffer_duration_ms: u64,
}

impl StreamingAsr {
    pub fn new() -> Self {
        Self {
            vad: VoiceActivityDetector::new(),
            audio_config: AudioConfig::default(),
            whisper_bin: None,
            whisper_model: None,
            buffer_duration_ms: 5000, // 5 second buffer
        }
    }

    pub fn with_whisper(mut self, bin_path: String, model_path: String) -> Self {
        self.whisper_bin = Some(bin_path);
        self.whisper_model = Some(model_path);
        self
    }

    pub fn with_vad_config(mut self, vad: VoiceActivityDetector) -> Self {
        self.vad = vad;
        self
    }

    pub fn with_audio_config(mut self, config: AudioConfig) -> Self {
        self.audio_config = config;
        self
    }

    /// Process audio chunks and detect speech segments
    async fn process_audio_stream(&self, events: mpsc::Sender<EngineEvent>) -> Result<Vec<AudioChunk>, EngineError> {
        info!("Starting streaming audio capture for ASR");
        
        // Create audio capture
        let capture = AudioCapture::new(self.audio_config.clone())
            .map_err(|e| EngineError::Asr(format!("Failed to create audio capture: {}", e)))?;
        
        capture.start()
            .map_err(|e| EngineError::Asr(format!("Failed to start audio capture: {}", e)))?;

        let mut audio_buffer = VecDeque::new();
        let mut speech_detected = false;
        let mut last_speech_time = std::time::Instant::now();
        let mut speech_start_time: Option<std::time::Instant> = None;
        
        // Collect audio until we detect end of speech
        loop {
            if let Some(chunk) = capture.next_chunk() {
                let mono_data = chunk.to_mono();
                let has_voice = self.vad.detect_voice(&mono_data);
                
                if has_voice {
                    if !speech_detected {
                        speech_detected = true;
                        speech_start_time = Some(std::time::Instant::now());
                        info!("Speech detected, starting recording");
                        
                        // Send partial transcript to indicate we're listening
                        let _ = events.send(EngineEvent::PartialTranscript(TranscriptFragment {
                            text: "...".to_string(),
                            is_final: false,
                        })).await;
                    }
                    last_speech_time = std::time::Instant::now();
                }
                
                // Always buffer audio when we're in speech mode
                if speech_detected {
                    audio_buffer.push_back(chunk);
                    
                    // Limit buffer size to prevent memory issues
                    let max_chunks = (self.buffer_duration_ms * self.audio_config.sample_rate as u64) / 
                                   (1000 * self.audio_config.buffer_size as u64);
                    
                    while audio_buffer.len() > max_chunks as usize {
                        audio_buffer.pop_front();
                    }
                }
                
                // Check for end of speech
                if speech_detected {
                    let silence_duration = last_speech_time.elapsed();
                    
                    if silence_duration.as_millis() > self.vad.max_silence_duration_ms as u128 {
                        if let Some(start_time) = speech_start_time {
                            let speech_duration = start_time.elapsed();
                            
                            if speech_duration.as_millis() > self.vad.min_speech_duration_ms as u128 {
                                info!("End of speech detected after {}ms", speech_duration.as_millis());
                                break;
                            } else {
                                debug!("Speech too short ({}ms), continuing", speech_duration.as_millis());
                                speech_detected = false;
                                audio_buffer.clear();
                            }
                        }
                    }
                }
            } else {
                // No audio available, small delay
                sleep(Duration::from_millis(10)).await;
            }
        }
        
        info!("Collected {} audio chunks for transcription", audio_buffer.len());
        Ok(audio_buffer.into())
    }

    /// Convert audio chunks to WAV file for Whisper processing
    async fn chunks_to_wav(&self, chunks: Vec<AudioChunk>) -> Result<String, EngineError> {
        if chunks.is_empty() {
            return Err(EngineError::Asr("No audio chunks to process".to_string()));
        }

        // Create temporary WAV file
        let temp_path = format!("/tmp/friday_audio_{}.wav", std::process::id());
        
        // Combine all audio data
        let mut combined_audio = Vec::new();
        for chunk in chunks {
            let mono_data = chunk.resample(self.audio_config.sample_rate);
            combined_audio.extend(mono_data);
        }

        // Write to WAV file
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.audio_config.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&temp_path, spec)
            .map_err(|e| EngineError::Asr(format!("Failed to create WAV file: {}", e)))?;

        for &sample in &combined_audio {
            let sample_i16 = (sample * i16::MAX as f32) as i16;
            writer.write_sample(sample_i16)
                .map_err(|e| EngineError::Asr(format!("Failed to write audio sample: {}", e)))?;
        }

        writer.finalize()
            .map_err(|e| EngineError::Asr(format!("Failed to finalize WAV file: {}", e)))?;

        debug!("Created temporary WAV file: {}", temp_path);
        Ok(temp_path)
    }

    /// Transcribe audio using Whisper
    async fn transcribe_with_whisper(&self, wav_path: &str, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        let whisper_bin = self.whisper_bin.as_ref()
            .ok_or_else(|| EngineError::Asr("Whisper binary not configured".to_string()))?;
        let whisper_model = self.whisper_model.as_ref()
            .ok_or_else(|| EngineError::Asr("Whisper model not configured".to_string()))?;

        info!("Transcribing audio with Whisper");
        
        // Send processing indicator
        let _ = events.send(EngineEvent::PartialTranscript(TranscriptFragment {
            text: "Processing...".to_string(),
            is_final: false,
        })).await;

        let mut cmd = tokio::process::Command::new(whisper_bin);
        cmd.arg("-m").arg(whisper_model)
            .arg("-f").arg(wav_path)
            .arg("--output-txt")
            .arg("--no-timestamps");

        // Set timeout for Whisper processing
        let output = timeout(Duration::from_secs(30), cmd.output()).await
            .map_err(|_| EngineError::Asr("Whisper processing timed out".to_string()))?
            .map_err(|e| EngineError::Asr(format!("Failed to run Whisper: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EngineError::Asr(format!("Whisper failed: {}", stderr)));
        }

        let transcript = String::from_utf8(output.stdout)
            .map_err(|e| EngineError::Asr(format!("Invalid UTF-8 in Whisper output: {}", e)))?;
        
        let transcript = transcript.trim();
        
        // Clean up temporary file
        let _ = tokio::fs::remove_file(wav_path).await;
        
        if transcript.is_empty() {
            warn!("Whisper returned empty transcript");
            Ok("".to_string())
        } else {
            info!("Whisper transcription: '{}'", transcript);
            Ok(transcript.to_string())
        }
    }

    /// Simulate streaming transcription with mock data
    async fn simulate_streaming_transcription(&self, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        info!("Simulating streaming transcription");
        
        let partial_transcripts = [
            "I",
            "I need",
            "I need to",
            "I need to set",
            "I need to set a",
            "I need to set a timer",
        ];

        for (i, partial) in partial_transcripts.iter().enumerate() {
            let _ = events.send(EngineEvent::PartialTranscript(TranscriptFragment {
                text: partial.to_string(),
                is_final: false,
            })).await;
            
            // Simulate processing delay
            sleep(Duration::from_millis(200 + i as u64 * 50)).await;
        }

        Ok("I need to set a timer for 5 minutes".to_string())
    }
}

#[async_trait]
impl AsrEngine for StreamingAsr {
    async fn stream_until_silence(&self, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        debug!("Starting streaming ASR");

        // If Whisper is configured, use real audio processing
        if self.whisper_bin.is_some() && self.whisper_model.is_some() {
            // Process real audio stream
            let audio_chunks = self.process_audio_stream(events.clone()).await?;
            
            if audio_chunks.is_empty() {
                return Ok("".to_string());
            }

            // Convert to WAV and transcribe
            let wav_path = self.chunks_to_wav(audio_chunks).await?;
            let transcript = self.transcribe_with_whisper(&wav_path, events).await?;
            
            Ok(transcript)
        } else {
            // Use simulation for development/testing
            self.simulate_streaming_transcription(events).await
        }
    }
}

impl Default for StreamingAsr {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_energy_calculation() {
        let vad = VoiceActivityDetector::new();
        
        // High energy signal
        let high_energy_frame = vec![0.5; 480];
        let energy = vad.calculate_energy(&high_energy_frame);
        assert!(energy > 0.4);
        
        // Low energy signal
        let low_energy_frame = vec![0.01; 480];
        let energy = vad.calculate_energy(&low_energy_frame);
        assert!(energy < 0.1);
    }

    #[test]
    fn test_vad_zcr_calculation() {
        let vad = VoiceActivityDetector::new();
        
        // Alternating signal (high ZCR)
        let mut alternating_frame = Vec::new();
        for i in 0..480 {
            alternating_frame.push(if i % 2 == 0 { 0.1 } else { -0.1 });
        }
        let zcr = vad.calculate_zcr(&alternating_frame);
        assert!(zcr > 0.8);
        
        // Constant signal (low ZCR)
        let constant_frame = vec![0.1; 480];
        let zcr = vad.calculate_zcr(&constant_frame);
        assert!(zcr < 0.1);
    }

    #[test]
    fn test_vad_voice_detection() {
        let vad = VoiceActivityDetector::new()
            .with_energy_threshold(0.05);
        
        // Create a frame that should be detected as voice
        let mut voice_frame = Vec::new();
        for i in 0..480 {
            // Sine wave with decent amplitude
            let sample = 0.1 * (i as f32 * 0.1).sin();
            voice_frame.push(sample);
        }
        
        assert!(vad.detect_voice(&voice_frame));
        
        // Create a frame that should be detected as silence
        let silence_frame = vec![0.001; 480];
        assert!(!vad.detect_voice(&silence_frame));
    }

    #[tokio::test]
    async fn test_streaming_asr_simulation() {
        let asr = StreamingAsr::new();
        let (tx, mut rx) = mpsc::channel(10);
        
        let result = asr.simulate_streaming_transcription(tx).await;
        assert!(result.is_ok());
        
        // Check that partial transcripts were sent
        let mut partial_count = 0;
        while let Ok(event) = rx.try_recv() {
            if let EngineEvent::PartialTranscript(_) = event {
                partial_count += 1;
            }
        }
        assert!(partial_count > 0);
    }
}