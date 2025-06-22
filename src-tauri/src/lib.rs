use once_cell::sync::Lazy;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::{AppHandle, Window};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

static ANTPP_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));
static DWEB_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));

#[tauri::command]
async fn start_server(app: AppHandle, window: Window) -> Result<String, String> {
    let mut ant_lock = ANTPP_PROCESS.lock().unwrap();
    let mut dweb_lock = DWEB_PROCESS.lock().unwrap();

    if ant_lock.is_some() || dweb_lock.is_some() {
        return Err("Services already running".into());
    }

    let sidecar_cmd = app
        .shell()
        .sidecar("anttp")
        .map_err(|e| format!("Failed to create anttp sidecar command: {}", e))?;

    let (mut anttp_rx, mut anttp_child) = sidecar_cmd
        .args(["-l", "127.0.0.1:8082"])
        .spawn()
        .map_err(|e| format!("Failed to spawn anttp: {}", e))?;

    let sidecar_cmd = app
        .shell()
        .sidecar("dweb")
        .map_err(|e| format!("Failed to create dweb sidecar command: {}", e))?;

    let (mut dweb_rx, mut dweb_child) = sidecar_cmd
        .args(["serve", "--port", "8083"])
        .spawn()
        .map_err(|e| format!("Failed to spawn dweb: {}", e))?;

    *ant_lock = Some(anttp_child);
    *dweb_lock = Some(dweb_child);

    let window_clone_for_anttp = window.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = anttp_rx.recv().await {
            if let CommandEvent::Stdout(bytes) = event {
                let line = String::from_utf8_lossy(&bytes);
                let _ = window_clone_for_anttp.emit("anttp-message", Some(line.to_string()));
            }
        }
    });

    let window_clone_for_dweb = window.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = dweb_rx.recv().await {
            if let CommandEvent::Stdout(bytes) = event {
                let line = String::from_utf8_lossy(&bytes);
                let _ = window_clone_for_dweb.emit("dweb-message", Some(line.to_string()));
            }
        }
    });

    Ok("anttp and dweb started".into())
}

#[tauri::command]
fn stop_server() -> Result<String, String> {
    let mut ant_lock = ANTPP_PROCESS.lock().unwrap();
    let mut dweb_lock = DWEB_PROCESS.lock().unwrap();

    if let Some(mut anttp_child) = ant_lock.take() {
        let _ = anttp_child.kill();
    }
    if let Some(mut dweb_child) = dweb_lock.take() {
        let _ = dweb_child.kill();
    }

    Ok("anttp and dweb stopped".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .invoke_handler(tauri::generate_handler![start_server, stop_server])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
