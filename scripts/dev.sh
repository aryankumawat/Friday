#!/bin/bash
# Friday Voice Assistant - Development Helper Script

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Function to run CLI with different configurations
run_cli() {
    local mode=${1:-"mock"}
    
    case $mode in
        "mock")
            log_info "Running Friday Assistant with mock engines..."
            cargo run -p assistant-cli -- --ui-events --sessions 2
            ;;
        "real")
            log_info "Running Friday Assistant with real engines..."
            cargo run -p assistant-cli -- \
                --tts piper \
                --piper-bin ./piper_macos_arm64 \
                --piper-model ./en_US-lessac-medium.onnx \
                --asr whisper \
                --whisper-bin ./whisper.cpp/whisper \
                --whisper-model ./whisper.cpp/models/ggml-base.bin \
                --sessions 2
            ;;
        "ui")
            log_info "Running Friday Assistant UI..."
            cargo tauri dev
            ;;
        "doctor")
            log_info "Running health check..."
            cargo run -p assistant-cli -- doctor
            ;;
        *)
            echo "Usage: $0 [mock|real|ui|doctor]"
            echo ""
            echo "Modes:"
            echo "  mock   - Run with mock engines (default)"
            echo "  real   - Run with real TTS/ASR engines"
            echo "  ui     - Run Tauri UI"
            echo "  doctor - Run health check"
            exit 1
            ;;
    esac
}

# Check if engines are available for real mode
check_real_engines() {
    local missing=0
    
    if [ ! -f "./piper_macos_arm64" ]; then
        log_warning "Piper binary not found. Run 'make setup-piper' first."
        missing=1
    fi
    
    if [ ! -f "./en_US-lessac-medium.onnx" ]; then
        log_warning "Piper model not found. Run 'make setup-piper' first."
        missing=1
    fi
    
    if [ ! -f "./whisper.cpp/whisper" ]; then
        log_warning "Whisper binary not found. Run 'make setup-whisper' first."
        missing=1
    fi
    
    if [ ! -f "./whisper.cpp/models/ggml-base.bin" ]; then
        log_warning "Whisper model not found. Run 'make setup-whisper' first."
        missing=1
    fi
    
    if [ $missing -eq 1 ]; then
        echo ""
        log_warning "Some engines are missing. Run 'make setup' to install them."
        echo ""
    fi
}

# Main execution
mode=${1:-"mock"}

if [ "$mode" = "real" ]; then
    check_real_engines
fi

run_cli "$mode"