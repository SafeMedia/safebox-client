use futures::lock::Mutex;
use safeapi::{Multiaddr, Network, Safe, SecretKey, XorName, XorNameBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};
use types::UploadFilePayload;

mod frontend;
mod helpers;
mod server;
mod types;

#[cfg(target_os = "linux")]
mod linux;

use frontend::*;

const ACCOUNTS_DIR: &str = "accounts";
const SK_FILENAME: &str = "sk.key";
const ADDRESS_FILENAME: &str = "evm_address";

const DEFAULT_LOG_LEVEL: &str = "ERROR";

#[derive(Debug, Serialize, Deserialize)]
enum Error {
    Common(String),
    BadLogin,
    BadPassword,
    NotConnected,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            crate::Error::Common(msg) => f.write_str(&msg),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for Error {}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::Common(err)
    }
}

impl From<tauri::Error> for Error {
    fn from(tauri_error: tauri::Error) -> Self {
        Self::Common(format!("Tauri: {}", tauri_error))
    }
}

impl From<safeapi::Error> for Error {
    fn from(safe_error: safeapi::Error) -> Self {
        Self::Common(format!("Safe: {}", safe_error))
    }
}

fn make_root(app: &mut AppHandle) -> Result<PathBuf, Error> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| Error::Common(format!("Could not get app data dir")))?;
    fs::create_dir_all(&app_data).map_err(|_| {
        Error::Common(format!(
            "Could not create app data dir: {}",
            &app_data.display()
        ))
    })?;

    Ok(app_data)
}

fn user_root(app_root: &PathBuf, login: String) -> PathBuf {
    let mut sk_dir = app_root.clone();
    sk_dir.push(ACCOUNTS_DIR);
    sk_dir.push(login);

    sk_dir
}

fn load_create_import_key(
    app_root: &PathBuf,
    login: String,
    password: String,
    eth_pk: Option<String>, // if you want to import ethereum private key during registration
    register: bool,
) -> Result<String, Error> {
    let sk_dir = user_root(app_root, login);
    let mut sk_file = sk_dir.clone();
    sk_file.push(SK_FILENAME);

    let not_readable_msg = format!("Could not read user key file: {}", &sk_file.display());
    let eth_pk = if sk_file
        .try_exists()
        .map_err(|_| Error::Common(not_readable_msg.clone()))?
    {
        if register {
            return Err(Error::Common(String::from("User already exists")));
        }

        let bytes = fs::read(&sk_file).map_err(|_| Error::Common(not_readable_msg.clone()))?;

        Safe::decrypt_eth(&bytes, &password)?
    } else {
        if !register {
            return Err(Error::BadLogin);
        }

        let pk = eth_pk.unwrap_or(SecretKey::random().to_hex()); // bls secret key can be used as eth privkey

        let file_bytes = Safe::encrypt_eth(pk.clone(), &password)?;
        fs::create_dir_all(sk_dir.clone()).map_err(|_| {
            Error::Common(format!("Could not create user dir: {}", &sk_dir.display()))
        })?;
        fs::write(&sk_file, file_bytes).map_err(|_| {
            Error::Common(format!(
                "Could not save user key file: {}",
                &sk_file.display()
            ))
        })?;
        pk
    };

    Ok(eth_pk)
}

#[tauri::command]
async fn list_accounts(mut app: AppHandle) -> Result<Vec<(String, String)>, Error> {
    let mut accounts_dir = make_root(&mut app)
        .map_err(|_| Error::Common(format!("Cannot access/create application folder.")))?;
    accounts_dir.push(ACCOUNTS_DIR);

    let default_mod_time = std::time::SystemTime::now();

    let mut entries = fs::read_dir(accounts_dir)
        .map_err(|err| Error::Common(format!("Error reading accounts. {}", err)))?
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .map(|entry| {
            let address_file = entry.path().join(ADDRESS_FILENAME);

            let modified = address_file
                .metadata()
                .map(|m| m.modified().unwrap_or(default_mod_time))
                .unwrap_or(default_mod_time);
            let username = entry.file_name().to_string_lossy().to_string();
            let address = fs::read_to_string(&address_file)
                .inspect_err(|err| eprintln!("Error reading address file. {}", err))
                .unwrap_or(String::from("<error>"));

            (modified, username, address)
        })
        .collect::<Vec<(std::time::SystemTime, String, String)>>();

    entries.sort(); // sort by "modified"

    Ok(entries
        .iter()
        .map(|(_modified, username, address)| (username.clone(), address.clone()))
        .collect::<Vec<(String, String)>>())
}

