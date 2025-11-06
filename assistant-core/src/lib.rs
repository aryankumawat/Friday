use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio::process::Command;
use tracing::{instrument};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavWriter;

pub mod audio;
pub mod realtime_wake;
pub mod enhanced_nlu;
pub mod enhanced_executor;
pub mod config;
pub mod streaming_asr;
pub mod plugin_system;
pub mod plugin_executor;
pub mod dialogue_manager;
pub mod memory_manager;
pub mod voice_training;
pub mod analytics;
pub mod advanced_config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptFragment {
    pub text: String,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineEvent {
    WakeDetected,
    PartialTranscript(TranscriptFragment),
    FinalTranscript(String),
    TtsStarted,
    TtsFinished,
    IntentRecognized(Intent),
    ExecutionStarted(String),
    ExecutionFinished(String),
    Notification(String),
}

#[derive(thiserror::Error, Debug)]
pub enum EngineError {
    #[error("audio error: {0}")]
    Audio(String),
    #[error("wake error: {0}")]
    Wake(String),
    #[error("asr error: {0}")]
    Asr(String),
    #[error("tts error: {0}")]
    Tts(String),
}

#[async_trait]
pub trait WakeDetector: Send + Sync {
    async fn wait_for_wake(&self) -> Result<(), EngineError>;
}

#[async_trait]
pub trait AsrEngine: Send + Sync {
    async fn stream_until_silence(&self, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError>;
}

#[async_trait]
pub trait TtsEngine: Send + Sync {
    async fn speak(&self, text: &str, events: mpsc::Sender<EngineEvent>) -> Result<(), EngineError>;
}

pub struct AudioCapture {
    pub sample_rate: u32,
}

impl AudioCapture {
    pub fn new(sample_rate: u32) -> Self { Self { sample_rate } }

    pub fn list_input_devices() -> Vec<String> {
        let host = cpal::default_host();
        match host.input_devices() {
            Ok(iter) => iter.filter_map(|d| d.name().ok()).collect(),
            Err(_) => vec![],
        }
    }

    pub fn start_logging_input(&self) -> Result<cpal::Stream, EngineError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| EngineError::Audio("No default input device".into()))?;
        let mut supported_configs_range = device.supported_input_configs().map_err(|e| EngineError::Audio(e.to_string()))?;
        let supported_config = supported_configs_range
            .find(|cfg| cfg.min_sample_rate().0 <= self.sample_rate && cfg.max_sample_rate().0 >= self.sample_rate)
            .ok_or_else(|| EngineError::Audio("No supported input config for requested sample rate".into()))?;
        let config = supported_config.with_sample_rate(cpal::SampleRate(self.sample_rate)).config();

        let stream = device.build_input_stream(
            &config,
            move |_data: &[f32], _| {
                // In future: push to ring buffer
            },
            move |err| {
                eprintln!("audio input error: {err}");
            },
            None,
        ).map_err(|e| EngineError::Audio(e.to_string()))?;
        stream.play().map_err(|e| EngineError::Audio(e.to_string()))?;
        Ok(stream)
    }

