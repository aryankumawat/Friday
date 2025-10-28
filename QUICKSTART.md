# Quick Start for Voice-Enabled Friday Assistant

## Current Status
The voice assistant is WORKING with mock wake → ASR → TTS. You saw the JSON output.

## To Make It Fully Voice-Enabled:

### Option 1: Install Real TTS/ASR (Recommended)
```bash
# 1. Download Piper TTS
curl -LO https://github.com/rhasspy/piper/releases/download/v1.2.0/piper_macos_arm64
chmod +x piper_macos_arm64

# 2. Download a Piper voice model
curl -LO https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx

# 3. Build Whisper.cpp
git clone https://github.com/ggerganov/whisper.cpp
cd whisper.cpp && make && cd ..

# 4. Download Whisper model
./models/download-ggml-model.sh base

# 5. Run with real voice
cargo run -p assistant-cli -- \
  --tts piper \
  --piper-bin ./piper_macos_arm64 \
  --piper-model ./en_US-lessac-medium.onnx \
  --asr whisper \
  --whisper-bin ./whisper.cpp/whisper \
  --whisper-model ./whisper.cpp/models/ggml-base.bin
```

### Option 2: Use What We Have (Mock Version)
The mock version WORKS and shows the complete pipeline!

```bash
# This shows the complete voice flow
cargo run -p assistant-cli -- --ui-events

# Or run Tauri UI
source ~/.cargo/env && cargo tauri dev
# Then click "Start Assistant"
```

## What Happens in Mock Mode:
1. **Wake detection** → "WakeDetected" event
2. **Partial transcripts** → "hello" → "hello there" → "hello there assistant"
3. **Final transcript** → "hello there assistant"
4. **Intent recognition** → Identified as "Unknown" (not a timer command)
5. **TTS response** → Spoken: "Okay." (default response)

## What You're Building:
A voice assistant like F.R.I.D.A.Y. from Iron Man that:
- Listens for wake words
- Transcribes speech
- Understands intent
- Responds with voice

The mock version proves the architecture works!
