use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::MainWindow;
use slint::ComponentHandle;

pub fn wire_diagnostics(ui: &MainWindow, _app_state: Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_export_diagnostics_clicked(
        move |project_path: slint::SharedString,
              output_path: slint::SharedString,
              api_base_url: slint::SharedString,
              profile: slint::SharedString,
              quality: slint::SharedString,
              status_text: slint::SharedString,
              recent_jobs: slint::SharedString,
              update_status: slint::SharedString| {
            let input = crate::diagnostics::DiagnosticInput {
                project_path: project_path.to_string(),
                output_path: output_path.to_string(),
                api_base_url: api_base_url.to_string(),
                profile: profile.to_string(),
                quality: quality.to_string(),
                status_text: status_text.to_string(),
                recent_jobs: recent_jobs.to_string(),
                update_status: update_status.to_string(),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
            };

            let result = crate::diagnostics::export_diagnostic_bundle(&input);
            if let Some(ui) = ui_weak.upgrade() {
                match result {
                    Ok(path) => ui.set_status_text(
                        format!("Diagnostic bundle exported:\n{}", path.display()).into(),
                    ),
                    Err(error) => {
                        ui.set_status_text(format!("Diagnostic export failed:\n{}", error).into())
                    }
                }
            }
        },
    );
}
