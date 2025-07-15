use crate::do_upload;
use crate::types::{
    Chunk, DownloadRequest, ToastEvent, UploadError, UploadFileEvent, UploadFilePayload,
};
use crate::{ANTTP_PORT, DWEB_PORT, WEBSOCKET_PORT};
use base64::decode;
use dirs::data_dir;
use futures::{stream::StreamExt, SinkExt};
use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{watch, Mutex};
use warp::ws::{Message, WebSocket};
use warp::Filter;

pub static WEBSOCKET_SHUTDOWN_TX: Lazy<Mutex<Option<watch::Sender<bool>>>> =
    Lazy::new(|| Mutex::new(None));
pub static WEBSOCKET_TASK_HANDLE: Lazy<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(None));

pub async fn stop_websocket_server() -> Result<(), String> {
    {
        let mut shutdown_lock = WEBSOCKET_SHUTDOWN_TX.lock().await;
        if let Some(shutdown_tx) = shutdown_lock.take() {
            let _ = shutdown_tx.send(true);
        }
    }

    {
        let mut handle_lock = WEBSOCKET_TASK_HANDLE.lock().await;
        if let Some(handle) = handle_lock.take() {
            handle
                .await
                .map_err(|e| format!("Failed to stop WS task: {:?}", e))?;
        }
    }

    Ok(())
}

async fn handle_root_ws(ws: WebSocket) {
    let (mut tx, mut rx) = ws.split();

    let _ = tx
        .send(Message::text("Connected to SafeBox Client WebSocket"))
        .await;

    while let Some(Ok(_msg)) = rx.next().await {
        // Ignore all messages on root WS
    }
}

pub async fn start_websocket_server(handle: AppHandle, mut shutdown_rx: watch::Receiver<bool>) {
    let port = *WEBSOCKET_PORT.lock().unwrap();

    let upload_handle = handle.clone();
    let download_handle = handle.clone();
    let chunk_store: FileChunks = Arc::new(Mutex::new(HashMap::new()));

    let upload_ws = warp::path("upload-ws")
        .and(warp::ws())
        .and(with_state(chunk_store.clone()))
        .and(with_handle(upload_handle))
        .map(|ws: warp::ws::Ws, store, handle| {
            ws.on_upgrade(move |socket| handle_upload_ws(socket, store, handle))
        });

    let download_ws = warp::path("download-ws")
        .and(warp::ws())
        .and(with_handle(download_handle))
        .map(|ws: warp::ws::Ws, handle| {
            ws.on_upgrade(move |socket| handle_download_ws(socket, handle))
        });

    let root_ws = warp::path::end()
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(move |socket| handle_root_ws(socket)));

    let routes = upload_ws
        .or(download_ws)
        .or(root_ws)
        .or(get_anttp_port())
        .or(get_dweb_port());

    let addr = ([127, 0, 0, 1], port);

    let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, async move {
        while !*shutdown_rx.borrow() {
            if shutdown_rx.changed().await.is_err() {
                break;
            }
        }
    });

    server.await;
}

fn get_anttp_port() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("getAntTPPort").map(|| {
        let port: u16 = *ANTTP_PORT.lock().unwrap();
        warp::reply::json(&json!({ "port": port }))
    })
}

fn get_dweb_port() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("getDWebPort").map(|| {
        let port: u16 = *DWEB_PORT.lock().unwrap();
        warp::reply::json(&json!({ "port": port }))
    })
}

fn with_handle(
    handle: AppHandle,
) -> impl Filter<Extract = (AppHandle,), Error = Infallible> + Clone {
    warp::any().map(move || handle.clone())
}

fn with_state(
    store: FileChunks,
) -> impl Filter<Extract = (FileChunks,), Error = Infallible> + Clone {
    warp::any().map(move || store.clone())
}

type FileChunks = Arc<Mutex<HashMap<String, Vec<Option<Vec<u8>>>>>>;

