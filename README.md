# Friday Assistant

Friday is a Rust workspace for a local, privacy-first voice assistant. It bundles a core library, a CLI, and a desktop UI shell, demonstrating an end-to-end pipeline: wake word detection → automatic speech recognition (ASR) → intent handling → text-to-speech (TTS).

## Features

- Offline-first architecture with optional integrations
- pluggable engines for Wake, ASR, and TTS (mock and real)
- event-driven session manager with clear boundaries and traits
- CLI for fast iteration and debugging
- Tauri-based desktop shell (WIP) for visual feedback and controls

## Workspace Layout

- `assistant-core`: core traits, session manager, mock engines, and plumbing
- `assistant-cli`: CLI that runs a sample session and logs events
- `assistant-ui`: Tauri-based UI shell (dev preview)

## Getting Started

Prerequisites:
- Rust toolchain via `rustup`

Run the sample pipeline using the CLI (mock engines):

```bash
cargo run -p assistant-cli -- --sessions 2
```

For a more visual flow, you can emit UI-friendly events:

```bash
cargo run -p assistant-cli -- --ui-events
```

Run a quick environment health check:

```bash
cargo run -p assistant-cli -- doctor
# Add flags to verify real engines, e.g.:
cargo run -p assistant-cli -- doctor \
  --piper-bin ./piper_macos_arm64 --piper-model ./en_US-lessac-medium.onnx \
  --whisper-bin ./whisper.cpp/whisper --whisper-model ./whisper.cpp/models/ggml-base.bin \
  --porcupine-bin ./porcupine_demo_mic --keyword-path /path/to/hey-friday.ppn
```

## Optional Integrations

Friday supports real engines in addition to the mock pipeline.

### Piper TTS

Install `piper` and a voice model, then:

```bash
cargo run -p assistant-cli -- \
  --tts piper \
  --piper-bin piper \
  --piper-model /path/to/voice.onnx \
  --piper-out /tmp/out.wav
```

Omit `--piper-out` to allow Piper to handle system audio output if configured.

### Whisper ASR

Build `whisper.cpp` and download a model, then:

```bash
cargo run -p assistant-cli -- \
  --asr whisper \
  --whisper-bin ./whisper \
  --whisper-model ./models/ggml-base.bin \
  --whisper-audio /path/to/input.wav
```

### Porcupine Wake Word

Porcupine is a lightweight, offline, commercial-friendly wake-word engine. Create a free account and download your keyword from Picovoice Console (`https://console.picovoice.ai/`).

1. Download `porcupine_demo_mic` for your OS from `https://github.com/Picovoice/porcupine/tree/master/demo/python` or install via `pip install pvporcupine`.
2. Download the `.ppn` file for your keyword (e.g., `hey-friday_en_raspberry-pi_v2_2_0.ppn`).
3. Verify it manually:
   ```bash
   ./porcupine_demo_mic --input_audio_device_index 0 --keyword_paths /path/to/hey-friday.ppn
   ```
4. Run the Friday CLI with Porcupine:
```bash
cargo run -p assistant-cli -- \
  --wake porcupine \
  --porcupine-bin ./porcupine_demo_mic \
  --keyword-path /path/to/hey-friday.ppn
```

You should see logs for wake detection, partial/final transcripts, and TTS.

## Tauri UI (Dev Preview)

The UI crate (`assistant-ui`) provides a Tauri shell for visualization and controls.

```bash
cargo tauri dev
```

This launches a desktop window that can display pipeline events and basic controls.

## Roadmap

Planned work includes real-time microphone capture, robust wake-word detection, streaming Whisper integration, richer intents, and multi-turn dialogue management. See `NEXT_STEPS.md` for a detailed plan.

## Mock Intents

- Timer: phrases like "set a timer for 10 seconds/minutes" → schedules a notification and speaks a confirmation.

## License

This repository is provided under a permissive open-source license. Review `LICENSE` if present, or set one appropriate for your use case.
