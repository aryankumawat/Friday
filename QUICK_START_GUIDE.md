# 🎤 Friday AI Assistant - Quick Start Guide

## Current Status
Friday is currently running in **MOCK MODE** for testing. To use it with your actual voice, follow these steps:

## 🚀 How to Use Friday with Real Audio

### Step 1: Check Your Audio Devices
```bash
cargo run -p assistant-cli devices
```

**Your available devices:**
- 0: MacBook Pro Microphone ✅ (Recommended)
- 1: Aryan's iPhone 15 Pro Microphone
- 2: WH-1000XM4
- 3: Voxal Virtual Device

### Step 2: Test Recording
Test that your microphone works:
```bash
# Record 5 seconds of audio to test.wav
cargo run -p assistant-cli record -- test.wav 5
```

### Step 3: Run Friday with Real Audio

#### Option A: Energy-Based Wake Detection (Easiest)
This listens for any sound above a threshold - just speak loudly:
```bash
make dev-energy
# Or:
cargo run -p assistant-cli -- --wake energy --capture --sessions 1
```

#### Option B: Real-Time Wake Detection
This listens for specific wake phrases like "Friday" or "Hey Friday":
```bash
make dev-realtime
# Or:
cargo run -p assistant-cli -- --wake realtime --capture --sessions 1
```

### Step 4: For Full Real Audio Pipeline (Advanced)

To use Whisper for real speech recognition and Piper for real TTS:

1. **Setup Whisper (Speech Recognition):**
```bash
make setup-whisper
# This downloads and builds Whisper.cpp
```

2. **Setup Piper (Text-to-Speech):**
```bash
make setup-piper
# This downloads Piper TTS
```

3. **Run with Real Engines:**
```bash
cargo run -p assistant-cli wakeflow
# This runs: wake detection → record audio → Whisper transcription → TTS response
```

## 🎯 What's Currently Working

### Mock Mode (What you just ran)
- ✅ Simulates the entire pipeline
- ✅ Tests all components
- ✅ No microphone needed
- ❌ Doesn't listen to your voice

### Real Audio Mode (What you need)
- ✅ Listens to your microphone
- ✅ Detects wake words
- ✅ Records your speech
- ⚠️ Needs Whisper for transcription (optional)
- ⚠️ Needs Piper for voice output (optional)

## 🔧 Quick Commands

### Test Your Setup
```bash
# Check system health
make doctor

# List audio devices
cargo run -p assistant-cli devices

# Record a test
cargo run -p assistant-cli record -- test.wav 5

# Show configuration
make config-show
```

### Run Different Modes
```bash
# Mock mode (no microphone)
make dev

# Energy wake detection (listens for loud sounds)
make dev-energy

# Real-time wake detection (listens for "Friday")
make dev-realtime

# Enhanced NLU (better intent recognition)
make dev-enhanced

# Plugin system (extensible commands)
make dev-plugins
```

## 💡 Why Nothing Happened When You Spoke

The system was running in **mock mode**, which means:
1. It's not actually listening to your microphone
2. It's using pre-recorded test phrases ("hello there assistant")
3. It's simulating the entire conversation pipeline
4. This is perfect for testing, but not for real use

## 🎤 To Actually Talk to Friday

Run one of these commands and Friday will listen to your microphone:

```bash
# Easiest - just speak loudly
cargo run -p assistant-cli -- --wake energy --capture

# Or use wake phrases
cargo run -p assistant-cli -- --wake realtime --capture
```

Then:
1. Wait for "Wake detected" message
2. Speak your command
3. Friday will process it and respond

## 🚀 Next Steps

1. **Test your microphone:** `cargo run -p assistant-cli record -- test.wav 5`
2. **Run with real audio:** `cargo run -p assistant-cli -- --wake energy --capture`
3. **Setup Whisper for better recognition:** `make setup-whisper`
4. **Setup Piper for voice responses:** `make setup-piper`

## 📝 Example Session

```bash
# Start Friday with energy-based wake detection
$ cargo run -p assistant-cli -- --wake energy --capture --sessions 1

# Output:
# Listening for wake word...
# [Speak loudly or make a sound]
# Wake detected!
# Recording... (speak your command)
# [System processes your speech]
# Response: "Okay, I heard you say..."
```

## 🎯 Supported Commands (Once Audio is Working)

- "Set a timer for 5 minutes"
- "What's the weather?"
- "Open Chrome"
- "Volume up"
- "What time is it?"
- And more with the enhanced NLU!

---

**TL;DR:** Run `cargo run -p assistant-cli -- --wake energy --capture` to actually use your microphone!
