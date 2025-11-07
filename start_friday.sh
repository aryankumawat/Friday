#!/bin/bash

# Friday AI Assistant - Easy Start Script
# This script helps you start Friday with real audio

echo "🤖 Friday AI Assistant - Starting..."
echo ""
echo "Choose how you want to run Friday:"
echo ""
echo "1) Mock Mode (Testing - no microphone needed)"
echo "2) Energy Wake Detection (Listens for loud sounds)"
echo "3) Real-time Wake Detection (Listens for 'Friday' or 'Hey Friday')"
echo "4) Test Microphone Recording"
echo "5) Show Available Audio Devices"
echo "6) Check System Health"
echo ""
read -p "Enter your choice (1-6): " choice

case $choice in
    1)
        echo ""
        echo "🎤 Starting Friday in Mock Mode..."
        echo "This simulates conversations without using your microphone."
        echo ""
        cargo run -p assistant-cli -- --ui-events --sessions 2
        ;;
    2)
        echo ""
        echo "🎤 Starting Friday with Energy-Based Wake Detection..."
        echo "Friday will wake up when it hears loud sounds."
        echo "Just speak loudly or make a noise!"
        echo ""
        echo "Press Ctrl+C to stop."
        echo ""
        cargo run -p assistant-cli -- --wake energy --capture --sessions 5
        ;;
    3)
        echo ""
        echo "🎤 Starting Friday with Real-time Wake Detection..."
        echo "Say 'Friday', 'Hey Friday', or 'Hello Friday' to wake it up."
        echo ""
        echo "Press Ctrl+C to stop."
        echo ""
        cargo run -p assistant-cli -- --wake realtime --capture --sessions 5
        ;;
    4)
        echo ""
        echo "🎤 Testing Microphone..."
        echo "Recording 5 seconds of audio to test_recording.wav"
        echo "Speak now!"
        echo ""
        cargo run -p assistant-cli record -- test_recording.wav 5
        echo ""
        echo "✅ Recording saved to test_recording.wav"
        echo "You can play it back to verify your microphone works."
        ;;
    5)
        echo ""
        echo "🎤 Available Audio Devices:"
        echo ""
        cargo run -p assistant-cli devices
        ;;
    6)
        echo ""
        echo "🏥 Running System Health Check..."
        echo ""
        cargo run -p assistant-cli doctor
        ;;
    *)
        echo ""
        echo "❌ Invalid choice. Please run the script again and choose 1-6."
        exit 1
        ;;
esac

echo ""
echo "✅ Done!"
