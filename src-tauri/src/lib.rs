use crate::types::UploadFilePayload;
use crate::websockets::start_websocket_server;
use crate::websockets::stop_websocket_server;
use crate::websockets::WEBSOCKET_SHUTDOWN_TX;
use crate::websockets::WEBSOCKET_TASK_HANDLE;
use once_cell::sync::Lazy;
use std::env;
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::WindowEvent;
use tauri::{AppHandle, Window};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_store::StoreBuilder;
use tempfile::NamedTempFile;

mod types;
mod websockets;

pub static ANT_PORT: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(8081));
pub static ANTTP_PORT: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(18888));
pub static DWEB_PORT: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(5537));
pub static WEBSOCKET_PORT: Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(8084));

static ANT_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));
static ANTPP_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));
static DWEB_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));

fn is_port_in_use(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_err()
}

#[tauri::command]
fn get_ports() -> Result<(u16, u16, u16, u16), String> {
    let ant_port = *ANT_PORT.lock().unwrap();
    let anttp_port = *ANTTP_PORT.lock().unwrap();
    let dweb_port = *DWEB_PORT.lock().unwrap();
    let websocket_port = *WEBSOCKET_PORT.lock().unwrap();
    Ok((ant_port, anttp_port, dweb_port, websocket_port))
}

fn set_port(mutex: &Mutex<u16>, port: u16) -> Result<(), String> {
    if port == 0 {
        return Err("Port must be > 0".into());
    }
    *mutex.lock().unwrap() = port;
    Ok(())
}

#[tauri::command]
fn set_ant_port(port: u16) -> Result<(), String> {
    set_port(&ANT_PORT, port)
}

#[tauri::command]
fn set_anttp_port(port: u16) -> Result<(), String> {
    set_port(&ANTTP_PORT, port)
}

#[tauri::command]
fn set_dweb_port(port: u16) -> Result<(), String> {
    set_port(&DWEB_PORT, port)
}

