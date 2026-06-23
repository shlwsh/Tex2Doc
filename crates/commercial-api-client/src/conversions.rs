//! Cloud conversion API methods.

use crate::client::ApiClient;
use crate::models::{ConversionJob, ConversionReport, CreateConversionRequest};
use crate::ApiError;

impl ApiClient {
    pub async fn create_conversion(
        &self,
        request: &CreateConversionRequest,
    ) -> Result<ConversionJob, ApiError> {
        self.post("conversions", request).await
    }

    pub async fn get_conversion(&self, job_id: &str) -> Result<ConversionJob, ApiError> {
        self.get(&format!("conversions/{job_id}")).await
    }

    pub async fn conversions(&self) -> Result<Vec<ConversionJob>, ApiError> {
        self.get("conversions").await
    }

    pub async fn download_conversion_docx(&self, job_id: &str) -> Result<Vec<u8>, ApiError> {
        self.get_bytes(&format!("conversions/{job_id}/download/docx"))
            .await
    }

    pub async fn get_conversion_report(&self, job_id: &str) -> Result<ConversionReport, ApiError> {
        self.get(&format!("conversions/{job_id}/report")).await
    }

    /// Download the original ZIP uploaded for a conversion job.
    pub async fn download_conversion_zip(&self, job_id: &str) -> Result<Vec<u8>, ApiError> {
        self.get_bytes(&format!("conversions/{job_id}/download/zip"))
            .await
    }

    /// Download the conversion log for a job.
    pub async fn download_conversion_log(&self, job_id: &str) -> Result<Vec<u8>, ApiError> {
        self.get_bytes(&format!("conversions/{job_id}/download/log"))
            .await
    }
}
