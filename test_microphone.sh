#!/bin/bash

echo "🎤 Friday Microphone Test"
echo "=========================="
echo ""
echo "This will test your microphone and show real-time audio levels."
echo "Speak into your microphone and you should see the levels change."
echo ""
echo "Press Ctrl+C to stop."
echo ""

# Create a simple Rust program to test microphone
cat > /tmp/test_mic.rs << 'EOF'
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

fn main() {
    println!("🎤 Initializing microphone...");
    
    let host = cpal::default_host();
    let device = host.default_input_device().expect("No input device found");
    let device_name = device.name().expect("Failed to get device name");
    
    println!("✅ Using device: {}", device_name);
    println!("");
    println!("📊 Audio levels (speak now!):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let config = device.default_input_config().expect("Failed to get config");
    
    let (tx, rx) = mpsc::channel();
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let buffer_clone = buffer.clone();
    
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = buffer_clone.lock().unwrap();
            buf.extend_from_slice(data);
            
            if buf.len() >= 1024 {
                let energy: f32 = buf.iter().map(|&x| x * x).sum::<f32>() / buf.len() as f32;
                let rms = energy.sqrt();
                let _ = tx.send(rms);
                buf.clear();
            }
        },
        |err| eprintln!("Error: {}", err),
        None,
    ).expect("Failed to build stream");
    
    use cpal::traits::StreamTrait;
    stream.play().expect("Failed to play stream");
    
    loop {
        if let Ok(level) = rx.recv() {
            let bars = (level * 100.0) as usize;
            let bar_str = "█".repeat(bars.min(50));
            print!("\r{:50} {:.4}", bar_str, level);
            use std::io::Write;
            std::io::stdout().flush().unwrap();
            
            if level > 0.02 {
                print!(" 🔊 SOUND DETECTED!");
            }
        }
    }
}
EOF

echo "Compiling test program..."
cd /tmp && rustc test_mic.rs -o test_mic 2>/dev/null

if [ $? -eq 0 ]; then
    ./test_mic
else
    echo "❌ Failed to compile test program"
    echo ""
    echo "Let's try a simpler approach - running Friday with verbose output:"
    echo ""
    cd - > /dev/null
    ~/.cargo/bin/cargo run -p assistant-cli -- --wake energy --capture --sessions 1
fi
