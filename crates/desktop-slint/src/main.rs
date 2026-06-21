//! P5: Tex2Doc Desktop — Slint-based GUI client.
//!
//! Provides a graphical interface for local TeX → DOCX conversion
//! with journal profile auto-detection, quality reporting, and job history.

mod app_state;
mod commands;
mod job;
mod local_convert;
mod report;
mod settings;
mod ui;

use app_state::AppState;
use report::ReportSummary;
use settings::Settings;
use slint::include_modules;
use std::sync::Arc;

// Re-export generated Slint components
include_modules!();

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Tex2Doc Desktop starting...");

    // Load settings
    let settings = Settings::load();
    log::info!("Settings loaded: quality={}, output_dir={}", settings.quality, settings.output_dir.display());

    // Initialize app state
    let app_state: Arc<AppState> = Arc::new(AppState::default());

    // Create and configure the UI
    let ui = MainWindow::new().unwrap();

    // Initialize property defaults from settings
    let default_output = settings.output_dir.display().to_string();
    let default_quality = settings.quality.clone();
    ui.set_output_path(std::borrow::Cow::Owned(default_output));
    ui.set_quality_level(std::borrow::Cow::Owned(default_quality));
    ui.set_detected_profile(std::borrow::Cow::Borrowed("auto"));
    ui.set_compatibility_score(std::borrow::Cow::Borrowed("--"));
    ui.set_quality_status(std::borrow::Cow::Borrowed("--"));
    ui.set_profile_confidence(std::borrow::Cow::Borrowed("--"));
    ui.set_recent_jobs(std::borrow::Cow::Borrowed("No recent jobs."));

    // Wire up the Convert button callback
    let app_state_clone = Arc::clone(&app_state);
    ui.on_convert_clicked(move |project_path: slint::SharedString,
                                 detected_profile: slint::SharedString,
                                 quality_level: slint::SharedString,
                                 output_path: slint::SharedString| {
        log::info!("Convert clicked: project={}, profile={}, quality={}", project_path, detected_profile, quality_level);

        let proj = std::path::PathBuf::from(project_path.as_str());
        let out = std::path::PathBuf::from(output_path.as_str());
        let profile = detected_profile.to_string();
        let quality = quality_level.to_string();
        let app = Arc::clone(&app_state_clone);

        std::thread::spawn(move || {
            log::info!("Starting conversion in background thread");

            match local_convert::convert(&proj, &proj.join("main.tex"), &out, None, &profile, &quality) {
                Ok(artifact) => {
                    let summary = ReportSummary::from_report(&artifact.report);
                    log::info!("Conversion succeeded: {} bytes, profile={}", artifact.report.docx_bytes, summary.profile);
                    // Note: UI updates from non-UI threads require Slint::invoke_from_event_loop
                    // For now, conversions are reflected in the job history
                }
                Err(e) => {
                    log::error!("Conversion failed: {}", e);
                }
            }
        });
    });

    // Wire up the Detect Profile button
    let app_state_clone2 = Arc::clone(&app_state);
    ui.on_detect_profile_clicked(move |project_path: slint::SharedString| {
        log::info!("Detect profile: {}", project_path);
        let proj = std::path::PathBuf::from(project_path.as_str());

        std::thread::spawn(move || {
            match commands::detect_profile(&proj) {
                Ok(profile) => {
                    log::info!("Detected profile: {}", profile);
                    // Profile detection result would be shown in UI via callback
                }
                Err(e) => {
                    log::error!("Profile detection failed: {}", e);
                }
            }
        });
    });

    log::info!("Tex2Doc Desktop UI initialized, entering event loop");

    ui.run().unwrap();
}
