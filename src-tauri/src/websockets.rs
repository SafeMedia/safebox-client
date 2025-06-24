use crate::do_upload;
use crate::types::{ToastEvent, UploadError, UploadFileEvent, UploadFilePayload};
use crate::ANTTP_PORT;
use crate::DWEB_PORT;
use dirs::data_dir;
use futures::{stream::StreamExt, SinkExt};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tauri::async_runtime::JoinHandle as TauriJoinHandle;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tokio::sync::Mutex;
use warp::ws::{Message, WebSocket};
use warp::Filter;

use crate::WEBSOCKET_PORT;

pub static WEBSOCKET_SHUTDOWN_TX: Lazy<Mutex<Option<watch::Sender<bool>>>> =
    Lazy::new(|| Mutex::new(None));
pub static WEBSOCKET_TASK_HANDLE: Lazy<Mutex<Option<TauriJoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(None));

pub async fn stop_websocket_server() -> Result<(), String> {
    // Signal shutdown
    {
        let mut shutdown_lock = WEBSOCKET_SHUTDOWN_TX.lock().await;
        if let Some(shutdown_tx) = shutdown_lock.take() {
            let _ = shutdown_tx.send(true); // ignore error if no receiver
        }
    }

    // Abort and await task
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
        // ignore
    }
}