async fn handle_upload_ws(ws: WebSocket, store: FileChunks, handle: AppHandle) {
    let (mut tx, mut rx) = ws.split();

    println!("[WS] Upload WebSocket connection established");

    while let Some(Ok(msg)) = rx.next().await {
        if msg.is_text() {
            match serde_json::from_str::<Chunk>(msg.to_str().unwrap()) {
                Ok(chunk) => {
                    let decoded_data = match decode(&chunk.data) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("Base64 decode failed: {}", e);
                            let error_msg = json!({
                                "action": "uploadError",
                                "upload_id": chunk.metadata.upload_id,
                                "error": format!("Base64 decode failed: {}", e),
                            });
                            let _ = tx.send(Message::text(error_msg.to_string())).await;
                            continue;
                        }
                    };

                    const MAX_CHUNKS: usize = 100_000;
                    let key = chunk.metadata.upload_id.clone();
                    let total_chunks = chunk.metadata.total_chunks;

                    if total_chunks == 0 || total_chunks > MAX_CHUNKS {
                        eprintln!("Invalid total_chunks: {}", total_chunks);
                        let error_msg = json!({
                            "action": "uploadError",
                            "upload_id": chunk.metadata.upload_id,
                            "error": format!("Invalid total_chunks: {}", total_chunks),
                        });
                        let _ = tx.send(Message::text(error_msg.to_string())).await;
                        continue;
                    }

                    if chunk.metadata.chunk_index >= total_chunks {
                        eprintln!(
                            "Invalid chunk_index {} for total_chunks {}",
                            chunk.metadata.chunk_index, total_chunks
                        );
                        let error_msg = json!({
                            "action": "uploadError",
                            "upload_id": chunk.metadata.upload_id,
                            "error": format!(
                                "Invalid chunk_index {} for total_chunks {}",
                                chunk.metadata.chunk_index, total_chunks
                            ),
                        });
                        let _ = tx.send(Message::text(error_msg.to_string())).await;
                        continue;
                    }

                    let mut store_guard = store.lock().await;
                    let entry = store_guard.entry(key.clone()).or_insert_with(|| {
                        let mut vec = Vec::new();
                        if vec.try_reserve_exact(total_chunks).is_err() {
                            eprintln!("Memory allocation for chunk vector failed");
                            return Vec::new();
                        }
                        vec.resize_with(total_chunks, || None);
                        vec
                    });

                    entry[chunk.metadata.chunk_index] = Some(decoded_data);

                    let ack = json!({
                        "type": "chunk_received",
                        "upload_id": chunk.metadata.upload_id,
                        "chunk_index": chunk.metadata.chunk_index
                    });

                    if let Err(e) = tx.send(Message::text(ack.to_string())).await {
                        eprintln!("Failed to send chunk ack: {}", e);
                    }

                    if entry.iter().all(|c| c.is_some()) {
                        let mut full_data = Vec::new();
                        for part in entry.iter().flatten() {
                            full_data.extend_from_slice(part);
                        }

                        let output_path = data_dir()
                            .expect("Cannot find data dir")
                            .join("safebox")
                            .join(&chunk.metadata.filename);

                        tokio::fs::create_dir_all(output_path.parent().unwrap())
                            .await
                            .unwrap();

                        let mut file = File::create(&output_path).await.unwrap();
                        file.write_all(&full_data).await.unwrap();

                        let file_data = tokio::fs::read(&output_path).await.unwrap();

                        let payload = UploadFilePayload {
                            name: chunk.metadata.filename.clone(),
                            mime_type: chunk.metadata.mime_type.clone(),
                            data: file_data,
                        };

                        let response = match do_upload(payload, &handle).await {
                            Ok(xorname) => {
                                handle
                                    .emit(
                                        "upload-file",
                                        UploadFileEvent {
                                            name: chunk.metadata.filename.clone(),
                                            mime_type: chunk.metadata.mime_type.clone(),
                                            xorname: Some(xorname.clone()),
                                            success: true,
                                            error: None,
                                        },
                                    )
                                    .unwrap();

                                json!({
                                    "type": "upload_complete",
                                    "upload_id": chunk.metadata.upload_id,
                                    "xorname": xorname
                                })
                            }
                            Err(error) => {
                                handle
                                    .emit(
                                        "upload-file",
                                        UploadFileEvent {
                                            name: chunk.metadata.filename.clone(),
                                            mime_type: chunk.metadata.mime_type.clone(),
                                            xorname: None,
                                            success: false,
                                            error: Some(UploadError {
                                                title: "Upload Failed".into(),
                                                description: format!("{:?}", error),
                                            }),
                                        },
                                    )
                                    .unwrap();

                                json!({
                                    "action": "uploadError",
                                    "upload_id": chunk.metadata.upload_id,
                                    "error": format!("{:?}", error)
                                })
                            }
                        };

                        if let Err(e) = tx.send(Message::text(response.to_string())).await {
                            eprintln!("Failed to send response to extension: {}", e);
                        }

                        store_guard.remove(&key);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to deserialize chunk JSON: {}", e);
                }
            }
        }
    }
}

async fn handle_download_ws(ws: WebSocket, handle: AppHandle) {
    let (mut tx, mut rx) = ws.split();

    if let Some(Ok(msg)) = rx.next().await {
        if msg.is_text() {
            match serde_json::from_str::<DownloadRequest>(msg.to_str().unwrap()) {
                Ok(req) if req.action == "download" => {
                    handle
                        .emit(
                            "show-toast",
                            ToastEvent {
                                title: "Success".into(),
                                description: "Download request sent to client".into(),
                            },
                        )
                        .unwrap();
                    // Implement actual download logic as needed
                }
                _ => {
                    let error_response = json!({ "error": "Invalid request format or xorname" });
                    let _ = tx.send(Message::text(error_response.to_string())).await;
                }
            }
        }
    }
}
