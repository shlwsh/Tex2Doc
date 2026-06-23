//! P5: Settings persistence for the desktop client.
//!
//! Stores user preferences (default output dir, quality level, theme)
//! in a JSON file under the platform-appropriate config directory.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:8080/v1/";
const LEGACY_ONLINE_API_BASE_URL: &str = "https://api.tex2doc.cn/v1/";

/// P5: User settings for the desktop client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Commercial API base URL.
    pub api_base_url: String,
    /// Default output directory for conversions.
    pub output_dir: PathBuf,
    /// Default quality level (preview|standard|strict).
    pub quality: String,
    /// Default profile (auto or a specific ID).
    pub default_profile: String,
    /// Release channel for update checks.
    #[serde(default = "default_release_channel")]
    pub release_channel: String,
    /// UI locale code.
    #[serde(default = "default_locale")]
    pub locale: String,
    /// UI theme code.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Last used login email. Passwords and tokens are intentionally not stored here.
    pub last_login_email: Option<String>,
    /// Last used project path.
    pub last_project_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_base_url: DEFAULT_API_BASE_URL.to_string(),
            output_dir: dirs_default_output(),
            quality: "standard".to_string(),
            default_profile: "auto".to_string(),
            release_channel: default_release_channel(),
            locale: default_locale(),
            theme: default_theme(),
            last_login_email: None,
            last_project_path: None,
        }
    }
}

impl Settings {
    /// Load settings from the config file.
    pub fn load() -> Self {
        let mut settings: Self = config_path()
            .and_then(|p| fs::read_to_string(&p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        settings.locale = crate::i18n::normalize_locale(&settings.locale);
        settings.theme = crate::theme::normalize_theme(&settings.theme);
        settings.api_base_url = normalize_api_base_url(&settings.api_base_url);
        settings
    }

    /// Save settings to the config file.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    /// Update the last project path.
    #[allow(dead_code)]
    pub fn set_last_project(&mut self, path: String) {
        self.last_project_path = Some(path);
        let _ = self.save();
    }
}

fn normalize_api_base_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == LEGACY_ONLINE_API_BASE_URL {
        DEFAULT_API_BASE_URL.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Returns the config file path for the desktop app.
fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "tex2doc", "Tex2Doc")
        .map(|dirs| dirs.config_dir().join("settings.json"))
}

/// Returns the default output directory.
fn dirs_default_output() -> PathBuf {
    ProjectDirs::from("com", "tex2doc", "Tex2Doc")
        .map(|dirs| dirs.data_dir().join("output"))
        .unwrap_or_else(|| PathBuf::from("./output"))
}

fn default_release_channel() -> String {
    "stable".to_string()
}

fn default_locale() -> String {
    crate::i18n::DEFAULT_LOCALE.to_string()
}

fn default_theme() -> String {
    crate::theme::DEFAULT_THEME.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_release_channel_defaults_to_stable() {
        let json = r#"{
            "api_base_url": "http://127.0.0.1:8080/v1/",
            "output_dir": "/tmp/tex2doc",
            "quality": "standard",
            "default_profile": "auto",
            "last_login_email": null,
            "last_project_path": null
        }"#;

        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.release_channel, "stable");
        assert_eq!(settings.locale, "en");
        assert_eq!(settings.theme, crate::theme::DEFAULT_THEME);
    }

    #[test]
    fn legacy_online_api_base_url_migrates_to_local_demo_server() {
        assert_eq!(
            normalize_api_base_url("https://api.tex2doc.cn/v1/"),
            DEFAULT_API_BASE_URL
        );
        assert_eq!(
            normalize_api_base_url(" http://127.0.0.1:9000/v1/ "),
            "http://127.0.0.1:9000/v1/"
        );
    }
}