pub async fn start_websocket_server(
    handle: tauri::AppHandle,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let port = *WEBSOCKET_PORT.lock().unwrap();

    let upload_handle = handle.clone();

    // not currently used as we directly use anttp/dweb
    let download_handle_for_ws = handle.clone();

    let chunk_store: FileChunks = Arc::new(Mutex::new(HashMap::new()));

    let upload_ws = warp::path("upload-ws")
        .and(warp::ws())
        .and(with_state(chunk_store.clone()))
        .and(with_handle(upload_handle))
        .map(|ws: warp::ws::Ws, store, handle| {
            ws.on_upgrade(move |socket| handle_upload_ws(socket, store, handle))
        });

    // not currently used as we directly use anttp/dweb
    let download_ws = warp::path("download-ws")
        .and(warp::ws())
        .and(with_handle(download_handle_for_ws))
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
        // wait until shutdown_rx changes to true
        while !*shutdown_rx.borrow() {
            if shutdown_rx.changed().await.is_err() {
                break; // channel closed
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Chunk {
    name: String,
    mime_type: String,
    chunk_index: usize,
    total_chunks: usize,
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct DownloadRequest {
    action: String,
    xorname: String,
}

async fn handle_upload_ws(ws: WebSocket, store: FileChunks, handle: AppHandle) {
    let (mut tx, mut rx) = ws.split();

    while let Some(Ok(msg)) = rx.next().await {
        if msg.is_text() {
            match serde_json::from_str::<Chunk>(msg.to_str().unwrap()) {
                Ok(chunk) => {
                    const MAX_CHUNKS: usize = 100_000;

                    let key = chunk.name.clone();
                    let chunk_size = std::mem::size_of::<Option<Vec<u8>>>();
                    let total_chunks = chunk.total_chunks;

                    if total_chunks == 0 || total_chunks > MAX_CHUNKS {
                        eprintln!("Invalid total_chunks value: {}", total_chunks);
                        continue;
                    }

                    if chunk.chunk_index >= total_chunks {
                        eprintln!(
                            "Invalid chunk_index {} for total_chunks {}",
                            chunk.chunk_index, total_chunks
                        );
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

                    entry[chunk.chunk_index] = Some(chunk.data.clone());

                    if entry.iter().all(|c| c.is_some()) {
                        handle
                            .emit(
                                "show-toast",
                                ToastEvent {
                                    title: "Received Upload Request".to_string(),
                                    description: "Starting file upload process now".to_string(),
                                },
                            )
                            .unwrap();

                        let mut full_data = Vec::new();
                        for part in entry.iter().flatten() {
                            full_data.extend_from_slice(part);
                        }

                        let output_path = data_dir()
                            .expect("Cannot find data dir")
                            .join("safebox")
                            .join(&chunk.name);

                        tokio::fs::create_dir_all(output_path.parent().unwrap())
                            .await
                            .unwrap();
                        let mut file = File::create(output_path.clone()).await.unwrap();
                        file.write_all(&full_data).await.unwrap();

                        let file_data = tokio::fs::read(&output_path).await.unwrap();

                        let payload = UploadFilePayload {
                            name: chunk.name.clone(),
                            mime_type: chunk.mime_type.clone(),
                            data: file_data,
                        };

                        match do_upload(payload, &handle).await {
                            Ok(success_message) => {
                                handle
                                    .emit(
                                        "upload-file",
                                        UploadFileEvent {
                                            name: chunk.name.clone(),
                                            mime_type: chunk.mime_type.clone(),
                                            xorname: Some(success_message),
                                            success: true,
                                            error: None,
                                        },
                                    )
                                    .unwrap();
                            }
                            Err(error) => {
                                handle
                                    .emit(
                                        "upload-file",
                                        UploadFileEvent {
                                            name: chunk.name.clone(),
                                            mime_type: chunk.mime_type.clone(),
                                            success: false,
                                            xorname: None,
                                            error: Some(UploadError {
                                                title: "Upload Failed".into(),
                                                description: format!("{:?}", error),
                                            }),
                                        },
                                    )
                                    .unwrap();
                            }
                        }

                        store_guard.remove(&key);
                    }
                }
                Err(e) => {
                    eprintln!("Invalid chunk data: {}", e);
                }
            }
        }
    }
}

// not currently used as we directly use anttp/dweb
async fn handle_download_ws(ws: WebSocket, handle: AppHandle) {
    let (mut tx, mut rx) = ws.split();

    if let Some(Ok(msg)) = rx.next().await {
        if msg.is_text() {
            match serde_json::from_str::<DownloadRequest>(msg.to_str().unwrap()) {
                Ok(req) if req.action == "download" 
               // && is_valid_xorname(&req.xorname) 
                => {
                    // Immediately emit the success toast
                    handle
                        .emit(
                            "show-toast",
                            ToastEvent {
                                title: "Success".to_string(),
                                description: "Download request sent to client".to_string(),
                            },
                        )
                        .unwrap();

                    // Proceed with the file download as before
                    // match crate::download(
                    //     req.xorname.clone(),
                    //     Some("file_name.png".to_string()), // You can set the filename as needed
                    //     "downloads".to_string(),
                    //     handle,
                    // )
                    // .await
                    // {
                    //     Ok(Some(file_data)) => {
                    //         // Split the file into chunks (e.g., 1MB chunks)
                    //         const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks
                    //         let mut chunk_index = 0;
                    //         let total_chunks = (file_data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE; // Round up

                    //         while chunk_index < total_chunks {
                    //             let start = chunk_index * CHUNK_SIZE;
                    //             let end =
                    //                 std::cmp::min((chunk_index + 1) * CHUNK_SIZE, file_data.len());
                    //             let chunk_data = file_data[start..end].to_vec();

                    //             // Send the chunk over WebSocket
                    //             let chunk = json!({
                    //                 "chunk_index": chunk_index,
                    //                 "total_chunks": total_chunks,
                    //                 "data": chunk_data,
                    //             });

                    //             if tx.send(Message::text(chunk.to_string())).await.is_err() {
                    //                 break;
                    //             }

                    //             chunk_index += 1;
                    //         }

                    //         // Send a "done" message when all chunks are sent
                    //         let done_msg = json!({ "done": true }).to_string();
                    //         let _ = tx.send(Message::text(done_msg)).await;
                    //     }
                    //     Ok(None) => {
                    //         // File not found or failed to download
                    //         let error_response =
                    //             json!({ "error": "File not found or failed to download" });
                    //         let _ = tx.send(Message::text(error_response.to_string())).await;
                    //     }
                    //     Err(e) => {
                    //         // Download error
                    //         let error_response =
                    //             json!({ "error": format!("Download failed: {}", e) });
                    //         let _ = tx.send(Message::text(error_response.to_string())).await;
                    //     }
                    // }
                }
                _ => {
                    // Invalid action or xorname
                    let error_response = json!({ "error": "Invalid request format or xorname" });
                    let _ = tx.send(Message::text(error_response.to_string())).await;
                }
            }
        }
    }
}
