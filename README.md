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


You should see logs for wake detection, partial/final transcripts, and TTS.

## Next Steps

- Replace mock engines with real wake word, ASR, and TTS implementations
- Add tests and benchmarks in `assistant-core`
- Create a Tauri UI shell

## Intents (mock)

- Timer: phrases like "set a timer for 10 seconds/minutes" → schedules a notification and speaks back confirmation.
