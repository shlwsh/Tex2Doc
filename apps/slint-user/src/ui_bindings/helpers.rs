use crate::app_state::AppState;
use crate::cloud_account::{self, CloudAccountSession};
use crate::ui::{JobRow, MainWindow};
use std::path::PathBuf;

pub(crate) fn apply_account_session(
    app: &AppState,
    ui: &MainWindow,
    api_base_url: &str,
    session: CloudAccountSession,
) {
    let quota_remaining = session
        .usage
        .cloud_conversions_limit
        .saturating_sub(session.usage.cloud_conversions_used) as usize;
    let quota_total = session.usage.cloud_conversions_limit as usize;
    let display_name = session
        .display_name
        .clone()
        .unwrap_or_else(|| session.email.clone());
    let store_status = match crate::credential_store::store_refresh_token(
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
    // Phase D.1: push structured account state
    ui.set_is_signed_in(true);
    ui.set_account_display_name(display_name.into());
    ui.set_account_tier(session.plan_id.into());
    ui.set_quota_remaining(quota_remaining as i32);
    ui.set_quota_total(quota_total as i32);
}

pub(crate) fn job_history_for_ui(app_state: &AppState) -> Vec<JobRow> {
    app_state
        .all_jobs()
        .into_iter()
        .map(|job| JobRow {
            id: job.id.into(),
            kind: "local".into(),
            input: job.project_path.into(),
            output: job.output_path.unwrap_or_default().into(),
            status: job.status.to_string().into(),
            opened_at: job.created_at.into(),
            error: job.error.unwrap_or_default().into(),
            html_report: job.report_path.unwrap_or_default().into(),
        })
        .collect()
}

pub(crate) fn recent_jobs_for_ui(app_state: &AppState) -> String {
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

pub(crate) fn persist_recent_jobs(app_state: &AppState) {
    if let Err(error) = crate::job_history::save_recent_jobs(&app_state.recent_jobs()) {
        log::warn!("Failed to persist recent jobs: {}", error);
    }
}

pub(crate) fn persist_settings(
    project_path: Option<&str>,
    output_path: Option<&str>,
    profile: Option<&str>,
    quality: Option<&str>,
    api_base_url: Option<&str>,
    login_email: Option<&str>,
) {
    let mut settings = crate::settings::Settings::load();
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

pub(crate) fn persist_redeem_code(
    code: &str,
    api_base_url: &str,
    login_email: &str,
) {
    let mut settings = crate::settings::Settings::load();
    if !code.trim().is_empty() {
        settings.last_redeem_code = Some(code.to_string());
    }
    if !api_base_url.trim().is_empty() {
        settings.api_base_url = api_base_url.to_string();
    }
    if !login_email.trim().is_empty() {
        settings.last_login_email = Some(login_email.to_string());
    }
    if let Err(error) = settings.save() {
        log::warn!("Failed to persist redeem code: {}", error);
    }
}

pub(crate) fn persist_release_channel(channel: &str) {
    let channel = channel.trim();
    if channel.is_empty() {
        return;
    }
    let mut settings = crate::settings::Settings::load();
    settings.release_channel = channel.to_string();
    if let Err(error) = settings.save() {
        log::warn!("Failed to persist release channel: {}", error);
    }
}

pub(crate) fn path_for_dialog(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

pub(crate) fn report_path_for_output(output_path: &str) -> Option<PathBuf> {
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

pub(crate) fn open_output_path(path: &str) -> std::io::Result<()> {
    let path = std::path::Path::new(path);
    let target = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    open_path(target)
}

pub(crate) fn open_report_path(output_path: &str) -> std::io::Result<PathBuf> {
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

pub(crate) fn open_path(target: &std::path::Path) -> std::io::Result<()> {
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

pub(crate) fn open_external_url(url: &str) -> std::io::Result<()> {
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
