//! Application state for the Slint desktop client.
//!
//! Holds auth info, usage counters, job list, and current UI state.
//! Shared across all commands and UI callbacks.

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// P5: Application-wide state shared by all UI components.
#[derive(Debug)]
pub struct AppState {
    /// Current authentication token (None = not logged in).
    pub auth_token: RwLock<Option<String>>,
    /// Current refresh token (None = not logged in).
    pub refresh_token: RwLock<Option<String>>,
    /// Display name of the logged-in user.
    pub user_name: RwLock<Option<String>>,
    /// Conversion quota (None = unlimited / local mode).
    pub quota_remaining: RwLock<Option<usize>>,
    /// Current conversion jobs.
    pub jobs: RwLock<Vec<JobEntry>>,
}

/// P5: A single conversion job entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEntry {
    pub id: String,
    #[serde(default)]
    pub remote_job_id: Option<String>,
    pub project_path: String,
    pub profile: String,
    pub status: JobStatus,
    pub output_path: Option<String>,
    #[serde(default)]
    pub report_path: Option<String>,
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
            auth_token: RwLock::new(None),
            refresh_token: RwLock::new(None),
            user_name: RwLock::new(None),
            quota_remaining: RwLock::new(None),
            jobs: RwLock::new(Vec::new()),
        }
    }
}

impl AppState {
    pub fn set_account_session(
        &self,
        access_token: String,
        refresh_token: String,
        user_name: Option<String>,
        quota_remaining: Option<usize>,
    ) {
        if let Ok(mut token) = self.auth_token.write() {
            *token = Some(access_token);
        }
        if let Ok(mut token) = self.refresh_token.write() {
            *token = Some(refresh_token);
        }
        if let Ok(mut user) = self.user_name.write() {
            *user = user_name;
        }
        if let Ok(mut quota) = self.quota_remaining.write() {
            *quota = quota_remaining;
        }
    }

    pub fn auth_token(&self) -> Option<String> {
        self.auth_token.read().ok().and_then(|token| token.clone())
    }

    pub fn refresh_token(&self) -> Option<String> {
        self.refresh_token
            .read()
            .ok()
            .and_then(|token| token.clone())
    }

    pub fn set_refresh_token(&self, refresh_token: String) {
        if let Ok(mut token) = self.refresh_token.write() {
            *token = Some(refresh_token);
        }
    }

    pub fn clear_account_session(&self) {
        if let Ok(mut token) = self.auth_token.write() {
            *token = None;
        }
        if let Ok(mut token) = self.refresh_token.write() {
            *token = None;
        }
        if let Ok(mut user) = self.user_name.write() {
            *user = None;
        }
        if let Ok(mut quota) = self.quota_remaining.write() {
            *quota = None;
        }
    }

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
                    JobUpdate::Succeeded {
                        remote_job_id,
                        output_path,
                        report_path,
                    } => {
                        job.status = JobStatus::Succeeded;
                        job.remote_job_id = remote_job_id;
                        job.output_path = Some(output_path);
                        job.report_path = report_path;
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
        self.jobs
            .read()
            .ok()
            .map(|j| j.iter().take(10).cloned().collect())
            .unwrap_or_default()
    }

    pub fn all_jobs(&self) -> Vec<JobEntry> {
        self.jobs
            .read()
            .ok()
            .map(|jobs| jobs.clone())
            .unwrap_or_default()
    }

    pub fn remove_job(&self, id: &str) -> Option<JobEntry> {
        self.jobs.write().ok().and_then(|mut jobs| {
            jobs.iter()
                .position(|job| job.id == id)
                .map(|index| jobs.remove(index))
        })
    }

    pub fn clear_jobs(&self) -> Vec<JobEntry> {
        self.jobs
            .write()
            .ok()
            .map(|mut jobs| std::mem::take(&mut *jobs))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub enum JobUpdate {
    Running,
    Succeeded {
        remote_job_id: Option<String>,
        output_path: String,
        report_path: Option<String>,
    },
    Failed(String),
}
