//! P5: small credential-store adapter for refresh tokens.
//!
//! Tokens are intentionally kept out of `settings.json`. The preview client uses
//! platform facilities when available and falls back to an in-memory session if
//! no secure store command exists.

use std::process::Command;

use sha2::{Digest, Sha256};
use thiserror::Error;

const SERVICE: &str = "tex2doc-desktop";

#[derive(Debug, Error)]
pub enum CredentialStoreError {
    #[error("missing account context")]
    MissingAccount,
    #[error("credential store unavailable: {0}")]
    Unavailable(String),
    #[error("credential command failed: {0}")]
    Command(String),
    #[cfg(target_os = "windows")]
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CredentialStoreError>;

pub fn store_refresh_token(api_base_url: &str, email: &str, refresh_token: &str) -> Result<()> {
    let key = account_key(api_base_url, email)?;
    platform_store(&key, refresh_token)
}

pub fn load_refresh_token(api_base_url: &str, email: &str) -> Result<Option<String>> {
    let key = account_key(api_base_url, email)?;
    platform_load(&key)
}

pub fn delete_refresh_token(api_base_url: &str, email: &str) -> Result<()> {
    let key = account_key(api_base_url, email)?;
    platform_delete(&key)
}

fn account_key(api_base_url: &str, email: &str) -> Result<String> {
    let base = api_base_url.trim();
    let email = email.trim().to_ascii_lowercase();
    if base.is_empty() || email.is_empty() {
        return Err(CredentialStoreError::MissingAccount);
    }
    let mut hasher = Sha256::new();
    hasher.update(base.as_bytes());
    hasher.update(b"\0");
    hasher.update(email.as_bytes());
    let digest = hasher.finalize();
    Ok(format!("{SERVICE}-refresh-{:x}", digest))
}

#[cfg(target_os = "macos")]
fn platform_store(key: &str, value: &str) -> Result<()> {
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-a",
            key,
            "-s",
            SERVICE,
            "-w",
            value,
            "-U",
        ])
        .status()
        .map_err(|e| CredentialStoreError::Unavailable(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(CredentialStoreError::Command(format!(
            "security exited with {status}"
        )))
    }
}

#[cfg(target_os = "macos")]
fn platform_load(key: &str) -> Result<Option<String>> {
    let output = Command::new("security")
        .args(["find-generic-password", "-a", key, "-s", SERVICE, "-w"])
        .output()
        .map_err(|e| CredentialStoreError::Unavailable(e.to_string()))?;
    if output.status.success() {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!token.is_empty()).then_some(token))
    } else {
        Ok(None)
    }
}

#[cfg(target_os = "macos")]
fn platform_delete(key: &str) -> Result<()> {
    let _ = Command::new("security")
        .args(["delete-generic-password", "-a", key, "-s", SERVICE])
        .status()
        .map_err(|e| CredentialStoreError::Unavailable(e.to_string()))?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_store(key: &str, value: &str) -> Result<()> {
    let mut child = Command::new("secret-tool")
        .args([
            "store",
            "--label",
            "Tex2Doc refresh token",
            "service",
            SERVICE,
            "account",
            key,
        ])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CredentialStoreError::Unavailable(e.to_string()))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(value.as_bytes())
            .map_err(|e| CredentialStoreError::Command(e.to_string()))?;
    }
    let status = child
        .wait()
        .map_err(|e| CredentialStoreError::Command(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(CredentialStoreError::Command(format!(
            "secret-tool exited with {status}"
        )))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_load(key: &str) -> Result<Option<String>> {
    let output = Command::new("secret-tool")
        .args(["lookup", "service", SERVICE, "account", key])
        .output()
        .map_err(|e| CredentialStoreError::Unavailable(e.to_string()))?;
    if output.status.success() {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!token.is_empty()).then_some(token))
    } else {
        Ok(None)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_delete(key: &str) -> Result<()> {
    let _ = Command::new("secret-tool")
        .args(["clear", "service", SERVICE, "account", key])
        .status()
        .map_err(|e| CredentialStoreError::Unavailable(e.to_string()))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn platform_store(key: &str, value: &str) -> Result<()> {
    let path = windows_token_path(key)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let script = format!(
        "$s = ConvertTo-SecureString '{}' -AsPlainText -Force; \
         $s | ConvertFrom-SecureString | Set-Content -Path '{}'",
        ps_escape(value),
        ps_escape(&path.display().to_string())
    );
    run_powershell(&script).map(|_| ())
}

#[cfg(target_os = "windows")]
fn platform_load(key: &str) -> Result<Option<String>> {
    let path = windows_token_path(key)?;
    if !path.is_file() {
        return Ok(None);
    }
    let script = format!(
        "$s = Get-Content -Path '{}' | ConvertTo-SecureString; \
         $b = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($s); \
         [Runtime.InteropServices.Marshal]::PtrToStringBSTR($b)",
        ps_escape(&path.display().to_string())
    );
    let token = run_powershell(&script)?;
    Ok((!token.trim().is_empty()).then_some(token.trim().to_string()))
}

#[cfg(target_os = "windows")]
fn platform_delete(key: &str) -> Result<()> {
    let path = windows_token_path(key)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_token_path(key: &str) -> Result<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "tex2doc", "Tex2Doc")
        .map(|dirs| dirs.config_dir().join("tokens").join(format!("{key}.txt")))
        .ok_or_else(|| {
            CredentialStoreError::Unavailable("config directory unavailable".to_string())
        })
}

#[cfg(target_os = "windows")]
fn run_powershell(script: &str) -> Result<String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .map_err(|e| CredentialStoreError::Unavailable(e.to_string()))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(CredentialStoreError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
fn ps_escape(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_key_is_stable_for_same_account() {
        let left = account_key("https://api.tex2doc.cn/v1/", "USER@example.com").unwrap();
        let right = account_key("https://api.tex2doc.cn/v1/", "user@example.com").unwrap();
        assert_eq!(left, right);
        assert!(left.starts_with("tex2doc-desktop-refresh-"));
    }

    #[test]
    fn account_key_rejects_missing_context() {
        assert!(matches!(
            account_key("", "user@example.com"),
            Err(CredentialStoreError::MissingAccount)
        ));
        assert!(matches!(
            account_key("https://api.tex2doc.cn/v1/", ""),
            Err(CredentialStoreError::MissingAccount)
        ));
    }
}
