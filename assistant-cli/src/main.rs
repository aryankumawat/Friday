use assistant_core::{EngineEvent, MockAsr, MockTts, MockWake, PiperTts, SessionManager, SimpleExecutor, SimpleNlu, WhisperAsr, PorcupineWake};
use clap::{Parser, ValueEnum};
use tokio::sync::mpsc;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum TtsKind { Mock, Piper }

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum AsrKind { Mock, Whisper }

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum WakeKind { Mock, Porcupine }

#[derive(Parser, Debug)]
#[command(author, version, about = "Assistant CLI mock loop", long_about = None)]
struct Args {
    /// Number of sessions to run before exit
    #[arg(short, long, default_value_t = 1)]
    sessions: u32,
    /// Wake word engine
    #[arg(long, value_enum, default_value_t = WakeKind::Mock)]
    wake: WakeKind,
    /// Path to porcupine binary (for porcupine wake)
    #[arg(long, default_value = "porcupine_demo_mic")]
    porcupine_bin: String,
    /// Path to porcupine keyword (.ppn) file (for porcupine wake)
    #[arg(long, default_value = "")]
    keyword_path: String,
    /// TTS engine to use
    #[arg(long, value_enum, default_value_t = TtsKind::Mock)]
    tts: TtsKind,
    /// ASR engine to use
    #[arg(long, value_enum, default_value_t = AsrKind::Mock)]
    asr: AsrKind,
    /// Path to piper binary
    #[arg(long, default_value = "piper")]
    piper_bin: String,
    /// Path to piper model (required for piper TTS)
    #[arg(long, default_value = "")]
    piper_model: String,
    /// Optional output wav path (if empty, piper will play to stdout/audio backend if configured)
    #[arg(long, default_value = "")] 
    piper_out: String,
    /// Path to whisper binary
    #[arg(long, default_value = "whisper")]
    whisper_bin: String,
    /// Path to whisper model (required for whisper ASR)
    #[arg(long, default_value = "")]
    whisper_model: String,
    /// Input audio file for whisper (required for whisper ASR)
    #[arg(long, default_value = "")]
    whisper_audio: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_max_level(Level::INFO)
        .init();

    let tts_engine: Box<dyn assistant_core::TtsEngine + Send + Sync> = match args.tts {
        TtsKind::Mock => Box::new(MockTts),
        TtsKind::Piper => {
            let out = if args.piper_out.is_empty() { None } else { Some(args.piper_out.clone()) };
            Box::new(PiperTts { piper_bin: args.piper_bin.clone(), model_path: args.piper_model.clone(), output_wav: out })
        }
    };

    // Adapters to allow dynamic engines with generic SessionManager
    struct TtsAdapter(Box<dyn assistant_core::TtsEngine + Send + Sync>);
    #[async_trait::async_trait]
    impl assistant_core::TtsEngine for TtsAdapter {
        async fn speak(&self, text: &str, events: mpsc::Sender<EngineEvent>) -> Result<(), assistant_core::EngineError> {
            self.0.speak(text, events).await
        }
    }

    struct AsrAdapter(Box<dyn assistant_core::AsrEngine + Send + Sync>);
    #[async_trait::async_trait]
    impl assistant_core::AsrEngine for AsrAdapter {
        async fn stream_until_silence(&self, events: mpsc::Sender<EngineEvent>) -> Result<String, assistant_core::EngineError> {
            self.0.stream_until_silence(events).await
        }
    }

    struct WakeAdapter(Box<dyn assistant_core::WakeDetector + Send + Sync>);
    #[async_trait::async_trait]
    impl assistant_core::WakeDetector for WakeAdapter {
        async fn wait_for_wake(&self) -> Result<(), assistant_core::EngineError> {
            self.0.wait_for_wake().await
        }
    }

    let asr_engine: Box<dyn assistant_core::AsrEngine + Send + Sync> = match args.asr {
        AsrKind::Mock => Box::new(MockAsr),
        AsrKind::Whisper => {
            Box::new(WhisperAsr {
                whisper_bin: args.whisper_bin.clone(),
                model_path: args.whisper_model.clone(),
                audio_wav: args.whisper_audio.clone(),
            })
        }
    };

    let wake_engine: Box<dyn assistant_core::WakeDetector + Send + Sync> = match args.wake {
        WakeKind::Mock => Box::new(MockWake),
        WakeKind::Porcupine => Box::new(PorcupineWake { porcupine_bin: args.porcupine_bin.clone(), keyword_path: args.keyword_path.clone() }),
    };

    let manager = SessionManager::new(WakeAdapter(wake_engine), AsrAdapter(asr_engine), TtsAdapter(tts_engine), SimpleNlu, SimpleExecutor);
    let (tx, mut rx) = mpsc::channel::<EngineEvent>(32);

    let ui = tokio::spawn(async move {
        while let Some(evt) = rx.recv().await {
            match evt {
                EngineEvent::WakeDetected => info!("Wake detected"),
                EngineEvent::PartialTranscript(p) => info!(partial = %p.text, "Partial"),
                EngineEvent::FinalTranscript(t) => info!(final_transcript = %t, "Final"),
                EngineEvent::IntentRecognized(i) => info!(intent = ?i, "Intent"),
                EngineEvent::ExecutionStarted(name) => info!(execution = %name, "Execution started"),
                EngineEvent::ExecutionFinished(name) => info!(execution = %name, "Execution finished"),
                EngineEvent::Notification(msg) => info!(notification = %msg, "Notify"),
                EngineEvent::TtsStarted => info!("TTS started"),
                EngineEvent::TtsFinished => info!("TTS finished"),
            }
        }
    });

    for _ in 0..args.sessions {
        if let Err(e) = manager.run_once(tx.clone()).await {
            eprintln!("session error: {e}");
        }
    }

    drop(tx);
    let _ = ui.await;
}


