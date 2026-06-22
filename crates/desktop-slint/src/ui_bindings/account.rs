use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::MainWindow;
use crate::ui_bindings::helpers;
use slint::ComponentHandle;

pub fn wire_account(ui: &MainWindow, app_state: Arc<AppState>) {
    // Login button
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_login_clicked(
        move |api_base_url: slint::SharedString,
              email: slint::SharedString,
              password: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let email_value = email.to_string();
            let password_value = password.to_string();
            helpers::persist_settings(None, None, None, None, Some(&base_url), Some(&email_value));
            let app = Arc::clone(&app_state_clone);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = crate::cloud_account::login_and_fetch_usage_blocking(
                    &base_url,
                    &email_value,
                    &password_value,
                );
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(session) => {
                                helpers::apply_account_session(&app, &ui, &base_url, session)
                            }
                            Err(error) => {
                                ui.set_account_status(format!("Login failed: {}", error).into());
                                ui.set_usage_status("--".into());
                            }
                        }
                    }
                });
                if let Err(error) = invoke_result {
                    log::error!("Failed to update UI after login: {}", error);
                }
            });
        },
    );

    // Register button
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_register_clicked(
        move |api_base_url: slint::SharedString,
              email: slint::SharedString,
              password: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let email_value = email.to_string();
            let password_value = password.to_string();
            helpers::persist_settings(None, None, None, None, Some(&base_url), Some(&email_value));
            let app = Arc::clone(&app_state_clone);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = crate::cloud_account::register_and_fetch_usage_blocking(
                    &base_url,
                    &email_value,
                    &password_value,
                );
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(session) => {
                                helpers::apply_account_session(&app, &ui, &base_url, session)
                            }
                            Err(error) => {
                                ui.set_account_status(format!("Register failed: {}", error).into());
                                ui.set_usage_status("--".into());
                            }
                        }
                    }
                });
                if let Err(error) = invoke_result {
                    log::error!("Failed to update UI after registration: {}", error);
                }
            });
        },
    );

    // Refresh login button
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_refresh_login_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let refresh_token = app_state_clone.refresh_token();
        let app = Arc::clone(&app_state_clone);
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result =
                crate::cloud_account::refresh_and_fetch_usage_blocking(&base_url, refresh_token);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(session) => {
                            helpers::apply_account_session(&app, &ui, &base_url, session)
                        }
                        Err(error) => {
                            ui.set_account_status(format!("Refresh failed: {}", error).into());
                            ui.set_usage_status("--".into());
                        }
                    }
                }
            });
            if let Err(error) = invoke_result {
                log::error!("Failed to update UI after login refresh: {}", error);
            }
        });
    });

    // Logout button
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_logout_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let base_url = ui.get_api_base_url().to_string();
            let email = ui.get_login_email().to_string();
            if let Err(error) = crate::credential_store::delete_refresh_token(&base_url, &email) {
                log::warn!("Failed to delete stored refresh token: {}", error);
            }
            app_state_clone.clear_account_session();
            ui.set_login_password("".into());
            ui.set_account_status("Signed out.".into());
            ui.set_usage_status("--".into());
            ui.set_billing_status("--".into());
            // Phase D.1: reset structured account state
            ui.set_is_signed_in(false);
            ui.set_account_display_name("Guest".into());
            ui.set_account_tier("free".into());
            ui.set_quota_remaining(0);
            ui.set_quota_total(0);
        }
    });

    // Usage button
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_refresh_usage_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let token = app_state_clone.auth_token();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = match token {
                Some(token) => crate::cloud_account::fetch_usage_blocking(&base_url, &token)
                    .map(|usage| {
                        let line = crate::cloud_account::usage_line(&usage);
                        // Phase D.1: structured counters
                        (
                            line,
                            usage.cloud_conversions_used,
                            usage.cloud_conversions_limit,
                        )
                    })
                    .map(|(line, _used, _limit)| line),
                None => Ok("Sign in before refreshing usage.".to_string()),
            };
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(line) => ui.set_usage_status(line.into()),
                        Err(error) => {
                            ui.set_usage_status(format!("Usage refresh failed: {}", error).into())
                        }
                    }
                }
            });
            if let Err(error) = invoke_result {
                log::error!("Failed to update UI after usage refresh: {}", error);
            }
        });
    });
}