// leave peer empty or anything other than Multiaddr to connect to official network.
#[tauri::command]
async fn connect(peer: Option<String>, app: AppHandle) -> Result<(), Error> {
    let state = app.try_state::<Mutex<Option<Safe>>>();
    if state.is_some() {
        return if state.unwrap().lock().await.is_some() {
            println!("Already connected.");
            Ok(())
        } else {
            println!("Already connecting...");
            Err(Error::Common(String::from("Already connecting.")))
        };
    }
    app.manage(Mutex::new(None::<Safe>));

    let mut peers: Vec<Multiaddr> = Vec::new();

    let add_network_contacts = peer
        .map(|p_str| {
            p_str
                .parse::<Multiaddr>()
                .inspect(|multi_addr| {
                    println!("Peer: {}", &multi_addr);
                    peers.push(multi_addr.clone());
                })
                .map(|_| false)
                .unwrap_or(true)
        })
        .unwrap_or(true);

    if add_network_contacts {
        println!("Connecting to official network.");
    }

    println!("\n\nConnecting...");

    let safe = Safe::connect(Network::Mainnet, None, DEFAULT_LOG_LEVEL)
        .await
        .inspect_err(|_| {
            app.unmanage::<Mutex<Option<Safe>>>();
        })?;

    println!("\n\nConnected.");

    // Emit the connect event with the extracted address
    let _ = app
        .emit("connected", ())
        .inspect_err(|e| eprintln!("{}", e));

    // Store the `safe` object in the application's state
    *(app.state::<Mutex<Option<Safe>>>().lock().await) = Some(safe);

    Ok(())
}

#[tauri::command]
async fn sign_in(
    login: String,
    password: String,
    eth_pk_import: Option<String>,
    register: bool,
    mut app: AppHandle,
) -> Result<(), Error> {
    let app_root = make_root(&mut app)?;
    println!("eth_pk_import: {:?}", eth_pk_import);
    println!("register: {:?}", register);

    let pk = load_create_import_key(&app_root, login.clone(), password, eth_pk_import, register)?;
    println!("\n\nEth Private Key: {:.4}(...)", pk);

    app.try_state::<Mutex<Option<Safe>>>()
        .ok_or(Error::NotConnected)?
        .lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)? // not signed in
        .login_with_eth(Some(pk))?; // sign in

    let address = client_address(
        app.try_state::<Mutex<Option<Safe>>>()
            .ok_or(Error::NotConnected)?,
    )
    .await?;
    println!("ETH wallet address: {}", address);

    // Prepare the address directory and file
    let addr_dir = user_root(&app_root, login.clone());
    let mut addr_file = addr_dir.clone();
    addr_file.push(ADDRESS_FILENAME);

    // Write the address to a file
    fs::write(&addr_file, &address).map_err(|_| {
        Error::Common(format!(
            "Could not save address file: {}",
            &addr_file.display()
        ))
    })?;

    session_set(
        String::from(USER_SESSION_KEY),
        Some(
            serde_json::to_string(&SimpleAccountUser {
                username: login,
                address: address,
            })
            .expect("Object values should be able to serialize."),
        ),
        app.clone(),
    )
    .await;

    let _ = app.emit("sign_in", ()).inspect_err(|e| eprintln!("{}", e));

    Ok(())
}

#[tauri::command]
async fn is_connected(app: AppHandle) -> bool {
    let safe = app.try_state::<Mutex<Option<Safe>>>();
    safe.is_some() // state is managed
        && safe.unwrap().lock().await.as_ref().is_some() // option<safe> is some
}

