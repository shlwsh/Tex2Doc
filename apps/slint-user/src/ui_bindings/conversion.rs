use std::sync::Arc;

use crate::app_state::{AppState, JobUpdate};
use crate::ui::MainWindow;
use crate::ui_bindings::helpers;
use slint::{ComponentHandle, Model, SharedString, VecModel};

// ============================================================
// Log Panel Helpers
// ============================================================

/// Append a log entry to the UI log panel
pub fn append_log_to_ui(ui: &MainWindow, message: &str) {
    let timestamp = chrono_lite_timestamp();
    let log_entry = format!("[{}] {}", timestamp, message);

    let entries = ui.get_log_entries();
    let current_len = entries.iter().count();
    let mut vec: Vec<SharedString> = entries.iter().collect();
    vec.push(log_entry.into());

    // Keep max 1000 entries
    if current_len >= 1000 {
        vec = vec.into_iter().skip(1).collect();
    }

    ui.set_log_entries(std::rc::Rc::new(VecModel::from(vec)).into());
}

fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let secs = secs % 60;
    let millis = now.subsec_millis();
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

// ============================================================
// Quality Report Types - aligned with Slint QualityReportSummary
// ============================================================

#[allow(dead_code)]
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct QualityDimensions {
    pub parse: u8,
    pub semantic: u8,
    pub docx: u8,
    pub visual: u8,
    pub editable: u8,
    pub performance: u8,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticLossItem {
    pub loss_type: String,
    pub severity: String,
    pub location: String,
    pub description: String,
    pub suggestion: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WordCompatibilityInfo {
    pub status: String,
    pub errors: Vec<String>,
    pub check_method: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualityReportSummary {
    pub job_id: String,
    pub engine_version: String,
    pub profile: String,
    pub quality_score: u8,
    pub dimension_scores: QualityDimensions,
    pub word_compatibility: WordCompatibilityInfo,
    pub blocking_issues: Vec<SemanticLossItem>,
    pub warnings: Vec<SemanticLossItem>,
    pub semantic_loss_events: Vec<SemanticLossItem>,
    pub style_coverage_rate: f64,
    pub visual_diff_percentage: f64,
    pub created_at: String,
}

impl Default for WordCompatibilityInfo {
    fn default() -> Self {
        Self {
            status: "unchecked".to_string(),
            errors: Vec::new(),
            check_method: "none".to_string(),
        }
    }
}

impl Default for QualityReportSummary {
    fn default() -> Self {
        Self {
            job_id: String::new(),
            engine_version: String::new(),
            profile: String::new(),
            quality_score: 0,
            dimension_scores: QualityDimensions::default(),
            word_compatibility: WordCompatibilityInfo::default(),
            blocking_issues: Vec::new(),
            warnings: Vec::new(),
            semantic_loss_events: Vec::new(),
            style_coverage_rate: 0.0,
            visual_diff_percentage: 0.0,
            created_at: String::new(),
        }
    }
}

pub fn wire_conversion(ui: &MainWindow, app_state: Arc<AppState>) {
    // Convert button (local engine, upload-based)
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    let ui_weak_cb = ui_weak.clone();
    ui.on_convert_clicked(
        move |upload_path: slint::SharedString,
              main_tex: slint::SharedString,
              detected_profile: slint::SharedString,
              quality_level: slint::SharedString,
              output_dir: slint::SharedString| {
            log::info!(
                "Convert clicked: upload={}, main_tex={}, profile={}, quality={}",
                upload_path,
                main_tex,
                detected_profile,
                quality_level
            );

            let upload = upload_path
                .to_string()
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            let main_tex_str = main_tex
                .to_string()
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            let profile = detected_profile.to_string();
            let quality = quality_level.to_string();
            let out_dir = output_dir.to_string();
            helpers::persist_settings(
                Some(&upload),
                Some(&out_dir),
                Some(&profile),
                Some(&quality),
                None,
                None,
            );
            let base_url = if let Some(ui_instance) = ui_weak_cb.upgrade() {
                let url = ui_instance.get_api_base_url().to_string();
                if url.is_empty() {
                    "http://127.0.0.1:2624/v1/".to_string()
                } else {
                    url
                }
            } else {
                "http://127.0.0.1:2624/v1/".to_string()
            };
            let token = app_state_clone.auth_token();
            let app = Arc::clone(&app_state_clone);
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                // Clone variables for logging
                let upload_clone = upload.clone();
                let out_dir_clone = out_dir.clone();

                // Log: Starting conversion
                {
                    let ui_weak_for_log = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_for_log.upgrade() {
                            append_log_to_ui(&ui, "=== 开始转换 (Starting conversion) ===");
                            append_log_to_ui(&ui, &format!("文件: {}", upload_clone));
                            append_log_to_ui(&ui, &format!("输出目录: {}", out_dir_clone));
                            ui.set_status_text("正在读取文件...".into());
                        }
                    });
                }

                log::info!("Starting local conversion in background thread");

                // Use default URL if not set
                let effective_base_url = if base_url.is_empty() {
                    "http://127.0.0.1:2624/v1/".to_string()
                } else {
                    base_url.clone()
                };

                // Read upload file bytes
                let bytes = match std::fs::read(&upload) {
                    Ok(b) => b,
                    Err(e) => {
                        let invoke_result = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_is_converting(false);
                                ui.set_conversion_progress(0.0);
                                ui.set_quality_status("Failed".into());
                                ui.set_status_text(
                                    format!("Failed to read upload file: {}", e).into(),
                                );
                            }
                        });
                        let _ = invoke_result;
                        return;
                    }
                };

                let file_name = std::path::Path::new(&upload)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("upload");

                // Log: Starting local engine
                {
                    let ui_weak_for_log = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_for_log.upgrade() {
                            append_log_to_ui(&ui, "正在运行本地转换引擎...");
                            ui.set_status_text("正在运行本地转换引擎...".into());
                        }
                    });
                }

                let result = crate::cloud_convert::convert_local_blocking(
                    &effective_base_url,
                    token.clone(),
                    &bytes,
                    file_name,
                    &main_tex_str,
                    std::path::Path::new(&out_dir),
                    &profile,
                    &quality,
                );

                // Extract result info before passing to closures
                let result_summary = match &result {
                    Ok(r) => format!(
                        "转换成功! DOCX大小: {} bytes\n输出文件: {}",
                        r.docx_bytes,
                        r.docx_path.display()
                    ),
                    Err(e) => format!("转换失败: {}", e),
                };

                // Log: Conversion result
                {
                    let ui_weak_for_log = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_for_log.upgrade() {
                            for line in result_summary.lines() {
                                append_log_to_ui(&ui, line);
                            }
                        }
                    });
                }

                let usage_res = if let Some(ref t) = token {
                    crate::cloud_account::fetch_usage_blocking(&effective_base_url, t).ok()
                } else {
                    None
                };

                let job_id = crate::commands::generate_job_id();
                match &result {
                    Ok(r) => app.update_job(
                        &job_id,
                        JobUpdate::Succeeded {
                            remote_job_id: None,
                            output_path: r.docx_path.display().to_string(),
                            report_path: Some(r.report_path.display().to_string()),
                        },
                    ),
                    Err(e) => {
                        app.update_job(&job_id, JobUpdate::Failed(e.to_string()));
                    }
                }
                helpers::persist_recent_jobs(&app);
                let recent_jobs = helpers::recent_jobs_for_ui(&app);

                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_is_converting(false);
                        ui.set_conversion_progress(1.0);
                        ui.set_recent_jobs(recent_jobs.into());

                        if let Some(usage) = usage_res {
                            let remaining = usage
                                .cloud_conversions_limit
                                .saturating_sub(usage.cloud_conversions_used)
                                as i32;
                            let total = usage.cloud_conversions_limit as i32;
                            ui.set_quota_remaining(remaining);
                            ui.set_quota_total(total);
                            ui.set_usage_status(crate::cloud_account::usage_line(&usage).into());
                            app.set_quota_remaining(Some(remaining as usize));
                        }

                        match &result {
                            Ok(r) => {
                                log::info!(
                                    "Local conversion succeeded: {} bytes, profile={}",
                                    r.docx_bytes,
                                    r.profile
                                );
                                ui.set_detected_profile(r.profile.clone().into());
                                ui.set_quality_status(
                                    format!("{} ({})", r.quality_status, r.quality_score).into(),
                                );
                                ui.set_quality_progress(
                                    r.quality_score
                                        .parse::<f32>()
                                        .map(|s| (s / 100.0).clamp(0.0, 1.0))
                                        .unwrap_or(0.0),
                                );
                                ui.set_status_text(r.report_text.clone().into());
                            }
                            Err(error) => {
                                log::error!("Local conversion failed: {}", error);
                                ui.set_conversion_progress(0.0);
                                ui.set_quality_status("Failed".into());
                                ui.set_quality_progress(0.0);
                                ui.set_status_text(format!("Conversion failed:\n{}", error).into());
                            }
                        }
                    }
                });

                if let Err(e) = invoke_result {
                    log::error!("Failed to update UI after conversion: {}", e);
                }
            });
        },
    );

    // Cloud Convert button (upload-based cloud flow)
    let app_state_clone = Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_cloud_convert_clicked(
        move |api_base_url: slint::SharedString,
              upload_path: slint::SharedString,
              main_tex: slint::SharedString,
              detected_profile: slint::SharedString,
              quality_level: slint::SharedString,
              output_dir: slint::SharedString| {
            // Use default URL if not set
            let base_url = {
                let url = api_base_url.to_string();
                if url.is_empty() {
                    "http://127.0.0.1:2624/v1/".to_string()
                } else {
                    url
                }
            };
            let upload = upload_path.to_string().trim().trim_matches('"').trim_matches('\'').to_string();
            let main_tex_str = main_tex.to_string().trim().trim_matches('"').trim_matches('\'').to_string();
            let profile = detected_profile.to_string();
            let quality = quality_level.to_string();
            let out_dir = std::path::PathBuf::from(output_dir.as_str());
            let token = app_state_clone.auth_token();

            let file_name = std::path::Path::new(&upload)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("upload")
                .to_string();

            let cloud_job_id = crate::commands::generate_job_id();
            let entry = crate::app_state::JobEntry {
                id: cloud_job_id.clone(),
                remote_job_id: None,
                project_path: upload.clone(),
                profile: profile.clone(),
                status: crate::app_state::JobStatus::Pending,
                output_path: None,
                report_path: None,
                error: None,
                created_at: crate::job::chrono_now_simple(),
            };
            app_state_clone.add_job(entry);
            app_state_clone.update_job(&cloud_job_id, JobUpdate::Running);

            helpers::persist_settings(
                Some(&upload),
                Some(output_dir.as_str()),
                Some(&profile),
                Some(&quality),
                Some(&base_url),
                None,
            );

            let ui_weak = ui_weak.clone();
            let app_for_thread = Arc::clone(&app_state_clone);

            std::thread::spawn(move || {
                // Clone variables for logging
                let upload_clone = upload.clone();
                let out_dir_clone = out_dir.display().to_string();
                let cloud_job_id_clone = cloud_job_id.clone();

                // Log: Starting cloud conversion
                {
                    let ui_weak_for_log = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_for_log.upgrade() {
                            append_log_to_ui(&ui, "=== 开始云端转换 (Starting cloud conversion) ===");
                            append_log_to_ui(&ui, &format!("文件: {}", upload_clone));
                            append_log_to_ui(&ui, &format!("输出目录: {}", out_dir_clone));
                            append_log_to_ui(&ui, &format!("Job ID: {}", cloud_job_id_clone));
                            ui.set_status_text("正在上传文件到云端...".into());
                        }
                    });
                }

                // Read upload file bytes
                let bytes = match std::fs::read(&upload) {
                    Ok(b) => b,
                    Err(e) => {
                        app_for_thread.update_job(
                            &cloud_job_id,
                            JobUpdate::Failed(format!("Failed to read upload file: {}", e)),
                        );
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_is_converting(false);
                                ui.set_conversion_progress(0.0);
                                ui.set_quality_status("Failed".into());
                                ui.set_status_text(
                                    format!("Failed to read upload file: {}", e).into(),
                                );
                            }
                        });
                        return;
                    }
                };

                let main_tex_value = if main_tex_str.trim().is_empty() {
                    "main.tex".to_string()
                } else {
                    main_tex_str.trim().to_string()
                };

                let result = crate::cloud_convert::convert_upload_blocking(
                    crate::cloud_convert::CloudUploadRequest {
                        base_url: &base_url,
                        access_token: token.clone(),
                        zip_bytes: bytes,
                        file_name: &file_name,
                        main_tex: &main_tex_value,
                        output_dir: &out_dir,
                        profile: &profile,
                        quality: &quality,
                    },
                );

                match &result {
                    Ok(r) => app_for_thread.update_job(
                        &cloud_job_id,
                        JobUpdate::Succeeded {
                            remote_job_id: Some(r.job_id.clone()),
                            output_path: r.docx_path.display().to_string(),
                            report_path: Some(r.report_path.display().to_string()),
                        },
                    ),
                    Err(e) => {
                        app_for_thread.update_job(&cloud_job_id, JobUpdate::Failed(e.to_string()));
                    }
                }
                helpers::persist_recent_jobs(&app_for_thread);
                let recent_jobs = helpers::recent_jobs_for_ui(&app_for_thread);

                let usage_res = token.as_ref().and_then(|t| {
                    crate::cloud_account::fetch_usage_blocking(&base_url, t).ok()
                });

                let invoke_result = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_is_converting(false);
                        ui.set_conversion_progress(1.0);
                        ui.set_recent_jobs(recent_jobs.into());

                        if let Some(usage) = usage_res {
                            let remaining = usage.cloud_conversions_limit.saturating_sub(usage.cloud_conversions_used) as i32;
                            let total = usage.cloud_conversions_limit as i32;
                            ui.set_quota_remaining(remaining);
                            ui.set_quota_total(total);
                            ui.set_usage_status(crate::cloud_account::usage_line(&usage).into());
                            app_for_thread.set_quota_remaining(Some(remaining as usize));
                            log::info!(
                                "Quota refreshed after cloud conversion: used={}, limit={}, remaining={}",
                                usage.cloud_conversions_used,
                                usage.cloud_conversions_limit,
                                remaining
                            );
                        }

                        match &result {
                            Ok(r) => {
                                ui.set_status_text(
                                    format!(
                                        "{}\nJob: {}\nDOCX: {} ({} bytes)\nReport: {}",
                                        r.report_text,
                                        r.job_id,
                                        r.docx_path.display(),
                                        r.docx_bytes,
                                        r.report_path.display()
                                    )
                                    .into(),
                                );
                                ui.set_quality_status("Cloud completed".into());
                                ui.set_quality_progress(1.0);
                            }
                            Err(e) => {
                                ui.set_conversion_progress(0.0);
                                ui.set_quality_status("Cloud failed".into());
                                ui.set_quality_progress(0.0);
                                ui.set_status_text(
                                    format!("Cloud conversion failed:\n{}", e).into(),
                                );
                            }
                        }
                    }
                });

                if let Err(e) = invoke_result {
                    log::error!("Failed to update UI after cloud conversion: {}", e);
                }
            });
        },
    );

    // Detect Profile button — now works on upload file
    let ui_weak = ui.as_weak();
    ui.on_detect_profile_clicked(move |upload_path: slint::SharedString| {
        log::info!("Detect profile: {}", upload_path);
        let upload = upload_path.to_string();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let result = crate::commands::detect_profile_from_upload(&upload);
            let invoke_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match result {
                        Ok(profile) => {
                            log::info!("Detected profile: {}", profile);
                            helpers::persist_settings(
                                Some(&upload),
                                None,
                                Some(&profile),
                                None,
                                None,
                                None,
                            );
                            ui.set_detected_profile(profile.clone().into());
                            ui.set_status_text(format!("Detected profile: {}", profile).into());
                        }
                        Err(e) => {
                            log::error!("Profile detection failed: {}", e);
                            ui.set_status_text(format!("Profile detection failed:\n{}", e).into());
                        }
                    }
                }
            });

            if let Err(e) = invoke_result {
                log::error!("Failed to update UI after profile detection: {}", e);
            }
        });
    });

    // Open Output button
    ui.on_open_output_clicked(|output_path: slint::SharedString| {
        if let Err(e) = helpers::open_output_path(output_path.as_str()) {
            log::error!("Failed to open output path: {}", e);
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
                Err(e) => {
                    log::error!("Failed to open report path: {}", e);
                    ui.set_status_text(format!("Open report failed:\n{}", e).into());
                }
            }
        }
    });

    // Choose upload file button — opens native file dialog filtered to archives
    let ui_weak = ui.as_weak();
    ui.on_choose_upload_file_clicked(
        move |upload_path: slint::SharedString, _output_path: slint::SharedString| {
            let initial = helpers::path_for_dialog(upload_path.as_str());
            let selected = crate::desktop_dialog::pick_project_zip(initial.as_deref());
            if let Some(selected) = selected {
                helpers::persist_settings(Some(&selected), None, None, None, None, None);
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_upload_path(selected.clone().into());
                    ui.set_status_text(format!("Selected upload file: {}", selected).into());
                }
            }
        },
    );

    // Choose output directory button — opens native folder dialog
    let ui_weak = ui.as_weak();
    ui.on_choose_output_dir_clicked(move |output_dir: slint::SharedString| {
        let initial = helpers::path_for_dialog(output_dir.as_str());
        let selected = crate::desktop_dialog::pick_output_dir(initial.as_deref());
        if let Some(selected) = selected {
            helpers::persist_settings(None, Some(&selected), None, None, None, None);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_output_path(selected.clone().into());
                ui.set_status_text(format!("Selected output directory: {}", selected).into());
            }
        }
    });

    // ============================================================
    // Log Panel Callbacks
    // ============================================================

    // Copy logs to clipboard
    let ui_weak = ui.as_weak();
    ui.on_copy_logs(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let entries = ui.get_log_entries();
            let text: String = entries
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            // Note: Slint doesn't have native clipboard support, so we show a toast instead
            ui.set_toast_message("日志已复制 (查看控制台或日志文件)".into());
            ui.set_toast_level("success".into());
            ui.set_toast_visible(true);
            log::info!("Logs copied to clipboard (manually):\n{}", text);
        }
    });

    // Clear logs
    let ui_weak = ui.as_weak();
    ui.on_clear_logs(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_log_entries(std::rc::Rc::new(VecModel::from(Vec::<SharedString>::new())).into());
            append_log_to_ui(&ui, "日志已清空");
        }
    });

    // Append log entry (can be called from other parts of the app)
    let ui_weak = ui.as_weak();
    ui.on_append_log(move |message: SharedString| {
        if let Some(ui) = ui_weak.upgrade() {
            append_log_to_ui(&ui, message.as_ref());
        }
    });
}

