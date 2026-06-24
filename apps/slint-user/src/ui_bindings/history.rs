use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::MainWindow;
use crate::ui_bindings::helpers;
use slint::{ComponentHandle, VecModel};

pub fn wire_history(ui: &MainWindow, app_state: Arc<AppState>) {
    // Remove one job
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_remove_job_clicked(move |job_id: slint::SharedString| {
        let id = job_id.to_string();
        let app = Arc::clone(&app_state_clone);
        let removed = app.remove_job(&id);
        if removed.is_some() {
            helpers::persist_recent_jobs(&app);
            let history = helpers::job_history_for_ui(&app);
            let recent_jobs = helpers::recent_jobs_for_ui(&app);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_job_history(slint::ModelRc::new(VecModel::from(history)));
                ui.set_recent_jobs(recent_jobs.into());
                ui.set_selected_job_index(-1);
                ui.set_status_text(format!("Removed job: {}", id).into());
            }
        } else if let Some(ui) = ui_weak.upgrade() {
            ui.set_status_text(format!("Job not found: {}", id).into());
        }
    });

    // Clear all jobs
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_clear_jobs_clicked(move || {
        let app = Arc::clone(&app_state_clone);
        let removed = app.clear_jobs();
        helpers::persist_recent_jobs(&app);
        let history = helpers::job_history_for_ui(&app);
        let recent_jobs = helpers::recent_jobs_for_ui(&app);
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_job_history(slint::ModelRc::new(VecModel::from(history)));
            ui.set_recent_jobs(recent_jobs.into());
            ui.set_selected_job_index(-1);
            ui.set_status_text(format!("Cleared {} job(s).", removed.len()).into());
        }
    });
}
