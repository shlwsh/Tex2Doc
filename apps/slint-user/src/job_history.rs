//! P5: Persistent recent job history for the desktop client.
//!
//! This keeps the UI job list useful across restarts without coupling desktop
//! state to the conversion engines.

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;

use crate::app_state::{JobEntry, JobStatus};

const MAX_RECENT_JOBS: usize = 50;

pub fn load_recent_jobs() -> std::io::Result<Vec<JobEntry>> {
    let Some(path) = history_path() else {
        return Ok(Vec::new());
    };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let json = fs::read_to_string(path)?;
    let mut jobs: Vec<JobEntry> = serde_json::from_str(&json)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    normalize_loaded_jobs(&mut jobs);
    jobs.truncate(MAX_RECENT_JOBS);
    Ok(jobs)
}

pub fn save_recent_jobs(jobs: &[JobEntry]) -> std::io::Result<()> {
    let Some(path) = history_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let jobs = trimmed_jobs(jobs);
    let json = serde_json::to_string_pretty(&jobs).map_err(std::io::Error::other)?;
    fs::write(path, json)
}

fn history_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "tex2doc", "Tex2Doc")
        .map(|dirs| dirs.data_dir().join("recent_jobs.json"))
}

fn normalize_loaded_jobs(jobs: &mut [JobEntry]) {
    for job in jobs {
        if matches!(job.status, JobStatus::Pending | JobStatus::Running) {
            job.status = JobStatus::Failed;
            if job.error.is_none() {
                job.error = Some("Interrupted before completion.".to_string());
            }
        }
    }
}

fn trimmed_jobs(jobs: &[JobEntry]) -> Vec<JobEntry> {
    jobs.iter().take(MAX_RECENT_JOBS).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: &str, status: JobStatus) -> JobEntry {
        JobEntry {
            id: id.to_string(),
            remote_job_id: None,
            project_path: "/tmp/project".to_string(),
            profile: "auto".to_string(),
            status,
            output_path: None,
            report_path: None,
            error: None,
            created_at: "1d00h00m00s".to_string(),
        }
    }

    #[test]
    fn loaded_running_jobs_are_marked_failed() {
        let mut jobs = vec![job("1", JobStatus::Running), job("2", JobStatus::Pending)];

        normalize_loaded_jobs(&mut jobs);

        assert_eq!(jobs[0].status, JobStatus::Failed);
        assert_eq!(jobs[1].status, JobStatus::Failed);
        assert_eq!(
            jobs[0].error.as_deref(),
            Some("Interrupted before completion.")
        );
    }

    #[test]
    fn trim_keeps_at_most_recent_limit() {
        let jobs = (0..60)
            .map(|idx| job(&idx.to_string(), JobStatus::Succeeded))
            .collect::<Vec<_>>();

        let trimmed = trimmed_jobs(&jobs);

        assert_eq!(trimmed.len(), MAX_RECENT_JOBS);
        assert_eq!(trimmed[0].id, "0");
        assert_eq!(trimmed[49].id, "49");
    }
}
