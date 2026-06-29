use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::{CloudConversionRow, MainWindow, RechargeRow};
use crate::ui_bindings::helpers;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

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

    // Redeem code
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_redeem_code_clicked(
        move |api_base_url: slint::SharedString, redeem_code: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let code = redeem_code.to_string();
            let token = app_state_clone.auth_token();
            let app = Arc::clone(&app_state_clone);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = crate::cloud_account::redeem_code_blocking(&base_url, token, &code);
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok((redeemed, usage)) => {
                                let remaining = usage
                                    .cloud_conversions_limit
                                    .saturating_sub(usage.cloud_conversions_used);
                                app.set_quota_remaining(Some(remaining as usize));
                                ui.set_quota_remaining(remaining as i32);
                                ui.set_quota_total(usage.cloud_conversions_limit as i32);
                                ui.set_usage_status(
                                    crate::cloud_account::usage_line(&usage).into(),
                                );
                                ui.set_billing_status(
                                    format!(
                                        "Redeemed {}: +{} conversions, count balance {}.",
                                        redeemed.package_name,
                                        redeemed.quantity,
                                        redeemed.count_balance
                                    )
                                    .into(),
                                );

                                // Persist the redeem code and anonymous email so it fetches on startup
                                let anonymous_email = format!("t2d-code-{}@anonymous.local", code);
                                crate::ui_bindings::helpers::persist_redeem_code(
                                    &code,
                                    &base_url,
                                    &anonymous_email,
                                );
                            }
                            Err(error) => {
                                ui.set_billing_status(format!("Redeem failed: {}", error).into())
                            }
                        }
                        ui.set_is_billing_busy(false);
                    }
                });
                if let Err(error) = invoke_result {
                    log::error!("Failed to update UI after redeem code: {}", error);
                }
            });
        },
    );

    // Cloud conversion records table
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_refresh_conversion_records_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let token = app_state_clone.auth_token();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = crate::cloud_account::fetch_conversion_table_blocking(&base_url, token);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(rows) => {
                            let rows = rows
                                .into_iter()
                                .map(|row| CloudConversionRow {
                                    id: row.id.into(),
                                    main_tex: row.main_tex.into(),
                                    profile: row.profile.into(),
                                    status: row.status.into(),
                                    updated_at: row.updated_at.into(),
                                    error: row.error.into(),
                                    has_zip: row.has_zip,
                                    has_docx: row.has_docx,
                                    has_log: row.has_log,
                                    docx_size: row.docx_size.into(),
                                    zip_size: row.zip_size.into(),
                                    log_size: row.log_size.into(),
                                })
                                .collect::<Vec<_>>();
                            let count = rows.len();
                            ui.set_cloud_conversion_records(ModelRc::new(VecModel::from(rows)));
                            ui.set_status_text(
                                format!("Loaded {} cloud conversion record(s).", count).into(),
                            );
                        }
                        Err(error) => ui.set_status_text(
                            format!("Conversion records failed: {}", error).into(),
                        ),
                    }
                    ui.set_is_billing_busy(false);
                }
            });
            if let Err(error) = invoke_result {
                log::error!("Failed to update UI after conversion records: {}", error);
            }
        });
    });

    // Recharge and redeem records table
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_refresh_recharge_records_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let token = app_state_clone.auth_token();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = crate::cloud_account::fetch_recharge_table_blocking(&base_url, token);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(rows) => {
                            let rows = rows
                                .into_iter()
                                .map(|row| RechargeRow {
                                    id: row.id.into(),
                                    kind: row.kind.into(),
                                    package: row.package.into(),
                                    quantity: row.quantity.into(),
                                    status: row.status.into(),
                                    provider: row.provider.into(),
                                    created_at: row.created_at.into(),
                                })
                                .collect::<Vec<_>>();
                            let count = rows.len();
                            ui.set_recharge_records(ModelRc::new(VecModel::from(rows)));
                            ui.set_billing_status(
                                format!("Loaded {} recharge record(s).", count).into(),
                            );
                        }
                        Err(error) => ui.set_billing_status(
                            format!("Recharge records failed: {}", error).into(),
                        ),
                    }
                    ui.set_is_billing_busy(false);
                }
            });
            if let Err(error) = invoke_result {
                log::error!("Failed to update UI after recharge records: {}", error);
            }
        });
    });
}
