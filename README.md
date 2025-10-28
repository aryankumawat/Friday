# Friday Assistant (Scaffold)

Rust workspace with a core library and CLI showcasing a mock wake → ASR → TTS loop.

## Structure

- `assistant-core`: core traits and a session manager with mock engines
- `assistant-cli`: CLI that runs a sample session and logs events

## Build & Run

Ensure Rust toolchain is installed (`rustup`), then:

```bash
cargo run -p assistant-cli -- --sessions 2
```
### Piper TTS (optional)

Install `piper` and a voice model, then run:

```bash
cargo run -p assistant-cli -- --tts piper --piper-bin piper --piper-model /path/to/voice.onnx --piper-out /tmp/out.wav
```

Omit `--piper-out` to let Piper handle audio output if configured.

### Whisper ASR (optional)

Build `whisper.cpp` and download a model, then run:

```bash
cargo run -p assistant-cli -- --asr whisper --whisper-bin ./whisper --whisper-model ./models/ggml-base.bin --whisper-audio /path/to/input.wav
```

### Porcupine Wake Word (optional)

Porcupine is a lightweight, offline, commercial-friendly wake word/keyword engine. Register for a free account and download your target keyword from Picovoice Console: https://console.picovoice.ai/

1. Download `porcupine_demo_mic` for your OS from https://github.com/Picovoice/porcupine/tree/master/demo/python or install via `pip install pvporcupine` (for advanced use).
2. Download the .ppn file(s) for your keyword (e.g., `hey-friday_en_raspberry-pi_v2_2_0.ppn`).
3. Test it manually: `./porcupine_demo_mic --input_audio_device_index 0 --keyword_paths /path/to/hey-friday.ppn`.
4. See CLI usage below to integrate with this project. The CLI expects `--wake porcupine --porcupine-bin <bin-path> --keyword-path <ppn-path>`

Example:

```bash
cargo run -p assistant-cli -- \
  --wake porcupine \
  --porcupine-bin ./porcupine_demo_mic \
  --keyword-path /path/to/hey-friday.ppn
```

You should see logs for wake detection, partial/final transcripts, and TTS.

## Next Steps

- Replace mock engines with real wake word, ASR, and TTS implementations
- Add tests and benchmarks in `assistant-core`
- Create a Tauri UI shell

## Intents (mock)

- Timer: phrases like "set a timer for 10 seconds/minutes" → schedules a notification and speaks back confirmation.
