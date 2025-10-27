# Next Steps Plan: Real Audio & Wake Word Integration

## Current State
✅ Rust workspace with core + CLI  
✅ Mock pipeline (Wake → ASR → NLU → Executor → TTS)  
✅ Piper TTS integration (optional)  
✅ Whisper ASR integration (optional)  
✅ Simple intent parser (Timer intent)  
✅ Session manager with event bus  

## Phase 1: Real Wake Word Detection (Priority 1)

### Goal
Replace `MockWake` with real wake word detection using Porcupine (via `porcupine-rs` or shell-out) or custom keyword spotting.

### Implementation Steps

#### Step 1.1: Add Porcupine/Built-in Wake Word Support
**File**: `assistant-core/Cargo.toml`
```toml
[dependencies]
# Option A: Official Porcupine via subprocess (easier, less control)
# No new dependencies

# Option B: Use a Rust-compatible VAD + keyword spotter
# rhubarb-vad = "0.1"  # Voice activity detection
# or build custom small hotword detector
```

**File**: `assistant-core/src/lib.rs`
- Add `PorcupineWake` struct that shells out to `porcupine_demo_mic`
- Or use `rhubarb-vad` for continuous VAD → trigger on keyword phrase
- Alternative: Build lightweight wake using Silero VAD + keyword matching

#### Step 1.2: CLI Integration
**File**: `assistant-cli/src/main.rs`
- Add `--wake {mock|porcupine|custom}` flag
- Add `--wake-keyword` (path to keyword model or phrase)
- Update `SessionManager` instantiation to use selected wake detector

#### Step 1.3: Testing
- Test with built-in wake words ("Hey Siri", "Alexa", or custom "Friday")
- Measure false positive rate (should be <1 per hour)
- Verify wake latency <300ms

---

## Phase 2: Real Audio Capture (Priority 2)

### Goal
Stream audio from microphone into the pipeline instead of mock data.

### Implementation Steps

#### Step 2.1: Add cpal Dependency
**File**: `assistant-core/Cargo.toml`
```toml
cpal = "0.15"  # Cross-platform audio I/O
```

#### Step 2.2: Implement Audio Capture
**File**: `assistant-core/src/lib.rs` or new module `src/audio.rs`
- Create `AudioCapture` struct wrapping `cpal::Stream`
- Support configurable sample rate (16kHz default for whisper)
- Implement ring buffer for streaming to ASR
- Add `start_capture()` and `stop_capture()` methods

#### Step 2.3: Stream Audio to Wake + ASR
**File**: `assistant-core/src/lib.rs`
- Update `SessionManager` to take audio stream
- Pipe audio chunks to wake detector continuously (background task)
- On wake, start streaming to ASR until silence detected (VAD)
- Integrate VAD: use Silero VAD or rhubarb-vad to detect speech end

#### Step 2.4: VAD Integration
- Add Silero VAD model (lightweight ONNX) or use cpal + energy threshold
- Detect speech start/stop for ASR streaming
- Emit `PartialTranscript` as Whisper processes chunks

---

## Phase 3: Improved Intent & Multi-turn (Priority 3)

### Goal
Add more intents and support multi-turn dialogues.

### Implementation Steps

#### Step 3.1: Expand Intent Coverage
**File**: `assistant-core/src/lib.rs`
- Add intents:
  - `Weather { location?: String }`
  - `AppLaunch { app_name: String }`
  - `Timer { duration_secs: u64, label?: String }(); ); };}); };};`
  - `Query { llm_fallback: String }`
- Use regex patterns or NLP embeddings (MiniLM via ONNX)

#### Step 3.2: Executor for Each Intent
- Implement weather via OpenWeatherMap API
- Implement app launch via system commands (macOS: `open -a`, Windows: start, Linux: xdg-open)
- Query LLM for unknown intents (local llama.cpp or API fallback)

#### Step 3.3: Add Dialogue Context
**New File**: `assistant-core/src/dialogue.rs`
- `DialogueManager` struct to track context
- Slot filling for multi-turn queries
- Example: "Set a timer" → "For how long?" → "5 minutes" → Execute

---

## Phase 4: Tauri UI Shell (Priority 4)

### Goal
Build a cross-platform desktop UI for visual feedback and settings.

### Implementation Steps

#### Step 4.1: Add Tauri Crate
**New File**: `assistant-ui/` (separate crate or embedded)
```toml
[dependencies]
tauri = { version = "1", features = ["shell-open"] }
```

#### Step 4.2: UI Components
**Files**: Frontend (Svelte/React)
- Status indicator (idle, listening, processing, speaking)
- Transcript display (partial + final)
- Intent visualization
- Settings panel (model paths, wake sensitivity, TTS voice)
- History log

#### Step 4.3: IPC Between UI and Core
**File**: `assistant-core/src/ipc.rs` or use Tauri commands
- Expose engine events as Tauri events to frontend
- Allow UI to start/stop sessions, configure models
- Update README with UI setup instructions

---

## Phase 5: Advanced Features

### 5.1 Wake Sentence Mode (not just single keyword)
- Grammar-based keyword spotting
- Allow phrases like "Hey Friday" or "Listen Friday"
- Custom KWS model trained on user samples

### 5.2 Real-time Streaming Whisper
- Use `whisper-rs` Rust bindings instead of shell-out
- Stream audio chunks for lower latency
- Emit partials as speech progresses

### 5.3 Plugins/Extensions
- Plugin interface: WASM modules or separate processes
- Permissions system for plugins
- Example plugins: Calendar integration, Smart home control (Home Assistant)

### 5.4 Context & Memory
- Vector database for RAG (past conversations)
- SQLite + sqlite-vss for embeddings
- Per-user memory with privacy controls

---

## Immediate Next Task (Start Here)

### Task: Integrate Porcupine Wake Word
**Estimated Time**: 2-3 hours

#### Steps:
1. Download `porcupine_demo_mic` binary or install via apt/pip
2. Test it manually: `./porcupine_demo_mic --keywords hey_friday`
3. Create `PorcupineWake` struct in `assistant-core/src/lib.rs`:
   ```rust
   pub struct PorcupineWake {
       pub porcupine_bin: String,
       pub keyword_path: String,
   }
   ```
4. Implement `WakeDetector` trait for `PorcupineWake`:
   - Spawn subprocess and parse stdout for wake events
   - Or use a library binding if available
5. Add CLI flags: `--wake porcupine --wake-keyword /path/to/hey_friday.ppn`
6. Test end-to-end with mock ASR to ensure wake triggers pipeline

#### Code Locations:
- Add to `assistant-core/src/lib.rs` after `MockWake` implementation
- Update `assistant-cli/src/main.rs` Args struct with wake flags
- Update README with Porcupine setup instructions

---

## Success Criteria
- [ ] Wake word detects reliably (<1 false positive/hour)
- [ ] Real microphone audio captured and streamed
- [ ] Whisper transcribes audio with <5% WER
- [ ] Multiple intents work (timer, app launch, weather, query)
- [ ] Tauri UI shows live transcripts and status
- [ ] Cross-platform (macOS, Windows, Linux)

## Notes
- Keep mock implementations as fallbacks for CI/testing
- All audio processing should work offline (privacy-first)
- Use shell-out approach initially, migrate to Rust bindings later for better control
- Benchmark latency at each stage to meet <1.5s E2E target

