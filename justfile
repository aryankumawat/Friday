# Friday Voice Assistant - Just Build Configuration
# Alternative to Makefile using the 'just' command runner

# Default recipe
default:
    @just --list

# Build all workspace crates
build:
    @echo "🔨 Building all crates..."
    cargo build --workspace

# Build release version
release:
    @echo "🚀 Building release version..."
    cargo build --workspace --release

# Run CLI assistant in development mode
dev:
    @echo "🎤 Starting Friday Assistant (CLI)..."
    cargo run -p assistant-cli -- --ui-events --sessions 2

# Run Tauri UI in development mode
dev-ui:
    @echo "🖥️  Starting Friday Assistant (UI)..."
    cargo tauri dev

# Run tests
test:
    @echo "🧪 Running tests..."
    cargo test --workspace

# Check code without building
check:
    @echo "🔍 Checking code..."
    cargo check --workspace

# Format code
fmt:
    @echo "✨ Formatting code..."
    cargo fmt --all

# Run linter
lint:
    @echo "📋 Running linter..."
    cargo clippy --workspace -- -D warnings

# Run health check
doctor:
    @echo "🏥 Running health check..."
    cargo run -p assistant-cli -- doctor

# Setup Whisper.cpp
setup-whisper:
    @echo "🗣️  Setting up Whisper.cpp..."
    #!/usr/bin/env bash
    if [ ! -d "whisper.cpp" ]; then
        git clone https://github.com/ggerganov/whisper.cpp.git
    fi
    cd whisper.cpp && make clean && make
    if [ ! -f "whisper.cpp/models/ggml-base.bin" ]; then
        cd whisper.cpp && bash ./models/download-ggml-model.sh base
    fi
    echo "✅ Whisper.cpp setup complete"

# Setup Piper TTS
setup-piper:
    @echo "🔊 Setting up Piper TTS..."
    #!/usr/bin/env bash
    if [ ! -f "piper_macos_arm64" ]; then
        curl -LO https://github.com/rhasspy/piper/releases/download/v1.2.0/piper_macos_arm64.tar.gz
        tar -xzf piper_macos_arm64.tar.gz
        chmod +x piper
        mv piper piper_macos_arm64
        rm piper_macos_arm64.tar.gz
    fi
    if [ ! -f "en_US-lessac-medium.onnx" ]; then
        curl -LO https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx
        curl -LO https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json
    fi
    echo "✅ Piper TTS setup complete"

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean

# Full development setup
setup: setup-whisper setup-piper
    @echo "🎉 Friday Assistant setup complete!"