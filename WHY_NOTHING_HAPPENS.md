# 🤔 Why Nothing Happens When You Say "Hey Friday"

## The Current Situation

When you run Friday and say "Hey Friday", nothing happens because of how the audio pipeline is currently set up. Here's what's actually going on:

## 🔍 The Problem

### What You're Running:
```bash
cargo run -p assistant-cli -- --wake realtime --sessions 1
```

### What's Actually Happening:

1. **Audio Capture is NOT Started** ❌
   - The `--wake realtime` flag uses the wake detector
   - But it doesn't automatically start audio capture
   - You need to add the `--capture` flag!

2. **The Wake Detector Needs Audio** ❌
   - The AudioWake detector I just created needs audio chunks
   - Without `--capture`, it has no audio to analyze
   - It's waiting for audio that never comes

## ✅ The Solution

### Option 1: Add the --capture Flag (EASIEST)

```bash
~/.cargo/bin/cargo run -p assistant-cli -- --wake realtime --capture --sessions 1
```

The `--capture` flag starts the microphone and feeds audio to the wake detector!

### Option 2: Use the Test Script

```bash
./test_audio_simple.sh
```

This script will:
- Show your microphones
- Start audio capture automatically
- Give you visual feedback

### Option 3: Use Energy-Based Detection (More Reliable)

```bash
~/.cargo/bin/cargo run -p assistant-cli -- --wake energy --capture --sessions 1
```

Energy-based detection is simpler and more reliable - it just listens for loud sounds!

## 🎯 What Each Flag Does

| Flag | What It Does |
|------|--------------|
| `--wake realtime` | Uses the AudioWake detector (needs --capture) |
| `--wake energy` | Uses energy-based detection (needs --capture) |
| `--wake mock` | Simulates wake detection (no mic needed) |
| `--capture` | **STARTS YOUR MICROPHONE** ⭐ This is the key! |
| `--sessions 1` | Runs one conversation session |

## 🔧 The Technical Details

### Without --capture:
```
Friday starts → Wake detector waits for audio → No audio coming → Waits forever
```

### With --capture:
```
Friday starts → Microphone starts → Audio flows → Wake detector analyzes → Detects sound → Wakes up!
```

## 🚀 Quick Test Right Now

Run this command and **speak loudly** or **clap**:

```bash
~/.cargo/bin/cargo run -p assistant-cli -- --wake energy --capture --sessions 1
```

You should see:
```
🎤 Starting REAL audio-based wake detection...
📊 Energy threshold: 0.0500
⏱️  Trigger duration: 300ms

💡 Speak loudly or make a sound to wake Friday!

🔊 Listening... (monitoring audio levels)
📊 Current audio level: 0.0023 (threshold: 0.0500, max seen: 0.0045)
🔊 Sound detected! Energy: 0.0678 (above threshold 0.0500)
⏳ Waiting for sustained sound...
✅ WAKE WORD DETECTED! (sustained for 312ms)
🎤 Friday is now listening to your command...
```

## 💡 Pro Tips

1. **Speak LOUDLY** - The default threshold is 0.05, which requires fairly loud speech
2. **Sustain the sound** - You need to speak for at least 300-500ms
3. **Try clapping** - A loud clap will definitely trigger it
4. **Check your mic** - Run `cargo run -p assistant-cli devices` to see your mics

## 🎤 Adjusting Sensitivity

If it's too sensitive or not sensitive enough, you can adjust the threshold in the code:

**More Sensitive (picks up quieter sounds):**
```rust
.with_threshold(0.01)  // Lower = more sensitive
```

**Less Sensitive (only loud sounds):**
```rust
.with_threshold(0.10)  // Higher = less sensitive
```

## 🐛 Still Not Working?

### Check These:

1. **Is your microphone working?**
   ```bash
   ~/.cargo/bin/cargo run -p assistant-cli record -- test.wav 5
   ```
   Speak for 5 seconds, then play back test.wav

2. **Are you using the --capture flag?**
   Without it, no audio is captured!

3. **Is the threshold too high?**
   Try energy-based detection with a lower threshold

4. **Check the logs**
   Look for messages like "🔊 Sound detected!" to see if audio is being picked up

## 📝 Summary

**The ONE thing you were missing: `--capture`**

This flag starts your microphone. Without it, Friday has no audio to listen to!

**Try this right now:**
```bash
~/.cargo/bin/cargo run -p assistant-cli -- --wake energy --capture --sessions 1
```

Then **SPEAK LOUDLY** or **CLAP YOUR HANDS**! 👏
