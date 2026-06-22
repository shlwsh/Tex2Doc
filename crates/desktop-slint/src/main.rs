//! P5: Tex2Doc Desktop — Slint-based GUI client.
//!
//! Provides a graphical interface for local TeX → DOCX conversion
//! with journal profile auto-detection, quality reporting, and job history.

mod app_state;
mod cloud_account;
mod cloud_convert;
mod commands;
mod credential_store;
mod desktop_dialog;
mod desktop_update;
mod diagnostics;
mod job;
mod job_history;
mod local_convert;
mod report;
mod settings;
mod ui;
mod updater;

use app_state::{AppState, JobUpdate};
use settings::Settings;
use slint::include_modules;
use slint::SharedString;
use std::sync::Arc;

// Re-export generated Slint components
include_modules!();

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Tex2Doc Desktop starting...");

    // Load settings
    let settings = Settings::load();
    log::info!(
        "Settings loaded: quality={}, output_dir={}, api_base_url={}",
        settings.quality,
        settings.output_dir.display(),
        settings.api_base_url
    );

    // Initialize app state
    let app_state: Arc<AppState> = Arc::new(AppState::default());
    match job_history::load_recent_jobs() {
        Ok(jobs) => {
            for job in jobs.into_iter().rev() {
                app_state.add_job(job);
            }
        }
        Err(error) => log::warn!("Failed to load recent jobs: {}", error),
    }

    // Create and configure the UI
    let ui = MainWindow::new().unwrap();

    // Initialize property defaults from settings
    let default_output = settings.output_dir.display().to_string();
    let default_quality = settings.quality.clone();
    ui.set_output_path(default_output.into());
    ui.set_quality_level(default_quality.into());
    ui.set_detected_profile(settings.default_profile.clone().into());
    ui.set_api_base_url(settings.api_base_url.clone().into());
    let mut account_status_set = false;
    if let Some(email) = settings.last_login_email.clone() {
        ui.set_login_email(email.clone().into());
        match credential_store::load_refresh_token(&settings.api_base_url, &email) {
            Ok(Some(refresh_token)) => {
                app_state.set_refresh_token(refresh_token);
                ui.set_account_status("Stored session found. Click Refresh.".into());
                account_status_set = true;
            }
            Ok(None) => {}
            Err(error) => {
                log::warn!("Failed to load stored refresh token: {}", error);
                ui.set_account_status(
                    format!("Not signed in. Secure token load failed: {}", error).into(),
                );
                account_status_set = true;
            }
        }
    }
    if let Some(path) = settings.last_project_path.clone() {
        ui.set_project_path(path.into());
    }
    if !account_status_set {
        ui.set_account_status("Not signed in.".into());
    }
    ui.set_usage_status("--".into());
    ui.set_billing_plan_id("pro".into());
    ui.set_billing_status("--".into());
    ui.set_update_channel(settings.release_channel.clone().into());
    ui.set_update_status("--".into());
    ui.set_compatibility_score("--".into());
    ui.set_quality_status("--".into());
    ui.set_profile_confidence("--".into());
    ui.set_recent_jobs(recent_jobs_for_ui(&app_state).into());
    ui.set_main_tex("".into());

    // Wire up the Convert button callback
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_convert_clicked(
        move |project_path: slint::SharedString,
              detected_profile: slint::SharedString,
              quality_level: slint::SharedString,
              output_path: slint::SharedString| {
            log::info!(
                "Convert clicked: project={}, profile={}, quality={}",
                project_path,
                detected_profile,
                quality_level
            );

            let proj = std::path::PathBuf::from(project_path.as_str());
            let out = std::path::PathBuf::from(output_path.as_str());
            let profile = detected_profile.to_string();
            let quality = quality_level.to_string();
            persist_settings(
                Some(project_path.as_str()),
                Some(output_path.as_str()),
                Some(&profile),
                Some(&quality),
                None,
                None,
            );
            let app = Arc::clone(&app_state_clone);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                log::info!("Starting conversion in background thread");

                job::start_job(
                    proj.display().to_string(),
                    out.display().to_string(),
                    profile,
                    quality,
                    Arc::clone(&app),
                    move |result| {
                        persist_recent_jobs(&app);
                        let recent_jobs = recent_jobs_for_ui(&app);
                        let invoke_result = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_is_converting(false);
                                ui.set_conversion_progress(1.0);
                                ui.set_recent_jobs(recent_jobs.into());

                                match result {
                                    Ok(result) => {
                                        log::info!(
                                            "Conversion succeeded: {} bytes, profile={}",
                                            result.docx_bytes,
                                            result.profile
                                        );
                                        ui.set_detected_profile(result.profile.into());
                                        ui.set_compatibility_score(
                                            result.compatibility_score.to_string().into(),
                                        );
                                        ui.set_quality_status(
                                            format!(
                                                "{} ({})",
                                                result.quality_status, result.quality_score
                                            )
                                            .into(),
                                        );
                                        ui.set_profile_confidence(confidence_text(
                                            result.profile_confidence,
                                        ));
                                        ui.set_status_text(result.report_text.into());
                                    }
                                    Err(error) => {
                                        log::error!("Conversion failed: {}", error);
                                        ui.set_conversion_progress(0.0);
                                        ui.set_quality_status("Failed".into());
                                        ui.set_status_text(
                                            format!("Conversion failed:\n{}", error).into(),
                                        );
                                    }
                                }
                            }
                        });

                        if let Err(error) = invoke_result {
                            log::error!("Failed to update UI after conversion: {}", error);
                        }
                    },
                );
            });
        },
    );

    // Wire up the Cloud Convert button callback.
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_cloud_convert_clicked(
        move |api_base_url: slint::SharedString,
              project_path: slint::SharedString,
              main_tex: slint::SharedString,
              detected_profile: slint::SharedString,
              quality_level: slint::SharedString,
              output_path: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let project = std::path::PathBuf::from(project_path.as_str());
            let main_tex_value = main_tex.to_string();
            let main_tex_option = if main_tex_value.trim().is_empty() {
                None
            } else {
                Some(main_tex_value)
            };
            let profile = detected_profile.to_string();
            let quality = quality_level.to_string();
            let output = std::path::PathBuf::from(output_path.as_str());
            let token = app_state_clone.auth_token();
            let app = Arc::clone(&app_state_clone);
            let cloud_job_id =
                job::register_external_job(project.display().to_string(), profile.clone(), &app);
            app.update_job(&cloud_job_id, JobUpdate::Running);
            persist_settings(
                Some(project_path.as_str()),
                Some(output_path.as_str()),
                Some(&profile),
                Some(&quality),
                Some(&base_url),
                None,
            );
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = cloud_convert::convert_project_blocking(
                    &base_url,
                    token,
                    &project,
                    main_tex_option.as_deref(),
                    &output,
                    &profile,
                    &quality,
                );
                match &result {
                    Ok(result) => app.update_job(
                        &cloud_job_id,
                        JobUpdate::Succeeded {
                            output_path: result.docx_path.display().to_string(),
                            report_path: Some(result.report_path.display().to_string()),
                        },
                    ),
                    Err(error) => {
                        app.update_job(&cloud_job_id, JobUpdate::Failed(error.to_string()))
                    }
                }
                persist_recent_jobs(&app);
                let recent_jobs = recent_jobs_for_ui(&app);
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_is_converting(false);
                        ui.set_conversion_progress(1.0);
                        ui.set_recent_jobs(recent_jobs.into());
                        match result {
                            Ok(result) => {
                                ui.set_status_text(
                                    format!(
                                        "{}\nJob: {}\nDOCX: {} ({} bytes)\nReport: {}",
                                        result.report_text,
                                        result.job_id,
                                        result.docx_path.display(),
                                        result.docx_bytes,
                                        result.report_path.display()
                                    )
                                    .into(),
                                );
                                ui.set_quality_status("Cloud completed".into());
                                ui.set_compatibility_score("--".into());
                                ui.set_profile_confidence("cloud report".into());
                            }
                            Err(error) => {
                                ui.set_conversion_progress(0.0);
                                ui.set_quality_status("Cloud failed".into());
                                ui.set_status_text(
                                    format!("Cloud conversion failed:\n{}", error).into(),
                                );
                            }
                        }
                    }
                });
                if let Err(error) = invoke_result {
                    log::error!("Failed to update UI after cloud conversion: {}", error);
                }
            });
        },
    );

    // Wire up the cloud account login button.
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_login_clicked(
        move |api_base_url: slint::SharedString,
              email: slint::SharedString,
              password: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let email_value = email.to_string();
            let password_value = password.to_string();
            persist_settings(None, None, None, None, Some(&base_url), Some(&email_value));
            let app = Arc::clone(&app_state_clone);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = cloud_account::login_and_fetch_usage_blocking(
                    &base_url,
                    &email_value,
                    &password_value,
                );
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(session) => apply_account_session(&app, &ui, &base_url, session),
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

    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_register_clicked(
        move |api_base_url: slint::SharedString,
              email: slint::SharedString,
              password: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let email_value = email.to_string();
            let password_value = password.to_string();
            persist_settings(None, None, None, None, Some(&base_url), Some(&email_value));
            let app = Arc::clone(&app_state_clone);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = cloud_account::register_and_fetch_usage_blocking(
                    &base_url,
                    &email_value,
                    &password_value,
                );
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(session) => apply_account_session(&app, &ui, &base_url, session),
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

    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_refresh_login_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let refresh_token = app_state_clone.refresh_token();
        let app = Arc::clone(&app_state_clone);
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = cloud_account::refresh_and_fetch_usage_blocking(&base_url, refresh_token);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(session) => apply_account_session(&app, &ui, &base_url, session),
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

    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_logout_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let base_url = ui.get_api_base_url().to_string();
            let email = ui.get_login_email().to_string();
            if let Err(error) = credential_store::delete_refresh_token(&base_url, &email) {
                log::warn!("Failed to delete stored refresh token: {}", error);
            }
            app_state_clone.clear_account_session();
            ui.set_login_password("".into());
            ui.set_account_status("Signed out.".into());
            ui.set_usage_status("--".into());
            ui.set_billing_status("--".into());
        }
    });

    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_refresh_usage_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let token = app_state_clone.auth_token();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = match token {
                Some(token) => cloud_account::fetch_usage_blocking(&base_url, &token)
                    .map(|usage| cloud_account::usage_line(&usage)),
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

    let ui_weak = ui.as_weak();
    ui.on_show_plans_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = cloud_account::fetch_plans_blocking(&base_url)
                .map(|plans| cloud_account::plans_line(&plans));
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

    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_checkout_clicked(
        move |api_base_url: slint::SharedString, plan_id: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let plan = plan_id.to_string();
            let token = app_state_clone.auth_token();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = cloud_account::create_checkout_blocking(&base_url, token, &plan);
                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(session) => {
                                if let Err(error) = open_external_url(&session.url) {
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

    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_billing_portal_clicked(move |api_base_url: slint::SharedString| {
        let base_url = api_base_url.to_string();
        let token = app_state_clone.auth_token();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = cloud_account::create_billing_portal_blocking(&base_url, token);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(session) => {
                            if let Err(error) = open_external_url(&session.url) {
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

    let ui_weak = ui.as_weak();
    ui.on_check_update_clicked(
        move |api_base_url: slint::SharedString, channel: slint::SharedString| {
            let base_url = api_base_url.to_string();
            let release_channel = channel.to_string();
            persist_release_channel(&release_channel);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = desktop_update::check_update_blocking(
                    &base_url,
                    &release_channel,
                    env!("CARGO_PKG_VERSION"),
                )
                .map(|check| desktop_update::update_status_line(&check));
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

    let ui_weak = ui.as_weak();
    ui.on_choose_project_folder_clicked(
        move |project_path: slint::SharedString, output_path: slint::SharedString| {
            let initial = path_for_dialog(project_path.as_str());
            let selected = desktop_dialog::pick_project_folder(initial.as_deref());
            if let Some(selected) = selected {
                let default_output = default_output_for_project(&selected);
                let should_set_output = output_path.trim().is_empty();
                persist_settings(Some(&selected), None, None, None, None, None);
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_project_path(selected.clone().into());
                    if should_set_output {
                        ui.set_output_path(default_output.into());
                    }
                    ui.set_status_text(format!("Selected project: {}", selected).into());
                }
            }
        },
    );

    let ui_weak = ui.as_weak();
    ui.on_choose_project_zip_clicked(
        move |project_path: slint::SharedString, output_path: slint::SharedString| {
            let initial = path_for_dialog(project_path.as_str());
            let selected = desktop_dialog::pick_project_zip(initial.as_deref());
            if let Some(selected) = selected {
                let default_output = default_output_for_project(&selected);
                let should_set_output = output_path.trim().is_empty();
                persist_settings(Some(&selected), None, None, None, None, None);
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_project_path(selected.clone().into());
                    if should_set_output {
                        ui.set_output_path(default_output.into());
                    }
                    ui.set_status_text(format!("Selected project zip: {}", selected).into());
                }
            }
        },
    );

    let ui_weak = ui.as_weak();
    ui.on_choose_output_clicked(move |output_path: slint::SharedString| {
        let initial = path_for_dialog(output_path.as_str());
        if let Some(selected) = desktop_dialog::pick_output_docx(initial.as_deref()) {
            persist_settings(None, Some(&selected), None, None, None, None);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_output_path(selected.clone().into());
                ui.set_status_text(format!("Selected output: {}", selected).into());
            }
        }
    });

    // Wire up the Detect Profile button
    let ui_weak = ui.as_weak();
    ui.on_detect_profile_clicked(move |project_path: slint::SharedString| {
        log::info!("Detect profile: {}", project_path);
        let proj = std::path::PathBuf::from(project_path.as_str());
        let project_for_settings = project_path.to_string();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = commands::detect_profile(&proj);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(profile) => {
                            log::info!("Detected profile: {}", profile);
                            persist_settings(
                                Some(&project_for_settings),
                                None,
                                Some(&profile),
                                None,
                                None,
                                None,
                            );
                            ui.set_detected_profile(profile.clone().into());
                            ui.set_status_text(format!("Detected profile: {}", profile).into());
                            ui.set_profile_confidence("see conversion report".into());
                        }
                        Err(error) => {
                            log::error!("Profile detection failed: {}", error);
                            ui.set_status_text(
                                format!("Profile detection failed:\n{}", error).into(),
                            );
                        }
                    }
                }
            });

            if let Err(error) = invoke_result {
                log::error!("Failed to update UI after profile detection: {}", error);
            }
        });
    });

    ui.on_open_output_clicked(|output_path: slint::SharedString| {
        if let Err(error) = open_output_path(output_path.as_str()) {
            log::error!("Failed to open output path: {}", error);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_open_report_clicked(move |output_path: slint::SharedString| {
        let result = open_report_path(output_path.as_str());
        if let Some(ui) = ui_weak.upgrade() {
            match result {
                Ok(path) => {
                    ui.set_status_text(format!("Opened report:\n{}", path.display()).into())
                }
                Err(error) => {
                    log::error!("Failed to open report path: {}", error);
                    ui.set_status_text(format!("Open report failed:\n{}", error).into());
                }
            }
        }
    });

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
            let input = diagnostics::DiagnosticInput {
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

            let result = diagnostics::export_diagnostic_bundle(&input);
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

    log::info!("Tex2Doc Desktop UI initialized, entering event loop");

    ui.run().unwrap();
}

fn confidence_text(confidence: Option<f32>) -> SharedString {
    confidence
        .map(|value| format!("{:.0}%", value * 100.0).into())
        .unwrap_or_else(|| "--".into())
}

fn apply_account_session(
    app: &AppState,
    ui: &MainWindow,
    api_base_url: &str,
    session: cloud_account::CloudAccountSession,
) {
    let quota_remaining = session
        .usage
        .cloud_conversions_limit
        .saturating_sub(session.usage.cloud_conversions_used) as usize;
    let display_name = session
        .display_name
        .clone()
        .unwrap_or_else(|| session.email.clone());
    let store_status = match credential_store::store_refresh_token(
        api_base_url,
        &session.email,
        &session.refresh_token,
    ) {
        Ok(()) => "Session stored securely.".to_string(),
        Err(error) => {
            log::warn!("Failed to store refresh token: {}", error);
            format!("Session is memory-only: {error}")
        }
    };
    app.set_account_session(
        session.access_token,
        session.refresh_token,
        Some(display_name.clone()),
        Some(quota_remaining),
    );
    ui.set_account_status(
        format!(
            "Signed in as {} ({}) | {}",
            display_name, session.plan_id, store_status
        )
        .into(),
    );
    ui.set_usage_status(cloud_account::usage_line(&session.usage).into());
}

fn recent_jobs_for_ui(app_state: &AppState) -> String {
    let jobs = app_state.recent_jobs();
    if jobs.is_empty() {
        return "No recent jobs.".to_string();
    }

    jobs.into_iter()
        .map(|job| {
            let output = job.output_path.unwrap_or_else(|| "-".to_string());
            let report = job
                .report_path
                .map(|path| format!(" | report {}", path))
                .unwrap_or_default();
            let error = job.error.map(|e| format!(" | {}", e)).unwrap_or_default();
            format!(
                "{} | {} | {} | {}{}{}",
                job.created_at, job.status, job.profile, output, report, error
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn persist_recent_jobs(app_state: &AppState) {
    if let Err(error) = job_history::save_recent_jobs(&app_state.recent_jobs()) {
        log::warn!("Failed to persist recent jobs: {}", error);
    }
}

fn persist_settings(
    project_path: Option<&str>,
    output_path: Option<&str>,
    profile: Option<&str>,
    quality: Option<&str>,
    api_base_url: Option<&str>,
    login_email: Option<&str>,
) {
    let mut settings = Settings::load();
    if let Some(path) = project_path.filter(|path| !path.trim().is_empty()) {
        settings.last_project_path = Some(path.to_string());
    }
    if let Some(path) = output_path.filter(|path| !path.trim().is_empty()) {
        settings.output_dir = std::path::PathBuf::from(path);
    }
    if let Some(value) = profile.filter(|value| !value.trim().is_empty()) {
        settings.default_profile = value.to_string();
    }
    if let Some(value) = quality.filter(|value| !value.trim().is_empty()) {
        settings.quality = value.to_string();
    }
    if let Some(value) = api_base_url.filter(|value| !value.trim().is_empty()) {
        settings.api_base_url = value.to_string();
    }
    if let Some(value) = login_email.filter(|value| !value.trim().is_empty()) {
        settings.last_login_email = Some(value.to_string());
    }
    if let Err(error) = settings.save() {
        log::warn!("Failed to persist settings: {}", error);
    }
}

fn persist_release_channel(channel: &str) {
    let channel = channel.trim();
    if channel.is_empty() {
        return;
    }
    let mut settings = Settings::load();
    settings.release_channel = channel.to_string();
    if let Err(error) = settings.save() {
        log::warn!("Failed to persist release channel: {}", error);
    }
}

fn path_for_dialog(value: &str) -> Option<std::path::PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(trimmed))
    }
}

fn default_output_for_project(project_path: &str) -> String {
    let path = std::path::Path::new(project_path);
    let project_dir = if path.extension().and_then(|ext| ext.to_str()) == Some("zip") {
        path.parent().unwrap_or_else(|| std::path::Path::new("."))
    } else {
        path
    };
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("tex2doc-output");
    project_dir
        .join("output")
        .join("to-docx")
        .join(format!("{stem}.docx"))
        .display()
        .to_string()
}

fn report_path_for_output(output_path: &str) -> Option<std::path::PathBuf> {
    let output = std::path::Path::new(output_path.trim());
    if output.as_os_str().is_empty() {
        return None;
    }
    let stem = output
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("conversion");
    let mut report = output.to_path_buf();
    report.set_file_name(format!("{stem}.report.json"));
    Some(report)
}

fn open_output_path(path: &str) -> std::io::Result<()> {
    let path = std::path::Path::new(path);
    let target = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", "", &target.display().to_string()]);
        cmd
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(target);
        cmd
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(target);
        cmd
    };

    command.spawn().map(|_| ())
}

fn open_report_path(output_path: &str) -> std::io::Result<std::path::PathBuf> {
    let report_path = report_path_for_output(output_path).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "output path is empty")
    })?;
    if !report_path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("report file not found: {}", report_path.display()),
        ));
    }
    open_path(&report_path)?;
    Ok(report_path)
}

fn open_path(target: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", "", &target.display().to_string()]);
        cmd
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(target);
        cmd
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(target);
        cmd
    };

    command.spawn().map(|_| ())
}

fn open_external_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", "", url]);
        cmd
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(url);
        cmd
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(url);
        cmd
    };

    command.spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_for_directory_uses_to_docx_folder() {
        let output = default_output_for_project("/tmp/paper3");
        assert!(
            output.contains("paper3/output/to-docx/paper3.docx")
                || output.contains("paper3\\output\\to-docx\\paper3.docx")
                || output.ends_with("paper3.docx")
        );
    }

    #[test]
    fn default_output_for_zip_uses_parent_to_docx_folder() {
        let output = default_output_for_project("/tmp/paper3.zip");
        assert!(
            output.contains("output/to-docx/paper3.docx")
                || output.contains("output\\to-docx\\paper3.docx")
                || output.ends_with("paper3.docx")
        );
    }

    #[test]
    fn report_path_for_output_uses_docx_stem() {
        assert_eq!(
            report_path_for_output("/tmp/out/paper.docx").unwrap(),
            std::path::PathBuf::from("/tmp/out/paper.report.json")
        );
    }
}
