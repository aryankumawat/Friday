#!/bin/bash

echo "🎤 FRIDAY AUDIO WAKE TEST"
echo "========================="
echo ""
echo "This will:"
echo "1. Show your available microphones"
echo "2. Start listening with REAL audio detection"
echo "3. Wake up when you speak loudly"
echo ""
echo "Ready? Press Enter to start..."
read

echo ""
echo "📱 Your audio devices:"
~/.cargo/bin/cargo run --quiet -p assistant-cli devices 2>/dev/null
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🎤 Starting Friday with REAL audio detection..."
echo "💡 TIP: Speak LOUDLY or clap your hands!"
echo "⏱️  Friday will wake up after 500ms of sustained sound"
echo ""
echo "Press Ctrl+C to stop"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Run with realtime wake detection
~/.cargo/bin/cargo run --quiet -p assistant-cli -- --wake realtime --sessions 1
