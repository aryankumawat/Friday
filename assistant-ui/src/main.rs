#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Emitter;
use tokio::io::AsyncBufReadExt;

#[tauri::command]
async fn start_assistant(app: tauri::AppHandle, args: Vec<String>) -> Result<(), String> {
    // Use the compiled binary directly
    let binary_path = "/Users/aryankumawat/Friday/target/debug/assistant-cli";
    let mut cmd = tokio::process::Command::new(binary_path);
    let mut full_args = vec!["--ui-events".to_string(), "--sessions".to_string(), "2".to_string()];
    full_args.extend(args);
    cmd.args(full_args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.current_dir("/Users/aryankumawat/Friday");
    println!("Spawning assistant CLI...");
    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn: {}", e))?;
    let stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app_clone.emit("assistant:event", line);
        }
    });
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![start_assistant])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
