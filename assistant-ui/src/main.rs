#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

#[tauri::command]
async fn start_assistant(window: tauri::Window, args: Vec<String>) -> Result<(), String> {
    let mut cmd = tokio::process::Command::new("cargo");
    let mut full_args = vec!["run".to_string(), "-p".to_string(), "assistant-cli".to_string(), "--".to_string(), "run".to_string(), "--ui-events".to_string()];
    full_args.extend(args);
    cmd.args(full_args);
    cmd.stdout(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    tauri::async_runtime::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = window.emit("assistant:event", line);
        }
    });
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![start_assistant])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
