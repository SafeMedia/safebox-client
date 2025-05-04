#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct UploadFilePayload {
    pub name: String,
    pub mime_type: String,
    pub data: Vec<u8>, // File bytes
}

#[derive(serde::Serialize, Clone)]
pub struct UploadFileEvent {
    pub name: String,      // Make fields public
    pub mime_type: String, // Make fields public
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
    pub title: String,       // Make fields public
    pub description: String, // Make fields public
}