#[tauri::command]
async fn disconnect(app: AppHandle) -> Result<(), Error> {
    app.unmanage::<Mutex<Option<Safe>>>()
        .ok_or(Error::NotConnected)?;

    let _ = app
        .emit("disconnected", ())
        .inspect_err(|e| eprintln!("{}", e));

    Ok(())
}

#[tauri::command]
async fn log_level(level: String, app: AppHandle) -> Result<(), Error> {
    let _ = app
        .try_state::<Mutex<Option<Safe>>>()
        .ok_or(Error::NotConnected)?
        .lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)? // safe
        .log_level(&level)?;

    Ok(())
}

type Session = std::collections::HashMap<String, String>;

#[tauri::command]
async fn session_set(key: String, value: Option<String>, app: AppHandle) -> Option<String> {
    let state = app
        .try_state::<Mutex<Session>>()
        .expect("Session not managed.");
    let mut session = state.lock().await;

    if let Some(v) = value {
        session.insert(key, v)
    } else {
        session.remove(&key)
    }
}

#[tauri::command]
async fn session_read(key: String, app: AppHandle) -> Option<String> {
    let state = app
        .try_state::<Mutex<Session>>()
        .expect("Session not managed.");
    let session = state.lock().await;

    session.get(&key).cloned()
}

fn meta_builder(name: Vec<String>) -> Result<XorNameBuilder, Error> {
    if name.is_empty() {
        return Err(Error::Common(String::from("Empty name.")));
    }
    let mut mb = XorNameBuilder::from_str(&name[0]);
    for n in &name[1..] {
        mb = mb.with_str(&n);
    }
    Ok(mb)
}

#[tauri::command]
async fn create_reg(
    name: Vec<String>,
    data: String,
    safe: State<'_, Mutex<Option<Safe>>>,
) -> Result<(), Error> {
    println!("\n\nReg create...");
    println!("Name: {:?}", name);

    let meta = meta_builder(name)
        .unwrap_or(XorNameBuilder::random())
        .build();
    println!("Data: {}", &data);
    println!("Meta: {}", &meta);

    //    let (mut reg, cost, royalties) = safe
    safe.lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)?
        .reg_create(data.as_bytes(), &meta)
        .await?;

    println!("\n\nReg created");
    //    println!("Costs: {}, {}", cost, royalties);

    //    Ok((reg.address().to_hex(), cost.as_nano(), royalties.as_nano()))
    Ok(())
}

#[tauri::command]
async fn read_reg(
    name: Vec<String>,
    safe: State<'_, Mutex<Option<Safe>>>,
) -> Result<String, Error> {
    let meta = meta_builder(name)?.build();

    let data = safe
        .lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)?
        .read_reg(&meta, None)
        .await?;

    Ok(String::from_utf8(data).map_err(|e| Error::Common(format!("{e}")))?)
}

#[tauri::command]
async fn write_reg(
    name: Vec<String>,
    data: String,
    safe: State<'_, Mutex<Option<Safe>>>,
) -> Result<(), Error> {
    println!("\n\nReg write...");
    println!("Name: {:?}", name);

    let meta = meta_builder(name)?.build();
    println!("Meta: {}", meta);

    println!("Writing data: {}", &data);
    if !data.is_empty() {
        safe.lock()
            .await
            .as_mut()
            .ok_or(Error::NotConnected)?
            .reg_write(data.as_bytes(), &meta)
            .await?;

        println!("\n\nReg updated.");
    } else {
        return Err(Error::Common(String::from("Empty data object string.")));
    }

    Ok(())
}

#[tauri::command]
async fn client_address(safe: State<'_, Mutex<Option<Safe>>>) -> Result<String, Error> {
    let address = safe
        .lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)?
        .address()?;
    Ok(address.to_string())
}

#[tauri::command]
async fn balance(safe: State<'_, Mutex<Option<Safe>>>) -> Result<String, Error> {
    let balance = safe
        .lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)?
        .balance()
        .await?;
    //    Ok(format!("{:x}", balance)) // hex string
    Ok(format!("{}", balance.0))
}

