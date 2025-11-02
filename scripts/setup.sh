#!/bin/bash
# Friday Voice Assistant - Development Setup Script

set -e

echo "🎤 Friday Voice Assistant - Development Setup"
echo "=============================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check Rust installation
check_rust() {
    log_info "Checking Rust installation..."
    if command_exists rustc && command_exists cargo; then
        RUST_VERSION=$(rustc --version)
        log_success "Rust found: $RUST_VERSION"
    else
        log_error "Rust not found. Please install Rust from https://rustup.rs/"
        exit 1
    fi
}

# Install system dependencies
install_system_deps() {
    log_info "Installing system dependencies..."
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if command_exists brew; then
            brew install cmake pkg-config
            log_success "macOS dependencies installed"
        else
            log_warning "Homebrew not found. Please install cmake and pkg-config manually"
        fi
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        if command_exists apt-get; then
            sudo apt-get update
            sudo apt-get install -y cmake pkg-config libasound2-dev
            log_success "Linux dependencies installed"
        elif command_exists yum; then
            sudo yum install -y cmake pkg-config alsa-lib-devel
            log_success "Linux dependencies installed"
        else
            log_warning "Package manager not recognized. Please install cmake, pkg-config, and ALSA development libraries manually"
        fi
    else
        log_warning "OS not recognized. Please install cmake and pkg-config manually"
    fi
}

# Setup Whisper.cpp
setup_whisper() {
    log_info "Setting up Whisper.cpp..."
    
    if [ ! -d "whisper.cpp" ]; then
        git clone https://github.com/ggerganov/whisper.cpp.git
        log_success "Whisper.cpp cloned"
    else
        log_info "Whisper.cpp already exists, updating..."
        cd whisper.cpp && git pull && cd ..
    fi
    
    cd whisper.cpp
    make clean && make
    log_success "Whisper.cpp built"
    
    if [ ! -f "models/ggml-base.bin" ]; then
        log_info "Downloading Whisper base model..."
        bash ./models/download-ggml-model.sh base
        log_success "Whisper model downloaded"
    else
        log_info "Whisper model already exists"
    fi
    
    cd ..
}

# Setup Piper TTS
setup_piper() {
    log_info "Setting up Piper TTS..."
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if [ ! -f "piper_macos_arm64" ]; then
            log_info "Downloading Piper for macOS ARM64..."
            curl -LO https://github.com/rhasspy/piper/releases/download/v1.2.0/piper_macos_arm64.tar.gz
            tar -xzf piper_macos_arm64.tar.gz
            chmod +x piper
            mv piper piper_macos_arm64
            rm piper_macos_arm64.tar.gz
            log_success "Piper binary installed"
        else
            log_info "Piper binary already exists"
        fi
    else
        log_warning "Please download the appropriate Piper binary for your platform from:"
        log_warning "https://github.com/rhasspy/piper/releases"
    fi
    
    if [ ! -f "en_US-lessac-medium.onnx" ]; then
        log_info "Downloading Piper voice model..."
        curl -LO https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx
        curl -LO https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json
        log_success "Piper voice model downloaded"
    else
        log_info "Piper voice model already exists"
    fi
}

# Build the project
build_project() {
    log_info "Building Friday Voice Assistant..."
    cargo build --workspace
    log_success "Project built successfully"
}

# Run health check
run_doctor() {
    log_info "Running health check..."
    cargo run -p assistant-cli -- doctor
}

# Main setup flow
main() {
    echo ""
    log_info "Starting setup process..."
    echo ""
    
    check_rust
    install_system_deps
    setup_whisper
    setup_piper
    build_project
    
    echo ""
    log_success "Setup complete! 🎉"
    echo ""
    echo "Next steps:"
    echo "  • Run 'make dev' or 'cargo run -p assistant-cli -- --ui-events'"
    echo "  • Run 'make doctor' to verify everything works"
    echo "  • Check README.md for usage examples"
    echo ""
    
    run_doctor
}

# Run main function
main "$@"