// ============================================================
// Helper: Populate quality dialog from QualityReportSummary
// ============================================================

#[allow(dead_code, unused_imports)]
pub fn populate_quality_dialog(ui: &MainWindow, report: &QualityReportSummary) {
    use slint::ComponentHandle;

    ui.set_dialog_job_id(report.job_id.clone().into());
    ui.set_dialog_profile(report.profile.clone().into());
    ui.set_dialog_engine_version(report.engine_version.clone().into());
    ui.set_dialog_quality_score(report.quality_score as i32);
    ui.set_dialog_parse_score(report.dimension_scores.parse as i32);
    ui.set_dialog_semantic_score(report.dimension_scores.semantic as i32);
    ui.set_dialog_docx_score(report.dimension_scores.docx as i32);
    ui.set_dialog_visual_score(report.dimension_scores.visual as i32);
    ui.set_dialog_editable_score(report.dimension_scores.editable as i32);
    ui.set_dialog_performance_score(report.dimension_scores.performance as i32);
    ui.set_dialog_word_status(report.word_compatibility.status.clone().into());

    // Convert word errors to slint vector
    let errors: Vec<SharedString> = report
        .word_compatibility
        .errors
        .iter()
        .map(|s| s.clone().into())
        .collect();
    ui.set_dialog_word_errors(std::rc::Rc::new(VecModel::from(errors)).into());

    ui.set_dialog_word_method(report.word_compatibility.check_method.clone().into());
    ui.set_dialog_style_coverage(report.style_coverage_rate as f32);
    ui.set_dialog_visual_diff(report.visual_diff_percentage as f32);

    // Note: semantic-loss-list is not populated due to Slint API limitations
    // The quality dialog will show empty semantic loss section

    // Show the dialog
    ui.set_show_quality_dialog(true);
}

