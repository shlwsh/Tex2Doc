use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::{FeedbackThreadRow, MainWindow};
use slint::{ComponentHandle, ModelRc, VecModel};

pub fn wire_feedback(ui: &MainWindow, app_state: Arc<AppState>) {
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_refresh_feedback_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let token = app_state_clone.auth_token();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = crate::cloud_account::fetch_feedback_threads_blocking(&base_url, token);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(rows) => {
                            let count = rows.len();
                            ui.set_feedback_threads(ModelRc::new(VecModel::from(
                                rows.into_iter()
                                    .map(feedback_row_for_ui)
                                    .collect::<Vec<_>>(),
                            )));
                            ui.set_feedback_status(
                                format!("Loaded {} feedback thread(s).", count).into(),
                            );
                        }
                        Err(error) => {
                            ui.set_feedback_status(format!("Feedback failed: {}", error).into())
                        }
                    }
                    ui.set_is_billing_busy(false);
                }
            });
            if let Err(error) = invoke_result {
                log::error!("Failed to update UI after feedback refresh: {}", error);
            }
        });
    });

    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_submit_feedback_clicked(
        move |api_base_url: slint::SharedString,
              feedback_type: slint::SharedString,
              title: slint::SharedString,
              message: slint::SharedString,
              job_id: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let feedback_type = feedback_type.to_string();
            let title = title.to_string();
            let message = message.to_string();
            let job_id = job_id.to_string();
            let token = app_state_clone.auth_token();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = crate::cloud_account::create_feedback_thread_blocking(
                    &base_url,
                    token,
                    &feedback_type,
                    &title,
                    &message,
                    &job_id,
                );
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(rows) => {
                                let count = rows.len();
                                ui.set_feedback_threads(ModelRc::new(VecModel::from(
                                    rows.into_iter()
                                        .map(feedback_row_for_ui)
                                        .collect::<Vec<_>>(),
                                )));
                                ui.set_feedback_title("".into());
                                ui.set_feedback_job_id("".into());
                                ui.set_feedback_message("".into());
                                ui.set_feedback_status(
                                    format!(
                                        "Feedback submitted. Loaded {} feedback thread(s).",
                                        count
                                    )
                                    .into(),
                                );
                            }
                            Err(error) => {
                                ui.set_feedback_status(format!("Submit failed: {}", error).into())
                            }
                        }
                        ui.set_is_billing_busy(false);
                    }
                });
                if let Err(error) = invoke_result {
                    log::error!("Failed to update UI after feedback submit: {}", error);
                }
            });
        },
    );
}

fn feedback_row_for_ui(row: crate::cloud_account::FeedbackTableRow) -> FeedbackThreadRow {
    FeedbackThreadRow {
        thread_id: row.thread_id.into(),
        title: row.title.into(),
        feedback_type: row.feedback_type.into(),
        status: row.status.into(),
        priority: row.priority.into(),
        message_count: row.message_count,
        latest_message_at: row.latest_message_at.into(),
        created_at: row.created_at.into(),
        conversion_job_id: row.conversion_job_id.into(),
    }
}
