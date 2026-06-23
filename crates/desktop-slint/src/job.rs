//! P5: Job management for the desktop client.
//!
//! Manages the lifecycle of conversion jobs (pending → running → done).

use crate::app_state::{AppState, JobEntry, JobStatus, JobUpdate};
use crate::commands::{self, CommandResult, LocalConvertResult};
use std::path::Path;
use std::sync::Arc;
use std::thread;

/// P5: A background conversion job.
pub struct ConvertJob {
    pub id: String,
    pub project_path: String,
    pub output_path: String,
    pub profile: String,
    pub quality: String,
}

impl ConvertJob {
    /// Execute the conversion in a background thread.
    /// Calls `on_done` when complete with the result.
    pub fn spawn<F>(self, app_state: Arc<AppState>, on_done: F)
    where
        F: FnOnce(CommandResult<LocalConvertResult>) + Send + 'static,
    {
        let id = self.id.clone();
        app_state.update_job(&id, JobUpdate::Running);

        let project_path = self.project_path.clone();
        let output_path = self.output_path.clone();
        let profile = self.profile.clone();
        let quality = self.quality.clone();
        let app = Arc::clone(&app_state);

        thread::spawn(move || {
            let result = commands::run_local_convert(
                Path::new(&project_path),
                Path::new(&output_path),
                &profile,
                &quality,
                &app,
            );

            match &result {
                Ok(r) => app.update_job(
                    &id,
                    JobUpdate::Succeeded {
                        remote_job_id: None,
                        output_path: r.docx_path.display().to_string(),
                        report_path: Some(r.report_path.display().to_string()),
                    },
                ),
                Err(e) => app.update_job(&id, JobUpdate::Failed(e.to_string())),
            }

            on_done(result);
        });
    }
}

/// P5: Start a new conversion job.
pub fn start_job(
    project_path: String,
    output_path: String,
    profile: String,
    quality: String,
    app_state: Arc<AppState>,
    on_done: impl FnOnce(CommandResult<LocalConvertResult>) + Send + 'static,
) -> String {
    let id = commands::generate_job_id();

    // Register pending job
    let entry = JobEntry {
        id: id.clone(),
        remote_job_id: None,
        project_path: project_path.clone(),
        profile: profile.clone(),
        status: JobStatus::Pending,
        output_path: None,
        report_path: None,
        error: None,
        created_at: chrono_now_simple(),
    };
    app_state.add_job(entry);

    let job = ConvertJob {
        id: id.clone(),
        project_path,
        output_path,
        profile,
        quality,
    };
    job.spawn(Arc::clone(&app_state), on_done);

    id
}

/// P5: Register a job whose execution is managed outside this module.
pub fn register_external_job(
    project_path: String,
    profile: String,
    app_state: &AppState,
) -> String {
    let id = commands::generate_job_id();
    let entry = JobEntry {
        id: id.clone(),
        remote_job_id: None,
        project_path,
        profile,
        status: JobStatus::Pending,
        output_path: None,
        report_path: None,
        error: None,
        created_at: chrono_now_simple(),
    };
    app_state.add_job(entry);
    id
}

fn chrono_now_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let mins = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    format!("{}d{:02}h{:02}m{:02}s", days, hours, mins, seconds)
}
