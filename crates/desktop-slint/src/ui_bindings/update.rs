use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::MainWindow;
use crate::ui_bindings::helpers;
use slint::ComponentHandle;

pub fn wire_update(ui: &MainWindow, _app_state: Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_check_update_clicked(
        move |api_base_url: slint::SharedString, channel: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let release_channel = channel.to_string();
            helpers::persist_release_channel(&release_channel);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = crate::desktop_update::check_update_blocking(
                    &base_url,
                    &release_channel,
                    env!("CARGO_PKG_VERSION"),
                )
                .map(|check| crate::desktop_update::update_status_line(&check));
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(line) => ui.set_update_status(line.into()),
                            Err(error) => ui.set_update_status(
                                format!("Update check failed: {}", error).into(),
                            ),
                        }
                    }
                });
                if let Err(error) = invoke_result {
                    log::error!("Failed to update UI after update check: {}", error);
                }
            });
        },
    );
}
