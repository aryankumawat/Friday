#!/bin/bash
echo "🎤 Starting Friday Voice Assistant..."
source ~/.cargo/env
cargo run -p assistant-cli -- --ui-events --sessions 2