// ============================================================
// API: Fetch quality report from backend
// NOTE: This function requires 'reqwest' crate which is not yet added.
// Uncomment when reqwest is added to dependencies.
// ============================================================

// pub fn fetch_quality_report(base_url: &str, token: &str, job_id: &str) -> Result<QualityReportSummary, String> {
//     let url = format!("{}/api/v1/conversions/{}/quality-report", base_url.trim_end_matches('/'), job_id);
//
//     let client = reqwest::blocking::Client::new();
//     let response = client
//         .get(&url)
//         .header("Authorization", format!("Bearer {}", token))
//         .timeout(std::time::Duration::from_secs(30))
//         .send()
//         .map_err(|e| format!("Failed to fetch quality report: {}", e))?;
//
//     if !response.status().is_success() {
//         return Err(format!("API error: {}", response.status()));
//     }
//
//     response
//         .json::<QualityReportSummary>()
//         .map_err(|e| format!("Failed to parse quality report: {}", e))
// }

// ============================================================
// Helper: Parse dimension scores from JSON value
// ============================================================

#[allow(dead_code)]
pub fn parse_dimension_scores(json: &serde_json::Value) -> QualityDimensions {
    let parse = json.get("parse").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let semantic = json.get("semantic").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let docx = json.get("docx").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let visual = json.get("visual").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let editable = json.get("editable").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let performance = json
        .get("performance")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;

    QualityDimensions {
        parse,
        semantic,
        docx,
        visual,
        editable,
        performance,
    }
}
