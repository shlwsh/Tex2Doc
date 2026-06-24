use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::MainWindow;
use crate::ui_bindings::helpers;
use slint::ComponentHandle;

pub fn wire_settings(ui: &MainWindow, _app_state: Arc<AppState>) {
    // Choose project folder
    let ui_weak = ui.as_weak();
    ui.on_choose_project_folder_clicked(
        move |project_path: slint::SharedString, output_path: slint::SharedString| {
            let initial = helpers::path_for_dialog(project_path.as_str());
            let selected = crate::desktop_dialog::pick_project_folder(initial.as_deref());
            if let Some(selected) = selected {
                let default_output = helpers::default_output_for_project(&selected);
                let should_set_output = output_path.trim().is_empty();
                helpers::persist_settings(Some(&selected), None, None, None, None, None);
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_project_path(selected.clone().into());
                    if should_set_output {
                        ui.set_output_path(default_output.into());
                    }
                    ui.set_status_text(format!("Selected project: {}", selected).into());
                    ui.set_settings_dirty(true);
                }
            }
        },
    );

    // Choose project zip
    let ui_weak = ui.as_weak();
    ui.on_choose_project_zip_clicked(
        move |project_path: slint::SharedString, output_path: slint::SharedString| {
            let initial = helpers::path_for_dialog(project_path.as_str());
            let selected = crate::desktop_dialog::pick_project_zip(initial.as_deref());
            if let Some(selected) = selected {
                let default_output = helpers::default_output_for_project(&selected);
                let should_set_output = output_path.trim().is_empty();
                helpers::persist_settings(Some(&selected), None, None, None, None, None);
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_project_path(selected.clone().into());
                    if should_set_output {
                        ui.set_output_path(default_output.into());
                    }
                    ui.set_status_text(format!("Selected project zip: {}", selected).into());
                    ui.set_settings_dirty(true);
                }
            }
        },
    );

    // Choose output
    let ui_weak = ui.as_weak();
    ui.on_choose_output_clicked(move |output_path: slint::SharedString| {
        let initial = helpers::path_for_dialog(output_path.as_str());
        if let Some(selected) = crate::desktop_dialog::pick_output_docx(initial.as_deref()) {
            helpers::persist_settings(None, Some(&selected), None, None, None, None);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_output_path(selected.clone().into());
                ui.set_status_text(format!("Selected output: {}", selected).into());
                ui.set_settings_dirty(true);
            }
        }
    });

    // Save Settings (Phase D.4)
    let ui_weak = ui.as_weak();
    ui.on_save_settings_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_settings_panel_state("saving".into());
            let project_path = ui.get_project_path().to_string();
            let output_path = ui.get_output_path().to_string();
            let profile = ui.get_detected_profile().to_string();
            let quality = ui.get_quality_level().to_string();
            let api_base_url = ui.get_api_base_url().to_string();
            let email = ui.get_login_email().to_string();
            let channel = ui.get_update_channel().to_string();
            helpers::persist_settings(
                Some(&project_path),
                Some(&output_path),
                Some(&profile),
                Some(&quality),
                Some(&api_base_url),
                Some(&email),
            );
            helpers::persist_release_channel(&channel);
            let saved_at = chrono_like_now();
            ui.set_settings_saved_at(saved_at.into());
            ui.set_settings_dirty(false);
            ui.set_settings_panel_state("idle".into());
            ui.set_status_text("Settings saved.".into());
        }
    });
}

/// Local "now" string without bringing chrono in. Good enough for UI.
fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch={}s", secs)
}
