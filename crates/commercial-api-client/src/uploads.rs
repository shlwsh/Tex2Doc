//! Project upload API methods.

use crate::client::ApiClient;
use crate::models::UploadResponse;
use crate::ApiError;

impl ApiClient {
    pub async fn upload_project_zip(
        &self,
        zip_bytes: Vec<u8>,
        file_name: impl Into<String>,
    ) -> Result<UploadResponse, ApiError> {
        let part = reqwest::multipart::Part::bytes(zip_bytes)
            .file_name(file_name.into())
            .mime_str("application/zip")
            .map_err(|e| ApiError::Transport(e.to_string()))?;
        let form = reqwest::multipart::Form::new().part("file", part);
        self.post_multipart("uploads", form).await
    }
}
