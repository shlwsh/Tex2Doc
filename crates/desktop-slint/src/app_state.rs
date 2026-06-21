//! Application state for the Slint desktop client.
//!
//! Holds auth info, usage counters, job list, and current UI state.
//! Shared across all commands and UI callbacks.

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// P5: Application-wide state shared by all UI components.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// Current authentication token (None = not logged in).
    pub auth_token: Option<String>,
    /// Display name of the logged-in user.
    pub user_name: Option<String>,
    /// Conversion quota (None = unlimited / local mode).
    pub quota_remaining: Option<usize>,
    /// Current conversion jobs.
    pub jobs: RwLock<Vec<JobEntry>>,
}

/// P5: A single conversion job entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEntry {
    pub id: String,
    pub project_path: String,
    pub profile: String,
    pub status: JobStatus,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running => write!(f, "Running"),
            Self::Succeeded => write!(f, "Succeeded"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            auth_token: None,
            user_name: None,
            quota_remaining: None,
            jobs: RwLock::new(Vec::new()),
        }
    }
}

impl AppState {
    pub fn add_job(&self, entry: JobEntry) {
        if let Ok(mut jobs) = self.jobs.write() {
            jobs.insert(0, entry);
            // Keep only last 50 jobs
            if jobs.len() > 50 {
                jobs.truncate(50);
            }
        }
    }

    pub fn update_job(&self, id: &str, update: JobUpdate) {
        if let Ok(mut jobs) = self.jobs.write() {
            if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
                match update {
                    JobUpdate::Running => job.status = JobStatus::Running,
                    JobUpdate::Succeeded(path) => {
                        job.status = JobStatus::Succeeded;
                        job.output_path = Some(path);
                    }
                    JobUpdate::Failed(err) => {
                        job.status = JobStatus::Failed;
                        job.error = Some(err);
                    }
                }
            }
        }
    }

    pub fn recent_jobs(&self) -> Vec<JobEntry> {
        self.jobs.read().ok()
            .map(|j| j.iter().take(10).cloned().collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub enum JobUpdate {
    Running,
    Succeeded(String),
    Failed(String),
}
