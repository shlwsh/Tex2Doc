use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::MainWindow;
use crate::ui_bindings::helpers;
use slint::ComponentHandle;

pub fn wire_settings(ui: &MainWindow, _app_state: Arc<AppState>) {
    // Save Settings (Phase D.4)
    let ui_weak = ui.as_weak();
    ui.on_save_settings_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_settings_panel_state("saving".into());
            let upload_path = ui.get_upload_path().to_string();
            let output_path = ui.get_output_path().to_string();
            let profile = ui.get_detected_profile().to_string();
            let quality = ui.get_quality_level().to_string();
            let api_base_url = ui.get_api_base_url().to_string();
            let email = ui.get_login_email().to_string();
            let channel = ui.get_update_channel().to_string();
            helpers::persist_settings(
                Some(&upload_path),
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