#[tauri::command]
async fn set_websocket_port(port: u16, app_handle: tauri::AppHandle) -> Result<(), String> {
    if port == 0 {
        return Err("Port must be > 0".into());
    }

    {
        let mut port_lock = WEBSOCKET_PORT.lock().unwrap();

        *port_lock = port;
    }

    // stop current WS server
    stop_websocket_server().await?;

    // create a new watch channel for shutdown signaling
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    {
        // store shutdown sender for later use
        let mut shutdown_lock = WEBSOCKET_SHUTDOWN_TX.lock().await;
        *shutdown_lock = Some(shutdown_tx);
    }

    // start new WS server
    let handle_clone = app_handle.clone();
    let join_handle =
        tauri::async_runtime::spawn(start_websocket_server(handle_clone, shutdown_rx));

    {
        let mut task_handle_lock = WEBSOCKET_TASK_HANDLE.lock().await;
        *task_handle_lock = Some(join_handle);
    }

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
fn kill_process_on_port(port: u16) -> Result<(), String> {
    use std::process::Command;

    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", &format!("netstat -ano | findstr :{}", port)])
        .output()
        .map_err(|e| e.to_string())?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .args(["-c", &format!("lsof -t -i :{}", port)])
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    #[cfg(target_os = "windows")]
    {
        for line in stdout.lines() {
            if let Some(pid) = line.split_whitespace().last() {
                Command::new("taskkill")
                    .args(["/PID", pid, "/F"])
                    .output()
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        for pid in stdout.lines() {
            Command::new("kill")
                .arg("-9")
                .arg(pid)
                .output()
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn get_app_store_path() -> Result<PathBuf, String> {
    // get the OS-specific user data directory (e.g. on Windows: C:\Users\<User>\AppData\Roaming)
    let mut app_dir = dirs::data_dir().ok_or("Could not find data directory")?;

    // append your app folder name to it (e.g., "SafeBoxClient")
    app_dir.push("client.safebox.desktop");

    // create the directory if it doesn't exist yet (optional)
    std::fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create app dir: {}", e))?;

    // append your store file name
    app_dir.push("store.bin");

    Ok(app_dir)
}

#[tauri::command]
async fn import_wallet(app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("Import wallet started");

    // use your get_app_store_path function to get the store path
    let store_path = get_app_store_path()?;

    // initialize the store
    let wallet_store = match StoreBuilder::new(&app_handle, store_path.clone()).build() {
        Ok(store) => store,
        Err(e) => {
            println!("Warning: Failed to build store (continuing without wallet): {e}");
            return Ok(()); // continue without wallet
        }
    };

    // get the wallet key and password entries (they are expected to be JSON objects with a "value" string)
    let key_entry = wallet_store.get("wallet-private-key");

    let pwd_entry = wallet_store.get("wallet-password");

    if let Some(serde_json::Value::Object(key_map)) = key_entry {
        if let Some(serde_json::Value::String(private_key)) = key_map.get("value") {
            if !private_key.trim().is_empty() {
                let import_cmd = app_handle
                    .shell()
                    .sidecar("ant")
                    .map_err(|e| format!("Failed to create ant sidecar: {}", e))?;

                // build args as owned Strings
                let mut args: Vec<String> = vec![
                    "wallet".to_string(),
                    "import".to_string(),
                    private_key.clone(),
                    "--no-password".to_string(),
                ];

                if let Some(serde_json::Value::Object(pwd_map)) = pwd_entry {
                    if let Some(serde_json::Value::String(password)) = pwd_map.get("value") {
                        if !password.trim().is_empty() {
                            args = vec![
                                "wallet".to_string(),
                                "import".to_string(),
                                private_key.clone(),
                                "--password".to_string(),
                                password.clone(),
                            ];
                        }
                    }
                }

                // convert owned Strings to &str slices for passing to args()
                let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

                match import_cmd.args(args_ref).output().await {
                    Ok(output) => {
                        if !output.status.success() {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            println!(
                                "Warning: Wallet import failed (continuing without wallet): {stderr}"
                            );
                        } else {
                            println!("Wallet imported successfully.");
                        }
                    }
                    Err(e) => {
                        println!("Warning: Failed to import wallet: {e}");
                    }
                }
            } else {
                println!("Warning: Private key is empty (skipping wallet import)");
            }
        } else {
            println!("Warning: 'value' field is missing or not a string in wallet-private-key");
        }
    } else {
        println!("No private key in store (wallet not imported)");
    }

    Ok(())
}

pub async fn do_upload(payload: UploadFilePayload, handle: &AppHandle) -> Result<String, String> {
    // create a temporary file synchronously
    let mut temp_file =
        NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;

    // write data to temp file synchronously because NamedTempFile uses std::fs::File
    temp_file
        .write_all(&payload.data)
        .map_err(|e| format!("Failed to write to temp file: {}", e))?;

    let file_path = temp_file.path();

    // launch ant sidecar command
    let ant_cmd = handle
        .shell()
        .sidecar("ant")
        .map_err(|e| format!("Failed to create ant sidecar: {}", e))?;

    let output = ant_cmd
        .args([
            "file",
            "upload",
            file_path.to_str().ok_or("Invalid file path")?,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to execute ant: {}", e))?;

    if output.status.success() {
        let xorname = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(xorname)
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("ant upload failed: {}", err))
    }
}

#[tauri::command]
fn is_server_running() -> bool {
    let ant_lock = ANT_PROCESS.lock().unwrap();
    let anttp_lock = ANTPP_PROCESS.lock().unwrap();
    let dweb_lock = DWEB_PROCESS.lock().unwrap();

    ant_lock.is_some() || anttp_lock.is_some() || dweb_lock.is_some()
}

#[tauri::command]
async fn start_server(app: AppHandle, window: Window) -> Result<String, String> {
    let mut ant_lock = ANT_PROCESS.lock().unwrap();
    let mut anttp_lock = ANTPP_PROCESS.lock().unwrap();
    let mut dweb_lock = DWEB_PROCESS.lock().unwrap();

    let ant_port = *ANT_PORT.lock().unwrap();
    let anttp_port = *ANTTP_PORT.lock().unwrap();
    let dweb_port = *DWEB_PORT.lock().unwrap();

    if ant_lock.is_some() || anttp_lock.is_some() || dweb_lock.is_some() {
        return Err("Services already running".into());
    }

    // check if ant port is in use
    if is_port_in_use(ant_port) {
        let _ = window.dialog().message(format!(
            "Port {} is already in use by another process. Cannot start 'ant'.",
            ant_port
        ));
        return Err(format!("Port {} in use", ant_port));
    }

    // check if anttp port is in use
    if is_port_in_use(anttp_port) {
        let _ = window.dialog().message(format!(
            "Port {} is already in use by another process. Cannot start 'anttp'.",
            anttp_port
        ));
        return Err(format!("Port {} in use", anttp_port));
    }

    // check if dweb port is in use
    if is_port_in_use(dweb_port) {
        let _ = window.dialog().message(format!(
            "Port {} is already in use by another process. Cannot start 'dweb'.",
            dweb_port
        ));
        return Err(format!("Port {} in use", dweb_port));
    }

    // start ant
    let ant_cmd = app
        .shell()
        .sidecar("ant")
        .map_err(|e| format!("Failed to create ant sidecar: {}", e))?;

    let (mut ant_rx, ant_child) = match ant_cmd
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

    // start anttp
    let anttp_cmd = app
        .shell()
        .sidecar("anttp")
        .map_err(|e| format!("Failed to create anttp sidecar: {}", e))?;

    let (mut anttp_rx, anttp_child) = match anttp_cmd
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

    // start dweb
    let dweb_cmd = app
        .shell()
        .sidecar("dweb")
        .map_err(|e| format!("Failed to create dweb sidecar: {}", e))?;

    let (mut dweb_rx, dweb_child) = match dweb_cmd
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

macro_rules! kill_child {
    ($lock:expr) => {
        if let Some(child) = $lock.lock().unwrap().take() {
            let _ = child.kill();
        }
    };
}

#[tauri::command]
fn stop_server() -> Result<String, String> {
    kill_child!(ANT_PROCESS);
    kill_child!(ANTPP_PROCESS);
    kill_child!(DWEB_PROCESS);

    Ok("ant, anttp and dweb stopped".into())
}

fn cleanup_processes() {
    let mut ant_lock = ANT_PROCESS.lock().unwrap();
    let mut anttp_lock = ANTPP_PROCESS.lock().unwrap();
    let mut dweb_lock = DWEB_PROCESS.lock().unwrap();

    tauri::async_runtime::block_on(stop_websocket_server()).ok();

    if let Some(ant_child) = ant_lock.take() {
        let _ = ant_child.kill();
    }
    if let Some(anttp_child) = anttp_lock.take() {
        let _ = anttp_child.kill();
    }
    if let Some(dweb_child) = dweb_lock.take() {
        let _ = dweb_child.kill();
    }
}

#[tauri::command]
fn get_binary_version(binary_name: String) -> Result<String, String> {
    use std::{path::PathBuf, process::Command};

    // figures out the correct filename based on OS
    let target_triple = if cfg!(target_os = "macos") {
        format!("{}-apple-darwin", std::env::consts::ARCH)
    } else if cfg!(target_os = "windows") {
        format!("{}-pc-windows-msvc.exe", std::env::consts::ARCH)
    } else {
        format!("{}-unknown-linux-gnu", std::env::consts::ARCH)
    };

    let filename = format!("{}-{}", binary_name, target_triple);
    let mut path = PathBuf::from("bin");
    path.push(filename);

    let output = Command::new(&path)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to start process {}: {}", path.display(), e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
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
            set_dweb_port,
            set_websocket_port,
            kill_process_on_port,
            is_server_running,
            get_binary_version,
            import_wallet,
        ])
        .setup(|app| {
            let handle = app.handle();
            let handle_clone = handle.clone();

            // register ctrl-c handler once at startup
            ctrlc::set_handler(|| {
                cleanup_processes();
                std::process::exit(0);
            })
            .expect("Error setting Ctrl-C handler");

            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

            // store the shutdown tx so you can stop it later
            tauri::async_runtime::spawn(async move {
                let mut tx_guard = websockets::WEBSOCKET_SHUTDOWN_TX.lock().await;
                *tx_guard = Some(shutdown_tx);
            });

            // start the server and store the handle
            let task: tauri::async_runtime::JoinHandle<()> =
                tauri::async_runtime::spawn(async move {
                    crate::websockets::start_websocket_server(handle_clone, shutdown_rx).await;
                });

            tauri::async_runtime::spawn(async move {
                let mut handle_guard = websockets::WEBSOCKET_TASK_HANDLE.lock().await;
                *handle_guard = Some(task);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                cleanup_processes();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running safebox client application");
}
