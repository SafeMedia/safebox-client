#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct UploadFilePayload {
    pub name: String,
    pub mime_type: String,
    pub data: Vec<u8>, //file bytes
}

#[derive(serde::Serialize, Clone)]
pub struct UploadFileEvent {
    pub name: String,
    pub mime_type: String,
    pub success: bool,
    pub error: Option<UploadError>,
    pub xorname: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct UploadError {
    pub title: String,
    pub description: String,
}

#[derive(serde::Serialize, Clone)]
pub struct ToastEvent {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Chunk {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub metadata: ChunkMetadata,
    pub data: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ChunkMetadata {
    pub filename: String,
    pub mime_type: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub upload_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct DownloadRequest {
    pub action: String,
    pub xorname: String,
}
