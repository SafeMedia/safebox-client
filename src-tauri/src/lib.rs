use once_cell::sync::Lazy;
use std::net::TcpListener;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::{AppHandle, Window};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

static ANT_PORT: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(8081));
static ANTPP_PORT: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(8082));
static DWEB_PORT: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(8083));

static ANT_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));
static ANTPP_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));
static DWEB_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));

fn is_port_in_use(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_err()
}

#[tauri::command]
fn get_ports() -> Result<(u16, u16, u16), String> {
    let ant_port = *ANT_PORT.lock().unwrap();
    let anttp_port = *ANTPP_PORT.lock().unwrap();
    let dweb_port = *DWEB_PORT.lock().unwrap();
    Ok((ant_port, anttp_port, dweb_port))
}

#[tauri::command]
fn set_ant_port(port: u16) -> Result<(), String> {
    if port == 0 {
        return Err("Port must be > 0".into());
    }
    *ANT_PORT.lock().unwrap() = port;
    Ok(())
}

#[tauri::command]
fn set_anttp_port(port: u16) -> Result<(), String> {
    if port == 0 {
        return Err("Port must be > 0".into());
    }
    *ANTPP_PORT.lock().unwrap() = port;
    Ok(())
}

#[tauri::command]
fn set_dweb_port(port: u16) -> Result<(), String> {
    if port == 0 {
        return Err("Port must be > 0".into());
    }
    *DWEB_PORT.lock().unwrap() = port;
    Ok(())
}

fn handle_permission_error(window: &Window, binary_name: &str) {
    #[cfg(target_os = "macos")]
    {
        let msg = format!(
            "macOS blocked the '{}' binary from running.\n\n\
            To fix:\n1. Open System Settings > Privacy & Security\n\
            2. Scroll down to 'Security'\n3. Click 'Allow Anyway' next to '{}'\n\
            4. Then restart the app.",
            binary_name, binary_name
        );
        let _ = window.dialog().message(msg);
    }

    #[cfg(target_os = "linux")]
    {
        let msg = format!(
            "Linux blocked the '{}' binary from executing.\n\n\
            Try running:\n  chmod +x {}\n\
            Or ensure it has the correct execution permissions.",
            binary_name, binary_name
        );
        let _ = window.dialog().message(msg);
    }

    #[cfg(target_os = "windows")]
    {
        let msg = format!(
            "Windows prevented the '{}' binary from running.\n\n\
            Try:\n1. Right-click the binary\n2. Choose 'Properties'\n\
            3. Under the 'General' tab, click 'Unblock' if available\n\
            4. Apply and try again.",
            binary_name
        );
        let _ = window.dialog().message(msg);
    }
}

#[tauri::command]
async fn start_server(app: AppHandle, window: Window) -> Result<String, String> {
    let mut ant_lock = ANT_PROCESS.lock().unwrap();
    let mut anttp_lock = ANTPP_PROCESS.lock().unwrap();
    let mut dweb_lock = DWEB_PROCESS.lock().unwrap();

    let ant_port = *ANT_PORT.lock().unwrap();
    let anttp_port = *ANTPP_PORT.lock().unwrap();
    let dweb_port = *DWEB_PORT.lock().unwrap();

    if ant_lock.is_some() || anttp_lock.is_some() || dweb_lock.is_some() {
        return Err("Services already running".into());
    }

    // Check if ant port is in use
    if is_port_in_use(ant_port) {
        let _ = window.dialog().message(format!(
            "Port {} is already in use by another process. Cannot start 'ant'.",
            ant_port
        ));
        return Err(format!("Port {} in use", ant_port));
    }

    // Check if anttp port is in use
    if is_port_in_use(anttp_port) {
        let _ = window.dialog().message(format!(
            "Port {} is already in use by another process. Cannot start 'anttp'.",
            anttp_port
        ));
        return Err(format!("Port {} in use", anttp_port));
    }

    // Check if dweb port is in use
    if is_port_in_use(dweb_port) {
        let _ = window.dialog().message(format!(
            "Port {} is already in use by another process. Cannot start 'dweb'.",
            dweb_port
        ));
        return Err(format!("Port {} in use", dweb_port));
    }

    // Start ant
    let ant_cmd = app
        .shell()
        .sidecar("ant")
        .map_err(|e| format!("Failed to create ant sidecar: {}", e))?;

    // I added argument to bind ant to the port
    let (mut ant_rx, mut ant_child) = match ant_cmd
        .args(["-l", &format!("127.0.0.1:{}", ant_port)])
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("Permission denied") || err_str.contains("operation not permitted")
            {
                handle_permission_error(&window, "ant");
            }
            return Err(format!("Failed to spawn ant: {}", err_str));
        }
    };

    *ant_lock = Some(ant_child);

    let window_clone_for_ant = window.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = ant_rx.recv().await {
            if let CommandEvent::Stdout(bytes) = event {
                let line = String::from_utf8_lossy(&bytes);
                let _ = window_clone_for_ant.emit("ant-message", Some(line.to_string()));
            }
        }
    });

    // Start anttp
    let anttp_cmd = app
        .shell()
        .sidecar("anttp")
        .map_err(|e| format!("Failed to create anttp sidecar: {}", e))?;

    let (mut anttp_rx, mut anttp_child) = match anttp_cmd
        .args(["-l", &format!("127.0.0.1:{}", anttp_port)])
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("Permission denied") || err_str.contains("operation not permitted")
            {
                handle_permission_error(&window, "anttp");
            }
            return Err(format!("Failed to spawn anttp: {}", err_str));
        }
    };

    *anttp_lock = Some(anttp_child);

    let window_clone_for_anttp = window.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = anttp_rx.recv().await {
            if let CommandEvent::Stdout(bytes) = event {
                let line = String::from_utf8_lossy(&bytes);
                let _ = window_clone_for_anttp.emit("anttp-message", Some(line.to_string()));
            }
        }
    });

    // Start dweb
    let dweb_cmd = app
        .shell()
        .sidecar("dweb")
        .map_err(|e| format!("Failed to create dweb sidecar: {}", e))?;

    let (mut dweb_rx, mut dweb_child) = match dweb_cmd
        .args(["serve", "--port", &dweb_port.to_string()])
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("Permission denied") || err_str.contains("operation not permitted")
            {
                handle_permission_error(&window, "dweb");
            }
            return Err(format!("Failed to spawn dweb: {}", err_str));
        }
    };

    *dweb_lock = Some(dweb_child);

    let window_clone_for_dweb = window.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = dweb_rx.recv().await {
            if let CommandEvent::Stdout(bytes) = event {
                let line = String::from_utf8_lossy(&bytes);
                let _ = window_clone_for_dweb.emit("dweb-message", Some(line.to_string()));
            }
        }
    });

    Ok("ant, anttp and dweb started".into())
}

#[tauri::command]
fn stop_server() -> Result<String, String> {
    let mut ant_lock = ANT_PROCESS.lock().unwrap();
    let mut anttp_lock = ANTPP_PROCESS.lock().unwrap();
    let mut dweb_lock = DWEB_PROCESS.lock().unwrap();

    if let Some(mut ant_child) = ant_lock.take() {
        let _ = ant_child.kill();
    }
    if let Some(mut anttp_child) = anttp_lock.take() {
        let _ = anttp_child.kill();
    }
    if let Some(mut dweb_child) = dweb_lock.take() {
        let _ = dweb_child.kill();
    }

    Ok("ant, anttp and dweb stopped".into())
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
        .invoke_handler(tauri::generate_handler![
            start_server,
            stop_server,
            get_ports,
            set_ant_port,
            set_anttp_port,
            set_dweb_port
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
