# Friday Voice Assistant - Build Configuration
.PHONY: help build test clean dev install-deps check format lint doctor setup-whisper setup-piper setup-ui

# Default target
help:
	@echo "Friday Voice Assistant Build System"
	@echo ""
	@echo "Available targets:"
	@echo "  build         - Build all workspace crates"
	@echo "  test          - Run all tests"
	@echo "  clean         - Clean build artifacts"
	@echo "  dev           - Run CLI in development mode"
	@echo "  dev-ui        - Run Tauri UI in development mode"
	@echo "  install-deps  - Install system dependencies"
	@echo "  check         - Run cargo check on all crates"
	@echo "  format        - Format code with rustfmt"
	@echo "  lint          - Run clippy linter"
	@echo "  doctor        - Run health check"
	@echo "  setup-whisper - Download and build Whisper.cpp"
	@echo "  setup-piper   - Download Piper TTS"
	@echo "  setup-ui      - Setup UI dependencies"
	@echo "  release       - Build optimized release"

# Build targets
build:
	@echo "🔨 Building all crates..."
	cargo build --workspace

release:
	@echo "🚀 Building release version..."
	cargo build --workspace --release

# Development targets
dev:
	@echo "🎤 Starting Friday Assistant (CLI)..."
	cargo run -p assistant-cli -- --ui-events --sessions 2

dev-ui:
	@echo "🖥️  Starting Friday Assistant (UI)..."
	cargo tauri dev

# Testing and quality
test:
	@echo "🧪 Running tests..."
	cargo test --workspace

check:
	@echo "🔍 Checking code..."
	cargo check --workspace

format:
	@echo "✨ Formatting code..."
	cargo fmt --all

lint:
	@echo "📋 Running linter..."
	cargo clippy --workspace -- -D warnings

# Health check
doctor:
	@echo "🏥 Running health check..."
	cargo run -p assistant-cli -- doctor

# Setup targets
install-deps:
	@echo "📦 Installing system dependencies..."
	@if command -v brew >/dev/null 2>&1; then \
		echo "Installing via Homebrew..."; \
		brew install cmake pkg-config; \
	else \
		echo "Please install cmake and pkg-config manually"; \
	fi

setup-whisper:
	@echo "🗣️  Setting up Whisper.cpp..."
	@if [ ! -d "whisper.cpp" ]; then \
		git clone https://github.com/ggerganov/whisper.cpp.git; \
	fi
	@cd whisper.cpp && make clean && make
	@if [ ! -f "whisper.cpp/models/ggml-base.bin" ]; then \
		cd whisper.cpp && bash ./models/download-ggml-model.sh base; \
	fi
	@echo "✅ Whisper.cpp setup complete"

setup-piper:
	@echo "🔊 Setting up Piper TTS..."
	@if [ ! -f "piper_macos_arm64" ]; then \
		curl -LO https://github.com/rhasspy/piper/releases/download/v1.2.0/piper_macos_arm64.tar.gz; \
		tar -xzf piper_macos_arm64.tar.gz; \
		chmod +x piper; \
		mv piper piper_macos_arm64; \
		rm piper_macos_arm64.tar.gz; \
	fi
	@if [ ! -f "en_US-lessac-medium.onnx" ]; then \
		curl -LO https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx; \
		curl -LO https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json; \
	fi
	@echo "✅ Piper TTS setup complete"

setup-ui:
	@echo "🖥️  Setting up UI dependencies..."
	@if ! command -v npm >/dev/null 2>&1; then \
		echo "❌ npm not found. Please install Node.js"; \
		exit 1; \
	fi
	@cd assistant-ui && npm install
	@echo "✅ UI dependencies installed"

# Cleanup
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	@if [ -d "target" ]; then rm -rf target; fi

# Full setup for new developers
setup: install-deps setup-whisper setup-piper
	@echo "🎉 Friday Assistant setup complete!"
	@echo ""
	@echo "Try these commands:"
	@echo "  make dev      - Run the CLI assistant"
	@echo "  make dev-ui   - Run the UI assistant"
	@echo "  make doctor   - Check system health"