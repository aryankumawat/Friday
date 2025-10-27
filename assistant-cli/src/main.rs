use assistant_core::{EngineEvent, MockAsr, MockTts, MockWake, SessionManager};
use clap::Parser;
use tokio::sync::mpsc;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about = "Assistant CLI mock loop", long_about = None)]
struct Args {
    /// Number of sessions to run before exit
    #[arg(short, long, default_value_t = 1)]
    sessions: u32,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_max_level(Level::INFO)
        .init();

    let manager = SessionManager::new(MockWake, MockAsr, MockTts);
    let (tx, mut rx) = mpsc::channel::<EngineEvent>(32);

    let ui = tokio::spawn(async move {
        while let Some(evt) = rx.recv().await {
            match evt {
                EngineEvent::WakeDetected => info!("Wake detected"),
                EngineEvent::PartialTranscript(p) => info!(partial = %p.text, "Partial"),
                EngineEvent::FinalTranscript(t) => info!(final_transcript = %t, "Final"),
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


