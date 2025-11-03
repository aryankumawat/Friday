# Friday Assistant

Friday is a Rust workspace for a local, privacy-first voice assistant. It bundles a core library, a CLI, and a desktop UI shell, demonstrating an end-to-end pipeline: wake word detection → automatic speech recognition (ASR) → intent handling → text-to-speech (TTS).

## Features

- Offline-first architecture with optional integrations
- Pluggable engines for Wake, ASR, and TTS (mock and real)
- Event-driven session manager with clear boundaries and traits
- CLI for fast iteration and debugging
- Tauri-based desktop shell (WIP) for visual feedback and controls
- Comprehensive build system with Make and Just support
- Automated development environment setup
- CI/CD pipeline with GitHub Actions

## Workspace Layout

- `assistant-core`: core traits, session manager, mock engines, and plumbing
- `assistant-cli`: CLI that runs a sample session and logs events
- `assistant-ui`: Tauri-based UI shell (dev preview)
- `scripts/`: development and setup automation scripts
- `.github/workflows/`: CI/CD pipeline configuration

## Quick Start

### Prerequisites
- Rust toolchain via `rustup`
- Make (optional, for build shortcuts)
- Just (optional, alternative build runner)

### Automated Setup
Run the setup script to install dependencies and configure the environment:

```bash
./scripts/setup.sh
```

### Build System Commands

Using Make:
```bash
make help         # Show all available targets
make dev          # Run CLI assistant with mock engines
make dev-ui       # Run Tauri UI in development mode
make build        # Build all workspace crates
make release      # Build optimized release version
make test         # Run all tests
make check        # Run cargo check on all crates
make format       # Format code with rustfmt
make lint         # Run clippy linter
make doctor       # Run health check
make setup        # Setup Whisper.cpp and Piper TTS
make setup-whisper # Setup Whisper.cpp only
make setup-piper  # Setup Piper TTS only
make clean        # Clean build artifacts
```

Using Just (alternative):
```bash
just --list       # Show all available recipes
just dev          # Run CLI assistant
just dev-ui       # Run Tauri UI
just build        # Build all crates
just test         # Run tests
just doctor       # Health check
```

Using Cargo directly:
```bash
cargo run -p assistant-cli -- --ui-events --sessions 2
```

### Health Check
Verify your environment:

```bash
make doctor
# Output: Friday Doctor
# [ok] 3 input devices detected
# [i] Piper not configured (set --piper-bin/--piper-model)
# [i] Whisper not configured (set --whisper-bin/--whisper-model)
# [i] Porcupine not configured (set --porcupine-bin/--keyword-path)
```

### Quick Development Guide

1. **First time setup:**
   ```bash
   git clone <your-repo>
   cd Friday
   ./scripts/setup.sh  # Installs dependencies and engines
   ```

2. **Daily development:**
   ```bash
   make dev           # Test with mock engines
   make test          # Run test suite
   make lint          # Check code quality
   ```

3. **Testing real engines:**
   ```bash
   make setup-piper   # Download TTS engine
   make setup-whisper # Build ASR engine
   make doctor        # Verify setup
   ```

## Development Workflow

### Mock Development (Recommended for Testing)
The mock engines provide a complete pipeline simulation:

```bash
make dev
# Shows: WakeDetected → PartialTranscript → FinalTranscript → IntentRecognized → TtsStarted → TtsFinished
```

### Real Engine Integration

Friday supports real engines in addition to the mock pipeline. Use the setup commands to install them:

```bash
make setup-piper    # Download Piper TTS
make setup-whisper  # Build Whisper.cpp
```

#### Piper TTS
After setup, test with:

```bash
cargo run -p assistant-cli -- \
  --tts piper \
  --piper-bin ./piper/piper \
  --piper-model ./en_US-lessac-medium.onnx \
  --sessions 1
```

#### Whisper ASR
After building Whisper.cpp:

```bash
cargo run -p assistant-cli -- \
  --asr whisper \
  --whisper-bin ./whisper.cpp/whisper \
  --whisper-model ./whisper.cpp/models/ggml-base.bin \
  --whisper-audio /path/to/input.wav
```

#### Porcupine Wake Word
For real wake word detection:

1. Create account at [Picovoice Console](https://console.picovoice.ai/)
2. Download your custom keyword `.ppn` file
3. Install porcupine demo: `pip install pvporcupine`
4. Run with wake word:

```bash
cargo run -p assistant-cli -- \
  --wake porcupine \
  --porcupine-bin porcupine_demo_mic \
  --keyword-path /path/to/hey-friday.ppn
```

## Tauri UI (Dev Preview)

The UI crate provides a desktop interface:

```bash
make dev-ui
# or
cargo tauri dev
```

This launches a desktop window for visual pipeline monitoring and controls.

## Build Configuration

### Cargo Configuration
- Optimized development builds in `.cargo/config.toml`
- Custom profiles for different use cases
- Cross-platform compilation settings

### CI/CD Pipeline
- GitHub Actions workflow in `.github/workflows/ci.yml`
- Multi-platform testing (Ubuntu, Windows, macOS)
- Automated formatting and linting checks

### Development Scripts
- `scripts/setup.sh`: Automated environment setup
- `scripts/dev.sh`: Development helper with multiple modes
- Makefile and Justfile for build automation

## Project Structure

```
Friday/
├── assistant-core/     # Core library and traits
├── assistant-cli/      # Command-line interface
├── assistant-ui/       # Tauri desktop application
├── scripts/           # Development automation
├── .github/           # CI/CD workflows
├── .cargo/            # Cargo configuration
├── piper/             # Piper TTS binary and data
├── whisper.cpp/       # Whisper ASR (git submodule)
├── Makefile           # Build automation
├── justfile           # Alternative build runner
└── rust-toolchain.toml # Rust version specification
```

## Mock Intents

Current intent recognition supports:
- Timer: "set a timer for 10 seconds/minutes" → schedules notification
- Unknown: fallback for unrecognized phrases → default response

## Roadmap

See `NEXT_STEPS.md` for detailed development plan including:
- Real-time microphone capture
- Streaming Whisper integration  
- Enhanced intent recognition
- Multi-turn dialogue management
- Plugin system architecture

## Contributing

1. Fork the repository
2. Run `./scripts/setup.sh` to configure development environment
3. Use `make dev` for testing changes
4. Run `make test` and `make lint` before submitting
5. Follow the CI pipeline requirements

## License

This repository is provided under a permissive open-source license. Review `LICENSE` if present, or set one appropriate for your use case.
