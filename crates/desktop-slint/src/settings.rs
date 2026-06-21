//! P5: Settings persistence for the desktop client.
//!
//! Stores user preferences (default output dir, quality level, theme)
//! in a JSON file under the platform-appropriate config directory.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// P5: User settings for the desktop client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Default output directory for conversions.
    pub output_dir: PathBuf,
    /// Default quality level (preview|standard|strict).
    pub quality: String,
    /// Default profile (auto or a specific ID).
    pub default_profile: String,
    /// Last used project path.
    pub last_project_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: dirs_default_output(),
            quality: "standard".to_string(),
            default_profile: "auto".to_string(),
            last_project_path: None,
        }
    }
}

impl Settings {
    /// Load settings from the config file.
    pub fn load() -> Self {
        config_path()
            .and_then(|p| fs::read_to_string(&p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save settings to the config file.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    /// Update the last project path.
    pub fn set_last_project(&mut self, path: String) {
        self.last_project_path = Some(path);
        let _ = self.save();
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
        .map(|dirs| dirs.document_dir().join("Tex2Doc").join("output"))
        .unwrap_or_else(|| PathBuf::from("./output"))
}
