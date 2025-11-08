use assistant_core::{EngineEvent, MockAsr, MockTts, MockWake, PiperTts, SessionManager, SimpleExecutor, SimpleNlu, WhisperAsr, PorcupineWake, AudioCapture, AsrEngine};
use assistant_core::realtime_wake::{RealtimeWake, EnergyWake};
use assistant_core::audio_wake::AudioWake;
use assistant_core::enhanced_nlu::EnhancedNlu;
use assistant_core::enhanced_executor::EnhancedExecutor;
use assistant_core::config::FridayConfig;
use assistant_core::streaming_asr::StreamingAsr;
use assistant_core::plugin_executor::{PluginExecutor, PluginExecutorBuilder};
use clap::{Parser, ValueEnum, Subcommand};
use tokio::sync::mpsc;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum TtsKind { Mock, Piper }

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum AsrKind { Mock, Whisper, Streaming }

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum WakeKind { Mock, Porcupine, Realtime, Energy }

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum NluKind { Simple, Enhanced }

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ExecutorKind { Simple, Enhanced, Plugin }

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Environment checks for binaries, models and audio devices
    Doctor,
    /// List input audio devices
    Devices,
    /// Record microphone to WAV
    Record {
        /// Output WAV path
        #[arg(long, default_value = "./out.wav")]
        out: String,
        /// Duration seconds
        #[arg(long, default_value_t = 5)]
        seconds: u32,
        /// Sample rate
        #[arg(long, default_value_t = 16_000)]
        sample_rate: u32,
    },
    /// End-to-end: wait for wake → record → whisper → TTS
    Wakeflow {
        /// Recording duration after wake (seconds)
        #[arg(long, default_value_t = 4)]
        seconds: u32,
        /// Sample rate
        #[arg(long, default_value_t = 16_000)]
        sample_rate: u32,
        /// Output wav path (optional; default temp)
        #[arg(long, default_value = "")]
        out: String,
    },
    /// Run assistant (default)
    Run,
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Plugin management
    Plugins {
        #[command(subcommand)]
        action: PluginAction,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// List available profiles
    Profiles,
    /// Set active profile
    SetProfile { name: String },
    /// Create default configuration file
    Init,
    /// Validate configuration
    Validate,
}

#[derive(Subcommand, Debug)]
enum PluginAction {
    /// List available plugins
    List,
    /// Show plugin information
    Info { name: String },
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Assistant CLI mock loop", long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Number of sessions to run before exit
    #[arg(short, long, default_value_t = 1)]
    sessions: u32,
    /// Emit EngineEvent as JSON on stdout for UI integration
    #[arg(long, default_value_t = false)]
    ui_events: bool,
    /// Start microphone capture during run
    #[arg(long, default_value_t = false)]
    capture: bool,
    /// Capture sample rate
    #[arg(long, default_value_t = 16_000)]
    sample_rate: u32,
    /// Wake word engine
    #[arg(long, value_enum, default_value_t = WakeKind::Mock)]
    wake: WakeKind,
    /// NLU engine to use
    #[arg(long, value_enum, default_value_t = NluKind::Enhanced)]
    nlu: NluKind,
    /// Executor engine to use
    #[arg(long, value_enum, default_value_t = ExecutorKind::Plugin)]
    executor: ExecutorKind,
    /// Path to porcupine binary (for porcupine wake)
    #[arg(long, default_value = "porcupine_demo_mic")]
    porcupine_bin: String,
    /// Path to porcupine keyword (.ppn) file (for porcupine wake)
    #[arg(long, default_value = "")]
    keyword_path: String,
    /// Porcupine input device index
    #[arg(long)]
    porcupine_device_index: Option<i32>,
    /// Porcupine sensitivity (0.0-1.0)
    #[arg(long)]
    porcupine_sensitivity: Option<f32>,
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
    /// Optional path to a JSON config with default args
    #[arg(long, default_value = "")]
    config: String,
}