    pub fn start_record_to_wav(&self, path: &str, duration_secs: u32) -> Result<(), EngineError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| EngineError::Audio("No default input device".into()))?;
        let mut supported_configs_range = device.supported_input_configs().map_err(|e| EngineError::Audio(e.to_string()))?;
        let supported_config = supported_configs_range
            .find(|cfg| cfg.min_sample_rate().0 <= self.sample_rate && cfg.max_sample_rate().0 >= self.sample_rate)
            .ok_or_else(|| EngineError::Audio("No supported input config for requested sample rate".into()))?;
        let config = supported_config.with_sample_rate(cpal::SampleRate(self.sample_rate)).config();

        let spec = hound::WavSpec {
            channels: config.channels as u16,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let writer = std::sync::Arc::new(std::sync::Mutex::new(WavWriter::create(path, spec).map_err(|e| EngineError::Audio(e.to_string()))?));

        let start = std::time::Instant::now();
        let writer_clone = writer.clone();
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                if let Ok(mut w) = writer_clone.lock() {
                    for &s in data {
                        let v = (s * i16::MAX as f32) as i16;
                        let _ = w.write_sample(v);
                    }
                }
            },
            move |err| {
                eprintln!("audio input error: {err}");
            },
            None,
        ).map_err(|e| EngineError::Audio(e.to_string()))?;
        stream.play().map_err(|e| EngineError::Audio(e.to_string()))?;

        // Block current thread for duration; in async context caller should spawn_blocking
        while start.elapsed().as_secs() < duration_secs as u64 {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        drop(stream);
        if let Ok(mut w) = writer.lock() {
            let _ = w.flush();
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    Timer { duration_secs: u64 },
    Unknown { text: String },
}

#[async_trait]
pub trait NluEngine: Send + Sync {
    async fn parse_intent(&self, text: &str) -> Intent;
}

#[async_trait]
pub trait Executor: Send + Sync {
    async fn execute(&self, intent: &Intent, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError>;
}

pub struct SessionManager<W: WakeDetector, A: AsrEngine, T: TtsEngine, N: NluEngine, E: Executor> {
    wake: W,
    asr: A,
    tts: T,
    nlu: N,
    exec: E,
}

impl<W: WakeDetector, A: AsrEngine, T: TtsEngine, N: NluEngine, E: Executor> SessionManager<W, A, T, N, E> {
    pub fn new(wake: W, asr: A, tts: T, nlu: N, exec: E) -> Self {
        Self { wake, asr, tts, nlu, exec }
    }

    #[instrument(skip(self))]
    pub async fn run_once(&self, event_tx: mpsc::Sender<EngineEvent>) -> Result<(), EngineError> {
        self.wake.wait_for_wake().await?;
        event_tx.send(EngineEvent::WakeDetected).await.map_err(|e| EngineError::Wake(e.to_string()))?;

        let final_text = self.asr.stream_until_silence(event_tx.clone()).await?;
        event_tx.send(EngineEvent::FinalTranscript(final_text.clone())).await.map_err(|e| EngineError::Asr(e.to_string()))?;

        let intent = self.nlu.parse_intent(&final_text).await;
        event_tx.send(EngineEvent::IntentRecognized(intent.clone())).await.map_err(|e| EngineError::Asr(e.to_string()))?;

        let speak_text = self.exec.execute(&intent, event_tx.clone()).await?;
        self.tts.speak(&speak_text, event_tx.clone()).await?;
        Ok(())
    }
}

// Mock implementations for CLI MVP
pub struct MockWake;
#[async_trait]
impl WakeDetector for MockWake {
    async fn wait_for_wake(&self) -> Result<(), EngineError> {
        sleep(Duration::from_millis(500)).await;
        Ok(())
    }
}

pub struct PorcupineWake {
    pub porcupine_bin: String,
    pub keyword_path: String,
    pub device_index: Option<i32>,
    pub sensitivity: Option<f32>,
}

#[async_trait]
impl WakeDetector for PorcupineWake {
    async fn wait_for_wake(&self) -> Result<(), EngineError> {
        let mut cmd = Command::new(&self.porcupine_bin);
        cmd.arg("--keyword_paths").arg(&self.keyword_path);
        if let Some(idx) = self.device_index {
            cmd.arg("--input_audio_device_index").arg(idx.to_string());
        }
        if let Some(s) = self.sensitivity {
            cmd.arg("--sensitivities").arg(format!("{}", s));
        }
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::time::timeout;
        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| EngineError::Wake(e.to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::Wake("No stdout from porcupine demo".to_string()))?;
        let mut lines = BufReader::new(stdout).lines();
        let deadline = Duration::from_secs(300);
        let res = timeout(deadline, async {
            while let Some(line) = lines.next_line().await.map_err(|e| EngineError::Wake(e.to_string()))? {
                if porcupine_line_has_detection(&line) {
                    return Ok(()) as Result<(), EngineError>;
                }
            }
            // Process ended; check exit status
            let status = child.wait().await.map_err(|e| EngineError::Wake(e.to_string()))?;
            if status.success() {
                Err(EngineError::Wake("Porcupine ended without detection".to_string()))
            } else {
                Err(EngineError::Wake(format!("Porcupine exited with status {status}")))
            }
        }).await;
        match res {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                let _ = child.kill().await; // best-effort
                Err(EngineError::Wake("Porcupine timed out without detection".to_string()))
            }
        }
    }
}

pub struct MockAsr;
#[async_trait]
impl AsrEngine for MockAsr {
    async fn stream_until_silence(&self, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        let partials = ["hello", "hello there", "hello there assistant"];
        for p in partials.iter() {
            events
                .send(EngineEvent::PartialTranscript(TranscriptFragment { text: p.to_string(), is_final: false }))
                .await
                .map_err(|e| EngineError::Asr(e.to_string()))?;
            sleep(Duration::from_millis(250)).await;
        }
        Ok("hello there assistant".to_string())
    }
}

pub struct WhisperAsr {
    pub whisper_bin: String,
    pub model_path: String,
    pub audio_wav: String,
}

#[async_trait]
impl AsrEngine for WhisperAsr {
    async fn stream_until_silence(&self, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        // For now, transcribe the audio file using whisper.cpp
        // In the real version, this will stream audio chunks and emit partials
        let mut cmd = Command::new(&self.whisper_bin);
        cmd.arg("-m").arg(&self.model_path)
            .arg("-f").arg(&self.audio_wav)
            .arg("--output-txt");
        
        let output = cmd.output().await.map_err(|e| EngineError::Asr(e.to_string()))?;
        
        if !output.status.success() {
            return Err(EngineError::Asr(format!("whisper failed: {}", String::from_utf8_lossy(&output.stderr))));
        }
        
        let transcript = String::from_utf8(output.stdout).map_err(|e| EngineError::Asr(e.to_string()))?;
        let transcript = transcript.trim();
        
        // Emit as a single final transcript for now
        events
            .send(EngineEvent::PartialTranscript(TranscriptFragment { text: transcript.to_string(), is_final: true }))
            .await
            .map_err(|e| EngineError::Asr(e.to_string()))?;
        
        Ok(transcript.to_string())
    }
}

pub struct MockTts;
#[async_trait]
impl TtsEngine for MockTts {
    async fn speak(&self, _text: &str, events: mpsc::Sender<EngineEvent>) -> Result<(), EngineError> {
        events.send(EngineEvent::TtsStarted).await.map_err(|e| EngineError::Tts(e.to_string()))?;
        sleep(Duration::from_millis(400)).await;
        events.send(EngineEvent::TtsFinished).await.map_err(|e| EngineError::Tts(e.to_string()))?;
        Ok(())
    }
}

pub struct PiperTts {
    pub piper_bin: String,
    pub model_path: String,
    pub output_wav: Option<String>,
}

#[async_trait]
impl TtsEngine for PiperTts {
    async fn speak(&self, text: &str, events: mpsc::Sender<EngineEvent>) -> Result<(), EngineError> {
        events.send(EngineEvent::TtsStarted).await.map_err(|e| EngineError::Tts(e.to_string()))?;
        let mut cmd = Command::new(&self.piper_bin);
        cmd.arg("--model").arg(&self.model_path)
            .arg("--sentence").arg(text);
        if let Some(path) = &self.output_wav {
            cmd.arg("--output_file").arg(path);
        }
        let status = cmd.status().await.map_err(|e| EngineError::Tts(e.to_string()))?;
        if !status.success() {
            return Err(EngineError::Tts(format!("piper exited with status {status}")));
        }
        events.send(EngineEvent::TtsFinished).await.map_err(|e| EngineError::Tts(e.to_string()))?;
        Ok(())
    }
}

// Simple NLU and Executor for CLI MVP
pub struct SimpleNlu;

#[async_trait]
impl NluEngine for SimpleNlu {
    async fn parse_intent(&self, text: &str) -> Intent {
        // very naive parsing: look for "timer" and a number + unit
        let lower = text.to_lowercase();
        if lower.contains("timer") || lower.contains("remind") {
            // extract first number
            let mut num: Option<u64> = None;
            for tok in lower.split_whitespace() {
                if let Ok(n) = tok.parse::<u64>() {
                    num = Some(n);
                    break;
                }
            }
            let seconds = if lower.contains("minute") {
                num.unwrap_or(1) * 60
            } else if lower.contains("second") {
                num.unwrap_or(10)
            } else {
                // default to seconds if unspecified
                num.unwrap_or(10)
            };
            return Intent::Timer { duration_secs: seconds };
        }
        Intent::Unknown { text: text.to_string() }
    }
}

pub struct SimpleExecutor;

#[async_trait]
impl Executor for SimpleExecutor {
    async fn execute(&self, intent: &Intent, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        match intent {
            Intent::Timer { duration_secs } => {
                let secs = *duration_secs;
                let msg = format!("Timer set for {} seconds", secs);
                events.send(EngineEvent::ExecutionStarted("timer".into())).await.map_err(|e| EngineError::Audio(e.to_string()))?;
                // Fire-and-forget notification after duration
                let events_clone = events.clone();
                tokio::spawn(async move {
                    sleep(Duration::from_secs(secs)).await;
                    let _ = events_clone.send(EngineEvent::Notification("Timer done".into())).await;
                    let _ = events_clone.send(EngineEvent::ExecutionFinished("timer".into())).await;
                });
                Ok(msg)
            }
            Intent::Unknown { .. } => Ok("Okay.".to_string()),
        }
    }
}

pub fn porcupine_line_has_detection(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    l.contains("detected") || l.contains("wake word detected") || l.contains("keyword detected")
}

pub fn simple_energy_vad(frame: &[f32], threshold: f32) -> bool {
    if frame.is_empty() { return false; }
    let mut sumsq = 0.0f32;
    for &s in frame {
        sumsq += s * s;
    }
    let rms = (sumsq / frame.len() as f32).sqrt();
    rms >= threshold
}

#[cfg(test)]
mod tests {
    use super::{porcupine_line_has_detection, simple_energy_vad};

    #[test]
    fn detects_generic_detected() {
        assert!(porcupine_line_has_detection("[0.123] detected"));
    }

    #[test]
    fn detects_phrase_variants() {
        assert!(porcupine_line_has_detection("Wake word detected"));
        assert!(porcupine_line_has_detection("KEYWORD DETECTED (index=0)"));
    }

    #[test]
    fn ignores_non_detection() {
        assert!(!porcupine_line_has_detection("listening..."));
        assert!(!porcupine_line_has_detection("noise level: -36dB"));
    }

    #[test]
    fn vad_detects_loud() {
        let frame = vec![0.2f32; 480];
        assert!(simple_energy_vad(&frame, 0.1));
    }

    #[test]
    fn vad_ignores_quiet() {
        let frame = vec![0.01f32; 480];
        assert!(!simple_energy_vad(&frame, 0.05));
    }
}


