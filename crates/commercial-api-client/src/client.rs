//! HTTP client for the Tex2Doc commercial API.

use std::time::Duration;

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

use crate::models::*;
use crate::ApiError;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: url::Url,
    pub api_key: String,
    pub timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: url::Url::parse("https://api.tex2doc.cn/v1").unwrap(),
            api_key: String::new(),
            timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    config: ClientConfig,
    http: Client,
}

impl ApiClient {
    pub fn new(config: ClientConfig) -> Result<Self, ApiError> {
        let http = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| ApiError::Transport(e.to_string()))?;
        Ok(Self { config, http })
    }

    pub fn from_api_key(api_key: impl Into<String>) -> Result<Self, ApiError> {
        Self::new(ClientConfig {
            api_key: api_key.into(),
            ..Default::default()
        })
    }

    async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R, ApiError> {
        let url = self.config.base_url.join(path).map_err(ApiError::Url)?;
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .map_err(|e| ApiError::Transport(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Http { status, body });
        }
        resp.json()
            .await
            .map_err(|e| ApiError::Decode(e.to_string()))
    }

    async fn post<R: Serialize + ?Sized, B: DeserializeOwned>(
        &self,
        path: &str,
        body: &R,
    ) -> Result<B, ApiError> {
        let url = self.config.base_url.join(path).map_err(ApiError::Url)?;
        let resp = self
            .http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::Transport(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Http { status, body });
        }
        resp.json()
            .await
            .map_err(|e| ApiError::Decode(e.to_string()))
    }

    /// Submit a DOCX for quality analysis.
    pub async fn submit_analysis(&self, docx: &[u8]) -> Result<AnalysisJob, ApiError> {
        let form = docx_to_form(docx)?;
        let url = self
            .config
            .base_url
            .join("/analysis/submit")
            .map_err(ApiError::Url)?;
        let resp = self
            .http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| ApiError::Transport(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Http { status, body });
        }
        resp.json()
            .await
            .map_err(|e| ApiError::Decode(e.to_string()))
    }

    /// Poll for analysis result.
    pub async fn get_analysis_result(&self, job_id: &str) -> Result<AnalysisResult, ApiError> {
        self.get(&format!("/analysis/{}", job_id)).await
    }
}

fn docx_to_form(docx: &[u8]) -> Result<reqwest::multipart::Form, ApiError> {
    let part = reqwest::multipart::Part::bytes(docx.to_vec())
        .file_name("document.docx")
        .mime_str(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        )
        .map_err(|e| ApiError::Transport(e.to_string()))?;
    Ok(reqwest::multipart::Form::new().part("file", part))
}
