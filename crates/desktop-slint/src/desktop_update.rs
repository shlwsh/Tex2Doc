//! P9: Desktop update check adapter.
//!
//! The preview client only checks release manifests. Download, signature
//! verification keys, and platform installer execution remain P9 follow-up work.

use std::time::Duration;

use doc_commercial_api_client::{ApiClient, ApiError, ClientConfig};
use thiserror::Error;

use crate::updater::{self, SignatureStatus, UpdateDecision};

#[derive(Debug)]
pub struct DesktopUpdateCheck {
    pub decision: UpdateDecision,
    pub signature_status: SignatureStatus,
    pub sha256: String,
}

#[derive(Debug, Error)]
pub enum DesktopUpdateError {
    #[error("invalid API base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    #[error("updater error: {0}")]
    Updater(#[from] updater::UpdaterError),
}

pub type Result<T> = std::result::Result<T, DesktopUpdateError>;

pub fn check_update_blocking(
    base_url: &str,
    channel: &str,
    current_version: &str,
) -> Result<DesktopUpdateCheck> {
    let base_url = parse_base_url(base_url)?;
    let channel = normalized_channel(channel);
    let current_version = current_version.to_string();
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = ApiClient::new(ClientConfig {
            base_url,
            api_key: String::new(),
            timeout: Duration::from_secs(30),
        })?;
        let manifest = client.release_manifest(&channel).await?;
        let manifest = updater::ReleaseManifest {
            version: manifest.version,
            channel: manifest.channel,
            download_url: manifest.download_url,
            sha256: manifest.sha256,
            signature: manifest.signature,
            release_notes: manifest.release_notes,
        };
        updater::validate_manifest(&manifest)?;
        let signature_status = updater::verify_manifest_signature(&manifest)?;
        let latest_version = manifest.version.clone();
        let update_available = updater::is_newer_version(&current_version, &latest_version);
        let decision = UpdateDecision {
            current_version,
            latest_version,
            channel: manifest.channel,
            update_available,
            download_url: manifest.download_url,
            release_notes: manifest.release_notes,
        };
        Ok(DesktopUpdateCheck {
            decision,
            signature_status,
            sha256: manifest.sha256,
        })
    })
}

pub fn update_status_line(check: &DesktopUpdateCheck) -> String {
    let decision = &check.decision;
    let signature = match check.signature_status {
        SignatureStatus::Unsigned => "unsigned",
        SignatureStatus::DeferredVerification => "signature deferred",
    };
    let availability = if decision.update_available {
        "update available"
    } else {
        "up to date"
    };
    format!(
        "{} | current {} -> latest {} ({}) | sha256 {} | {} | {}",
        decision.channel,
        decision.current_version,
        decision.latest_version,
        availability,
        check.sha256,
        signature,
        decision.release_notes
    )
}

fn normalized_channel(channel: &str) -> String {
    let value = channel.trim();
    if value.is_empty() {
        "stable".to_string()
    } else {
        value.to_string()
    }
}

fn parse_base_url(value: &str) -> Result<url::Url> {
    value
        .trim()
        .parse()
        .map_err(|e: url::ParseError| DesktopUpdateError::InvalidBaseUrl(e.to_string()))
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DesktopUpdateError::Runtime(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_channel_defaults_to_stable() {
        assert_eq!(normalized_channel(""), "stable");
        assert_eq!(normalized_channel("  "), "stable");
        assert_eq!(normalized_channel("beta"), "beta");
    }

    #[test]
    fn update_status_mentions_current_and_latest_versions() {
        let check = DesktopUpdateCheck {
            decision: UpdateDecision {
                current_version: "0.1.0".to_string(),
                latest_version: "0.2.0".to_string(),
                channel: "stable".to_string(),
                update_available: true,
                download_url: "https://example.com/app".to_string(),
                release_notes: "Preview".to_string(),
            },
            signature_status: SignatureStatus::Unsigned,
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        };

        let line = update_status_line(&check);
        assert!(line.contains("current 0.1.0 -> latest 0.2.0"));
        assert!(line.contains("update available"));
        assert!(line.contains("unsigned"));
    }
}
