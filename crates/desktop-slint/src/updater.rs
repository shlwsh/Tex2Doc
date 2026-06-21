//! P9: Desktop update manifest validation.
//!
//! This module intentionally stops before installing artifacts. The production
//! installer flow still needs platform-specific MSI/DMG/AppImage handling and
//! real signature verification keys.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    pub channel: String,
    pub download_url: String,
    pub sha256: String,
    pub signature: String,
    pub release_notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDecision {
    pub current_version: String,
    pub latest_version: String,
    pub channel: String,
    pub update_available: bool,
    pub download_url: String,
    pub release_notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    Unsigned,
    DeferredVerification,
}

#[derive(Debug, Error)]
pub enum UpdaterError {
    #[error("release manifest decode failed: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("release manifest is invalid: {0}")]
    InvalidManifest(String),
    #[error("sha256 mismatch: expected {expected}, actual {actual}")]
    Sha256Mismatch { expected: String, actual: String },
    #[error("update installation is not implemented for this preview build")]
    InstallUnsupported,
}

pub type Result<T> = std::result::Result<T, UpdaterError>;

pub fn parse_manifest(json: &str) -> Result<ReleaseManifest> {
    let manifest: ReleaseManifest = serde_json::from_str(json)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn check_update_from_manifest(
    current_version: &str,
    manifest_json: &str,
) -> Result<UpdateDecision> {
    let manifest = parse_manifest(manifest_json)?;
    Ok(UpdateDecision {
        current_version: current_version.to_string(),
        latest_version: manifest.version.clone(),
        channel: manifest.channel.clone(),
        update_available: is_newer_version(current_version, &manifest.version),
        download_url: manifest.download_url,
        release_notes: manifest.release_notes,
    })
}

pub fn validate_manifest(manifest: &ReleaseManifest) -> Result<()> {
    if manifest.version.trim().is_empty() {
        return Err(UpdaterError::InvalidManifest(
            "version is empty".to_string(),
        ));
    }
    if manifest.channel.trim().is_empty() {
        return Err(UpdaterError::InvalidManifest(
            "channel is empty".to_string(),
        ));
    }
    if manifest.download_url.trim().is_empty() {
        return Err(UpdaterError::InvalidManifest(
            "download_url is empty".to_string(),
        ));
    }
    if !is_valid_sha256_hex(&manifest.sha256) {
        return Err(UpdaterError::InvalidManifest(
            "sha256 must be 64 hex characters".to_string(),
        ));
    }
    Ok(())
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    compare_versions(latest, current).is_gt()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

pub fn verify_sha256(bytes: &[u8], expected_sha256: &str) -> Result<()> {
    let actual = sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected_sha256) {
        Ok(())
    } else {
        Err(UpdaterError::Sha256Mismatch {
            expected: expected_sha256.to_string(),
            actual,
        })
    }
}

pub fn verify_manifest_signature(manifest: &ReleaseManifest) -> Result<SignatureStatus> {
    validate_manifest(manifest)?;
    if manifest.signature.trim().is_empty() || manifest.signature.starts_with("pending-") {
        return Ok(SignatureStatus::Unsigned);
    }
    Ok(SignatureStatus::DeferredVerification)
}

pub fn download_and_verify_from_bytes(bytes: &[u8], expected_sha256: &str) -> Result<Vec<u8>> {
    verify_sha256(bytes, expected_sha256)?;
    Ok(bytes.to_vec())
}

pub fn install_update_placeholder() -> Result<()> {
    Err(UpdaterError::InstallUnsupported)
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let left_segments = version_segments(left);
    let right_segments = version_segments(right);
    let len = left_segments.len().max(right_segments.len());
    for idx in 0..len {
        let left_value = *left_segments.get(idx).unwrap_or(&0);
        let right_value = *right_segments.get(idx).unwrap_or(&0);
        match left_value.cmp(&right_value) {
            std::cmp::Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    std::cmp::Ordering::Equal
}

fn version_segments(version: &str) -> Vec<u64> {
    version
        .trim_start_matches('v')
        .split(|ch: char| ch == '.' || ch == '-' || ch == '+')
        .map(|segment| {
            segment
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect::<String>()
        })
        .map(|segment| segment.parse::<u64>().unwrap_or(0))
        .collect()
}

fn is_valid_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn detects_newer_versions() {
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.1.9", "0.2.0"));
        assert!(!is_newer_version("0.2.0", "0.2.0"));
        assert!(!is_newer_version("0.3.0", "0.2.9"));
    }

    #[test]
    fn parses_manifest_and_decides_update() {
        let json = format!(
            r#"{{
                "version": "0.2.0",
                "channel": "stable",
                "download_url": "https://releases.tex2doc.cn/desktop/0.2.0/app",
                "sha256": "{EMPTY_SHA256}",
                "signature": "",
                "release_notes": "Preview release"
            }}"#
        );

        let decision = check_update_from_manifest("0.1.0", &json).unwrap();
        assert!(decision.update_available);
        assert_eq!(decision.latest_version, "0.2.0");
        assert_eq!(decision.channel, "stable");
    }

    #[test]
    fn rejects_invalid_sha256() {
        let manifest = ReleaseManifest {
            version: "0.2.0".to_string(),
            channel: "stable".to_string(),
            download_url: "https://example.com/app".to_string(),
            sha256: "pending".to_string(),
            signature: String::new(),
            release_notes: String::new(),
        };

        assert!(matches!(
            validate_manifest(&manifest),
            Err(UpdaterError::InvalidManifest(_))
        ));
    }

    #[test]
    fn verifies_sha256() {
        verify_sha256(b"", EMPTY_SHA256).unwrap();
        assert!(matches!(
            verify_sha256(b"not empty", EMPTY_SHA256),
            Err(UpdaterError::Sha256Mismatch { .. })
        ));
    }

    #[test]
    fn signature_placeholder_reports_unsigned() {
        let manifest = ReleaseManifest {
            version: "0.2.0".to_string(),
            channel: "stable".to_string(),
            download_url: "https://example.com/app".to_string(),
            sha256: EMPTY_SHA256.to_string(),
            signature: "pending-p9-signature".to_string(),
            release_notes: String::new(),
        };

        assert_eq!(
            verify_manifest_signature(&manifest).unwrap(),
            SignatureStatus::Unsigned
        );
    }
}