#[tokio::main]
async fn main() {
    let mut args = Args::parse();
    // Load JSON config (if provided) and fill missing values
    if !args.config.is_empty() {
        if let Ok(cfg_text) = std::fs::read_to_string(&args.config) {
            #[derive(serde::Deserialize)]
            struct Cfg {
                sessions: Option<u32>,
                ui_events: Option<bool>,
                capture: Option<bool>,
                sample_rate: Option<u32>,
                wake: Option<String>,
                porcupine_bin: Option<String>,
                keyword_path: Option<String>,
                porcupine_device_index: Option<i32>,
                porcupine_sensitivity: Option<f32>,
                tts: Option<String>,
                asr: Option<String>,
                piper_bin: Option<String>,
                piper_model: Option<String>,
                piper_out: Option<String>,
                whisper_bin: Option<String>,
                whisper_model: Option<String>,
                whisper_audio: Option<String>,
            }
            if let Ok(cfg) = serde_json::from_str::<Cfg>(&cfg_text) {
                if let Some(v) = cfg.sessions { if args.sessions == 1 { args.sessions = v; } }
                if let Some(v) = cfg.ui_events { if !args.ui_events { args.ui_events = v; } }
                if let Some(v) = cfg.capture { if !args.capture { args.capture = v; } }
                if let Some(v) = cfg.sample_rate { if args.sample_rate == 16_000 { args.sample_rate = v; } }
                if let Some(v) = cfg.wake { if args.wake == WakeKind::Mock { args.wake = match v.as_str() { "porcupine" => WakeKind::Porcupine, _ => WakeKind::Mock } } }
                if let Some(v) = cfg.porcupine_bin { if args.porcupine_bin == "porcupine_demo_mic" { args.porcupine_bin = v; } }
                if let Some(v) = cfg.keyword_path { if args.keyword_path.is_empty() { args.keyword_path = v; } }
                if let Some(v) = cfg.porcupine_device_index { if args.porcupine_device_index.is_none() { args.porcupine_device_index = Some(v); } }
                if let Some(v) = cfg.porcupine_sensitivity { if args.porcupine_sensitivity.is_none() { args.porcupine_sensitivity = Some(v); } }
                if let Some(v) = cfg.tts { if args.tts == TtsKind::Mock { args.tts = match v.as_str() { "piper" => TtsKind::Piper, _ => TtsKind::Mock } } }
                if let Some(v) = cfg.asr { if args.asr == AsrKind::Mock { args.asr = match v.as_str() { "whisper" => AsrKind::Whisper, _ => AsrKind::Mock } } }
                if let Some(v) = cfg.piper_bin { if args.piper_bin == "piper" { args.piper_bin = v; } }
                if let Some(v) = cfg.piper_model { if args.piper_model.is_empty() { args.piper_model = v; } }
                if let Some(v) = cfg.piper_out { if args.piper_out.is_empty() { args.piper_out = v; } }
                if let Some(v) = cfg.whisper_bin { if args.whisper_bin == "whisper" { args.whisper_bin = v; } }
                if let Some(v) = cfg.whisper_model { if args.whisper_model.is_empty() { args.whisper_model = v; } }
                if let Some(v) = cfg.whisper_audio { if args.whisper_audio.is_empty() { args.whisper_audio = v; } }
            }
        }
    }
    match &args.cmd {
        Some(Cmd::Doctor) => {
            // Basic checks for external deps and system devices
            println!("Friday Doctor\n================");
            // Audio input device
            let devices = AudioCapture::list_input_devices();
            if devices.is_empty() { println!("[x] No input devices found"); } else { println!("[ok] {} input devices detected", devices.len()); }
            // Piper
            if !args.piper_model.is_empty() {
                let status = tokio::process::Command::new(&args.piper_bin)
                    .arg("--help")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status().await;
                match status { Ok(s) if s.success() || s.code().is_some() => println!("[ok] Piper binary reachable: {}", args.piper_bin), _ => println!("[x] Piper binary not found or not runnable: {}", args.piper_bin) }
                if std::path::Path::new(&args.piper_model).exists() { println!("[ok] Piper model present: {}", args.piper_model); } else { println!("[x] Piper model missing: {}", args.piper_model); }
            } else {
                println!("[i] Piper not configured (set --piper-bin/--piper-model)");
            }
            // Whisper
            if !args.whisper_model.is_empty() {
                let status = tokio::process::Command::new(&args.whisper_bin)
                    .arg("-h")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status().await;
                match status { Ok(s) if s.success() || s.code().is_some() => println!("[ok] Whisper binary reachable: {}", args.whisper_bin), _ => println!("[x] Whisper binary not found or not runnable: {}", args.whisper_bin) }
                if std::path::Path::new(&args.whisper_model).exists() { println!("[ok] Whisper model present: {}", args.whisper_model); } else { println!("[x] Whisper model missing: {}", args.whisper_model); }
                if !args.whisper_audio.is_empty() {
                    if std::path::Path::new(&args.whisper_audio).exists() { println!("[ok] Whisper input audio present: {}", args.whisper_audio); } else { println!("[x] Whisper input audio missing: {}", args.whisper_audio); }
                } else {
                    println!("[i] Whisper input audio not set (--whisper-audio)");
                }
            } else {
                println!("[i] Whisper not configured (set --whisper-bin/--whisper-model)");
            }
            // Porcupine
            if !args.keyword_path.is_empty() {
                let status = tokio::process::Command::new(&args.porcupine_bin)
                    .arg("--help")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status().await;
                match status { Ok(s) if s.success() || s.code().is_some() => println!("[ok] Porcupine binary reachable: {}", args.porcupine_bin), _ => println!("[x] Porcupine binary not found or not runnable: {}", args.porcupine_bin) }
                if std::path::Path::new(&args.keyword_path).exists() { println!("[ok] Porcupine keyword present: {}", args.keyword_path); } else { println!("[x] Porcupine keyword missing: {}", args.keyword_path); }
            } else {
                println!("[i] Porcupine not configured (set --porcupine-bin/--keyword-path)");
            }
            return;
        }
        Some(Cmd::Config { action }) => {
            match action {
                ConfigAction::Show => {
                    match FridayConfig::load_default() {
                        Ok(config) => {
                            println!("{}", serde_json::to_string_pretty(&config).unwrap());
                        }
                        Err(e) => {
                            eprintln!("Failed to load configuration: {}", e);
                        }
                    }
                }
                ConfigAction::Profiles => {
                    match FridayConfig::load_default() {
                        Ok(config) => {
                            println!("Available profiles:");
                            for profile in config.list_profiles() {
                                let active = if config.active_profile.as_ref() == Some(&profile.name) { " (active)" } else { "" };
                                println!("  {} - {}{}", profile.name, profile.description, active);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to load configuration: {}", e);
                        }
                    }
                }
                ConfigAction::SetProfile { name } => {
                    match FridayConfig::load_default() {
                        Ok(mut config) => {
                            match config.apply_profile(name) {
                                Ok(_) => {
                                    match config.save_default() {
                                        Ok(_) => println!("Profile '{}' activated", name),
                                        Err(e) => eprintln!("Failed to save configuration: {}", e),
                                    }
                                }
                                Err(e) => eprintln!("Failed to set profile: {}", e),
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to load configuration: {}", e);
                        }
                    }
                }
                ConfigAction::Init => {
                    let config = FridayConfig::default();
                    match config.save_default() {
                        Ok(_) => {
                            if let Ok(path) = FridayConfig::default_config_path() {
                                println!("Configuration initialized at: {}", path.display());
                            } else {
                                println!("Configuration initialized");
                            }
                        }
                        Err(e) => eprintln!("Failed to initialize configuration: {}", e),
                    }
                }
                ConfigAction::Validate => {
                    match FridayConfig::load_default() {
                        Ok(config) => {
                            match config.validate() {
                                Ok(_) => println!("Configuration is valid"),
                                Err(e) => eprintln!("Configuration validation failed: {}", e),
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to load configuration: {}", e);
                        }
                    }
                }
            }
            return;
        }
        Some(Cmd::Plugins { action }) => {
            match action {
                PluginAction::List => {
                    let mut plugin_executor = PluginExecutor::new(SimpleExecutor);
                    match plugin_executor.initialize().await {
                        Ok(_) => {
                            println!("Available plugins:");
                            let plugins = plugin_executor.list_plugins();
                            if plugins.is_empty() {
                                println!("  No plugins loaded");
                            } else {
                                for plugin in plugins {
                                    println!("  {}", plugin);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to initialize plugin system: {}", e);
                        }
                    }
                }
                PluginAction::Info { name } => {
                    println!("Plugin information for: {}", name);
                    println!("  (Plugin info command not yet implemented)");
                }
            }
            return;
        }
        Some(Cmd::Devices) => {
            for (i, name) in AudioCapture::list_input_devices().iter().enumerate() {
                println!("{}: {}", i, name);
            }
            return;
        }
        Some(Cmd::Record { out, seconds, sample_rate }) => {
            let cap = AudioCapture::new(*sample_rate);
            if let Err(e) = cap.start_record_to_wav(out, *seconds) {
                eprintln!("record failed: {e}");
            } else {
                println!("recorded {}s to {}", seconds, out);
            }
            return;
        }
        Some(Cmd::Wakeflow { seconds, sample_rate, out }) => {
            // Build wake
            let wake: Box<dyn assistant_core::WakeDetector + Send + Sync> = match args.wake {
                WakeKind::Mock => Box::new(MockWake),
                WakeKind::Porcupine => Box::new(PorcupineWake {
                    porcupine_bin: args.porcupine_bin.clone(),
                    keyword_path: args.keyword_path.clone(),
                    device_index: args.porcupine_device_index,
                    sensitivity: args.porcupine_sensitivity,
                }),
                WakeKind::Realtime => Box::new(RealtimeWake::new()),
                WakeKind::Energy => Box::new(EnergyWake::new()),
            };
            println!("Waiting for wake...");
            if let Err(e) = wake.wait_for_wake().await { eprintln!("wake error: {e}"); return; }

            // Record
            let cap = AudioCapture::new(*sample_rate);
            let out_path = if out.is_empty() { "./wakeflow.wav".to_string() } else { out.clone() };
            println!("Recording {}s to {}", seconds, out_path);
            let rec_res = tokio::task::spawn_blocking({
                let out_path = out_path.clone(); let seconds = *seconds; move || {
                    cap.start_record_to_wav(&out_path, seconds)
                }
            }).await;
            match rec_res {
                Ok(Ok(())) => {},
                Ok(Err(e)) => { eprintln!("record failed: {e}"); return; },
                Err(e) => { eprintln!("join error: {e}"); return; },
            }

            // ASR (requires whisper)
            if args.asr != AsrKind::Whisper || args.whisper_model.is_empty() {
                eprintln!("whisper ASR not configured; provide --asr whisper --whisper-model <path>");
                return;
            }
            let (tx, _rx) = mpsc::channel::<EngineEvent>(8);
            let asr = WhisperAsr { whisper_bin: args.whisper_bin.clone(), model_path: args.whisper_model.clone(), audio_wav: out_path.clone() };
            match asr.stream_until_silence(tx).await {
                Ok(text) => {
                    println!("Transcript: {}", text);
                    // TTS
                    let tts: Box<dyn assistant_core::TtsEngine + Send + Sync> = match args.tts {
                        TtsKind::Mock => Box::new(MockTts),
                        TtsKind::Piper => {
                            let out = if args.piper_out.is_empty() { None } else { Some(args.piper_out.clone()) };
                            Box::new(PiperTts { piper_bin: args.piper_bin.clone(), model_path: args.piper_model.clone(), output_wav: out })
                        }
                    };
                    let (etx, _erx) = mpsc::channel::<EngineEvent>(8);
                    let _ = tts.speak(&format!("You said: {}", text), etx).await;
                }
                Err(e) => eprintln!("asr error: {e}"),
            }
            return;
        }
        _ => {}
    }

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
        AsrKind::Streaming => {
            let mut streaming_asr = StreamingAsr::new();
            if !args.whisper_model.is_empty() && !args.whisper_bin.is_empty() {
                streaming_asr = streaming_asr.with_whisper(args.whisper_bin.clone(), args.whisper_model.clone());
            }
            Box::new(streaming_asr)
        }
    };

    let wake_engine: Box<dyn assistant_core::WakeDetector + Send + Sync> = match args.wake {
        WakeKind::Mock => Box::new(MockWake),
        WakeKind::Porcupine => Box::new(PorcupineWake {
            porcupine_bin: args.porcupine_bin.clone(),
            keyword_path: args.keyword_path.clone(),
            device_index: args.porcupine_device_index,
            sensitivity: args.porcupine_sensitivity,
        }),
        WakeKind::Realtime => {
            // Use real audio-based wake detection
            Box::new(AudioWake::new()
                .with_threshold(0.01) // Very sensitive for voice detection
                .with_trigger_duration(200)) // 200ms of sustained sound
        },
        WakeKind::Energy => {
            // Use real audio-based wake detection - VERY SENSITIVE
            Box::new(AudioWake::new()
                .with_threshold(0.02) // Much lower threshold - easier to trigger
                .with_trigger_duration(150)) // Only 150ms needed!
        },
    };

    // Optionally start audio capture and keep stream alive
    let _stream_guard = if args.capture {
        let cap = AudioCapture::new(args.sample_rate);
        match cap.start_logging_input() {
            Ok(stream) => {
                info!("audio capture started at {} Hz", args.sample_rate);
                Some(stream)
            }
            Err(e) => {
                eprintln!("failed to start audio capture: {e}");
                None
            }
        }
    } else { None };

    // Create NLU engine
    let nlu_engine: Box<dyn assistant_core::NluEngine + Send + Sync> = match args.nlu {
        NluKind::Simple => Box::new(SimpleNlu),
        NluKind::Enhanced => Box::new(EnhancedNlu::new()),
    };

    struct NluAdapter(Box<dyn assistant_core::NluEngine + Send + Sync>);
    #[async_trait::async_trait]
    impl assistant_core::NluEngine for NluAdapter {
        async fn parse_intent(&self, text: &str) -> assistant_core::Intent {
            self.0.parse_intent(text).await
        }
    }

    // Create Executor engine
    let executor_engine: Box<dyn assistant_core::Executor + Send + Sync> = match args.executor {
        ExecutorKind::Simple => Box::new(SimpleExecutor),
        ExecutorKind::Enhanced => Box::new(EnhancedExecutor::new()),
        ExecutorKind::Plugin => {
            let mut plugin_executor = PluginExecutor::new(EnhancedExecutor::new());
            if let Err(e) = plugin_executor.initialize().await {
                eprintln!("Failed to initialize plugin executor: {}", e);
                Box::new(EnhancedExecutor::new())
            } else {
                Box::new(plugin_executor)
            }
        }
    };

    struct ExecutorAdapter(Box<dyn assistant_core::Executor + Send + Sync>);
    #[async_trait::async_trait]
    impl assistant_core::Executor for ExecutorAdapter {
        async fn execute(&self, intent: &assistant_core::Intent, events: mpsc::Sender<EngineEvent>) -> Result<String, assistant_core::EngineError> {
            self.0.execute(intent, events).await
        }
    }

    let manager = SessionManager::new(WakeAdapter(wake_engine), AsrAdapter(asr_engine), TtsAdapter(tts_engine), NluAdapter(nlu_engine), ExecutorAdapter(executor_engine));
    let (tx, mut rx) = mpsc::channel::<EngineEvent>(32);

    let emit_json = args.ui_events;
    let ui = tokio::spawn(async move {
        while let Some(evt) = rx.recv().await {
            if emit_json {
                if let Ok(s) = serde_json::to_string(&evt) { println!("{}", s); }
                continue;
            }
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


