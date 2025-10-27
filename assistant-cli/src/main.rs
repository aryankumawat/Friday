use assistant_core::{EngineEvent, MockAsr, MockTts, MockWake, PiperTts, SessionManager, SimpleExecutor, SimpleNlu};
use clap::{Parser, ValueEnum};
use tokio::sync::mpsc;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum TtsKind { Mock, Piper }

#[derive(Parser, Debug)]
#[command(author, version, about = "Assistant CLI mock loop", long_about = None)]
struct Args {
    /// Number of sessions to run before exit
    #[arg(short, long, default_value_t = 1)]
    sessions: u32,
    /// TTS engine to use
    #[arg(long, value_enum, default_value_t = TtsKind::Mock)]
    tts: TtsKind,
    /// Path to piper binary
    #[arg(long, default_value = "piper")]
    piper_bin: String,
    /// Path to piper model (required for piper TTS)
    #[arg(long, default_value = "")]
    piper_model: String,
    /// Optional output wav path (if empty, piper will play to stdout/audio backend if configured)
    #[arg(long, default_value = "")] 
    piper_out: String,
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

    // SessionManager is generic; to keep using it, we wrap TTS in an enum-like via dynamic dispatch adapter.
    // For simplicity, use Piper or Mock via a small adapter struct.
    struct TtsAdapter(Box<dyn assistant_core::TtsEngine + Send + Sync>);
    #[async_trait::async_trait]
    impl assistant_core::TtsEngine for TtsAdapter {
        async fn speak(&self, text: &str, events: mpsc::Sender<EngineEvent>) -> Result<(), assistant_core::EngineError> {
            self.0.speak(text, events).await
        }
    }

    let manager = SessionManager::new(MockWake, MockAsr, TtsAdapter(tts_engine), SimpleNlu, SimpleExecutor);
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


