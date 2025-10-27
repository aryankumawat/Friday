use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio::process::Command;
use tracing::{info, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptFragment {
    pub text: String,
    pub is_final: bool,
}

#[derive(Debug)]
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
    pub async fn run_once(&self, mut event_tx: mpsc::Sender<EngineEvent>) -> Result<(), EngineError> {
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
                let mut events_clone = events.clone();
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


