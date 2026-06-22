use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::MainWindow;
use crate::ui_bindings::helpers;
use slint::{ComponentHandle, Model};

pub fn wire_billing(ui: &MainWindow, _app_state: Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_change_plan_clicked(move |plan_id: slint::SharedString| {
        let plan = plan_id.to_string();
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_billing_plan_id(plan.clone().into());
            let index = ui
                .get_plan_catalog()
                .iter()
                .position(|entry| entry.id == plan.clone())
                .map(|i| i as i32)
                .unwrap_or(0);
            ui.set_current_plan_index(index);
            ui.set_billing_status(
                format!("Plan set to {}. Use Checkout to activate.", plan).into(),
            );
        }
    });
}

pub fn wire_billing_cloud(ui: &MainWindow, app_state: Arc<AppState>) {
    // Show plans
    let ui_weak = ui.as_weak();
    ui.on_show_plans_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = crate::cloud_account::fetch_plans_blocking(&base_url)
                .map(|plans| crate::cloud_account::plans_line(&plans));
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(line) => ui.set_billing_status(line.into()),
                        Err(error) => {
                            ui.set_billing_status(format!("Plans failed: {}", error).into())
                        }
                    }
                }
            });
            if let Err(error) = invoke_result {
                log::error!("Failed to update UI after plans request: {}", error);
            }
        });
    });

    // Checkout
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_checkout_clicked(
        move |api_base_url: slint::SharedString, plan_id: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let plan = plan_id.to_string();
            let token = app_state_clone.auth_token();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result =
                    crate::cloud_account::create_checkout_blocking(&base_url, token, &plan);
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(session) => {
                                if let Err(error) = helpers::open_external_url(&session.url) {
                                    ui.set_billing_status(
                                        format!(
                                            "Checkout created but browser open failed: {}\n{}",
                                            error, session.url
                                        )
                                        .into(),
                                    );
                                } else {
                                    ui.set_billing_status(
                                        format!(
                                            "Checkout opened. Expires at {}",
                                            session.expires_at
                                        )
                                        .into(),
                                    );
                                }
                            }
                            Err(error) => {
                                ui.set_billing_status(format!("Checkout failed: {}", error).into())
                            }
                        }
                    }
                });
                if let Err(error) = invoke_result {
                    log::error!("Failed to update UI after checkout: {}", error);
                }
            });
        },
    );

    // Billing portal
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_billing_portal_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let token = app_state_clone.auth_token();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = crate::cloud_account::create_billing_portal_blocking(&base_url, token);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(session) => {
                            if let Err(error) = helpers::open_external_url(&session.url) {
                                ui.set_billing_status(
                                    format!(
                                        "Billing portal created but browser open failed: {}\n{}",
                                        error, session.url
                                    )
                                    .into(),
                                );
                            } else {
                                ui.set_billing_status(
                                    format!(
                                        "Billing portal opened. Expires at {}",
                                        session.expires_at
                                    )
                                    .into(),
                                );
                            }
                        }
                        Err(error) => ui
                            .set_billing_status(format!("Billing portal failed: {}", error).into()),
                    }
                }
            });
            if let Err(error) = invoke_result {
                log::error!("Failed to update UI after billing portal: {}", error);
            }
        });
    });
}