#[tauri::command]
async fn gas_balance(safe: State<'_, Mutex<Option<Safe>>>) -> Result<String, Error> {
    let balance = safe
        .lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)?
        .balance()
        .await?;
    //    Ok(format!("{:x}", balance)) // hex string
    Ok(format!("{}", balance.1))
}

#[tauri::command]
fn check_key(login: String, password: String, mut app: AppHandle) -> Result<String, Error> {
    let app_root = make_root(&mut app)?;
    load_create_import_key(&app_root, login, password, None, false)
}

#[tauri::command]
fn delete_account(login: String, mut app: AppHandle) -> Result<(), Error> {
    let app_root = make_root(&mut app)?;
    let sk_dir = user_root(&app_root, login);
    if sk_dir.try_exists().map_err(|_| {
        Error::Common(format!(
            "Could not check existence of {}.",
            sk_dir.display()
        ))
    })? {
        fs::remove_dir_all(&sk_dir)
            .map_err(|e| Error::Common(format!("Could not remove {}: {}", sk_dir.display(), e)))?
    }
    Ok(())
}

#[tauri::command]
async fn download(
    xorname: String,
    file_name: Option<String>, // name with extension
    destination: String,       // directory to download to
    app: AppHandle,
) -> Result<Option<Vec<u8>>, Error> {
    // Only return file data

    let xorname_bytes: [u8; 32] = hex::decode(xorname)
        .map_err(|e| Error::Common(format!("Invalid xorname: {}", e)))?[0..32]
        .try_into()
        .unwrap();
    let xorname = XorName(xorname_bytes);

    let data = app
        .try_state::<Mutex<Option<Safe>>>()
        .ok_or(Error::NotConnected)?
        .lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)?
        .download(xorname)
        .await?;

    // Return just the file data
    Ok(Some(data)) // Return the file data as Option<Vec<u8>>
}

async fn do_upload(payload: UploadFilePayload, app: &AppHandle) -> Result<String, Error> {
    let data = payload.data;
    println!(
        "Received file: {} (type: {})",
        payload.name, payload.mime_type
    );
    put_data(data, app.clone()).await
}

#[tauri::command]
async fn upload(payload: UploadFilePayload, app: AppHandle) -> Result<String, Error> {
    do_upload(payload, &app).await
}

// returns hex-encoded xorname
#[tauri::command]
async fn put_data(data: Vec<u8>, app: AppHandle) -> Result<String, Error> {
    let data_address = app
        .try_state::<Mutex<Option<Safe>>>()
        .ok_or(Error::NotConnected)?
        .lock()
        .await
        .as_mut()
        .ok_or(Error::NotConnected)? // safe
        .upload(&data)
        .await?;

    println!("{:?}", data_address); // Debug output
    println!("{}", hex::encode(data_address)); // String output

    Ok(hex::encode(data_address))
}

fn rename_with_extension<P: AsRef<Path>>(file_path: P) -> std::io::Result<PathBuf> {
    let old_path = file_path.as_ref();

    if let Some((_, ext)) = detect_file_type(old_path) {
        let new_path = old_path.with_extension(ext);
        fs::rename(old_path, &new_path)?;
        println!("Renamed to: {}", new_path.display());
        Ok(new_path)
    } else {
        println!("Could not determine extension to rename.");
        Ok(old_path.to_path_buf())
    }
}

fn detect_file_type<P: AsRef<Path>>(file_path: P) -> Option<(String, String)> {
    let data = fs::read(&file_path).ok()?;

    let info = infer::get(&data)?;
    Some((info.mime_type().to_string(), info.extension().to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .manage(Mutex::new(Session::new()))
        .invoke_handler(tauri::generate_handler![
            list_accounts,
            connect,
            sign_in,
            is_connected,
            disconnect,
            log_level,
            session_set,
            session_read,
            create_reg,
            read_reg,
            write_reg,
            client_address,
            balance,
            gas_balance,
            check_key,
            delete_account,
            download,
            upload,
            put_data,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                server::run(handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
