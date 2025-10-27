use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
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

pub struct SessionManager<W: WakeDetector, A: AsrEngine, T: TtsEngine> {
    wake: W,
    asr: A,
    tts: T,
}

impl<W: WakeDetector, A: AsrEngine, T: TtsEngine> SessionManager<W, A, T> {
    pub fn new(wake: W, asr: A, tts: T) -> Self {
        Self { wake, asr, tts }
    }

    #[instrument(skip(self))]
    pub async fn run_once(&self, mut event_tx: mpsc::Sender<EngineEvent>) -> Result<(), EngineError> {
        self.wake.wait_for_wake().await?;
        event_tx.send(EngineEvent::WakeDetected).await.map_err(|e| EngineError::Wake(e.to_string()))?;

        let final_text = self.asr.stream_until_silence(event_tx.clone()).await?;
        event_tx.send(EngineEvent::FinalTranscript(final_text.clone())).await.map_err(|e| EngineError::Asr(e.to_string()))?;

        self.tts.speak(&format!("You said: {}", final_text), event_tx.clone()).await?;
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


