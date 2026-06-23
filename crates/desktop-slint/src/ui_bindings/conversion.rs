use std::sync::Arc;

use crate::app_state::{AppState, JobUpdate};
use crate::ui::MainWindow;
use crate::ui_bindings::helpers;
use slint::ComponentHandle;

pub fn wire_conversion(ui: &MainWindow, app_state: Arc<AppState>) {
    // Convert button
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
            helpers::persist_settings(
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

                crate::job::start_job(
                    proj.display().to_string(),
                    out.display().to_string(),
                    profile,
                    quality,
                    Arc::clone(&app),
                    move |result| {
                        helpers::persist_recent_jobs(&app);
                        let recent_jobs = helpers::recent_jobs_for_ui(&app);
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
                                        ui.set_profile_confidence(helpers::confidence_text(
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

    // Cloud Convert button
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
            let cloud_job_id = crate::job::register_external_job(
                project.display().to_string(),
                profile.clone(),
                &app,
            );
            app.update_job(&cloud_job_id, JobUpdate::Running);
            helpers::persist_settings(
                Some(project_path.as_str()),
                Some(output_path.as_str()),
                Some(&profile),
                Some(&quality),
                Some(&base_url),
                None,
            );
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let result = crate::cloud_convert::convert_project_blocking(
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
                helpers::persist_recent_jobs(&app);
                let recent_jobs = helpers::recent_jobs_for_ui(&app);
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

    // Detect Profile button
    let ui_weak = ui.as_weak();
    ui.on_detect_profile_clicked(move |project_path: slint::SharedString| {
        log::info!("Detect profile: {}", project_path);
        let proj = std::path::PathBuf::from(project_path.as_str());
        let project_for_settings = project_path.to_string();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = crate::commands::detect_profile(&proj);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(profile) => {
                            log::info!("Detected profile: {}", profile);
                            helpers::persist_settings(
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

    // Open Output button
    ui.on_open_output_clicked(|output_path: slint::SharedString| {
        if let Err(error) = helpers::open_output_path(output_path.as_str()) {
            log::error!("Failed to open output path: {}", error);
        }
    });

    // Open Report button
    let ui_weak = ui.as_weak();
    ui.on_open_report_clicked(move |output_path: slint::SharedString| {
        let result = helpers::open_report_path(output_path.as_str());
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
}
