//! Lightweight native dialog adapter for the desktop preview client.
//!
//! This avoids adding a GUI dialog dependency while P5 is still a preview.
//! If no platform dialog command is available, callers keep the manual path UI.

use std::path::Path;
use std::process::Command;

pub fn pick_project_folder(initial: Option<&Path>) -> Option<String> {
    pick_folder(initial, "Select TeX project folder")
}

pub fn pick_project_zip(initial: Option<&Path>) -> Option<String> {
    pick_file(initial, "Select TeX project zip", DialogFilter::Zip)
}

pub fn pick_output_docx(initial: Option<&Path>) -> Option<String> {
    pick_save_file(initial, "Choose output DOCX", DialogFilter::Docx)
}

enum DialogFilter {
    Zip,
    Docx,
}

#[cfg(target_os = "windows")]
fn pick_folder(_initial: Option<&Path>, title: &str) -> Option<String> {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $d = New-Object System.Windows.Forms.FolderBrowserDialog; \
         $d.Description = '{}'; \
         if ($d.ShowDialog() -eq 'OK') {{ $d.SelectedPath }}",
        powershell_escape(title)
    );
    run_output(Command::new("powershell").args(["-NoProfile", "-Command", &script]))
}

#[cfg(target_os = "windows")]
fn pick_file(initial: Option<&Path>, title: &str, filter: DialogFilter) -> Option<String> {
    let file_name = initial
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $d = New-Object System.Windows.Forms.OpenFileDialog; \
         $d.Title = '{}'; \
         $d.Filter = '{}'; \
         $d.FileName = '{}'; \
         if ($d.ShowDialog() -eq 'OK') {{ $d.FileName }}",
        powershell_escape(title),
        powershell_escape(filter_string(filter)),
        powershell_escape(file_name)
    );
    run_output(Command::new("powershell").args(["-NoProfile", "-Command", &script]))
}

#[cfg(target_os = "windows")]
fn pick_save_file(initial: Option<&Path>, title: &str, filter: DialogFilter) -> Option<String> {
    let file_name = initial
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("output.docx");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $d = New-Object System.Windows.Forms.SaveFileDialog; \
         $d.Title = '{}'; \
         $d.Filter = '{}'; \
         $d.FileName = '{}'; \
         if ($d.ShowDialog() -eq 'OK') {{ $d.FileName }}",
        powershell_escape(title),
        powershell_escape(filter_string(filter)),
        powershell_escape(file_name)
    );
    run_output(Command::new("powershell").args(["-NoProfile", "-Command", &script]))
}

#[cfg(target_os = "macos")]
fn pick_folder(_initial: Option<&Path>, title: &str) -> Option<String> {
    run_output(Command::new("osascript").args([
        "-e",
        &format!(
            "POSIX path of (choose folder with prompt \"{}\")",
            applescript_escape(title)
        ),
    ]))
}

#[cfg(target_os = "macos")]
fn pick_file(_initial: Option<&Path>, title: &str, filter: DialogFilter) -> Option<String> {
    let types = match filter {
        DialogFilter::Zip => "{\"zip\"}",
        DialogFilter::Docx => "{\"docx\"}",
    };
    run_output(Command::new("osascript").args([
        "-e",
        &format!(
            "POSIX path of (choose file with prompt \"{}\" of type {})",
            applescript_escape(title),
            types
        ),
    ]))
}

#[cfg(target_os = "macos")]
fn pick_save_file(initial: Option<&Path>, title: &str, _filter: DialogFilter) -> Option<String> {
    let default_name = initial
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("output.docx");
    run_output(Command::new("osascript").args([
        "-e",
        &format!(
            "POSIX path of (choose file name with prompt \"{}\" default name \"{}\")",
            applescript_escape(title),
            applescript_escape(default_name)
        ),
    ]))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn pick_folder(initial: Option<&Path>, title: &str) -> Option<String> {
    let initial = path_arg(initial);
    run_output(Command::new("zenity").args([
        "--file-selection",
        "--directory",
        "--title",
        title,
        "--filename",
        &initial,
    ]))
    .or_else(|| {
        run_output(Command::new("kdialog").args([
            "--title",
            title,
            "--getexistingdirectory",
            &initial,
        ]))
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn pick_file(initial: Option<&Path>, title: &str, filter: DialogFilter) -> Option<String> {
    let initial = path_arg(initial);
    match filter {
        DialogFilter::Zip => run_output(Command::new("zenity").args([
            "--file-selection",
            "--title",
            title,
            "--filename",
            &initial,
            "--file-filter",
            "Zip archives | *.zip",
        ]))
        .or_else(|| {
            run_output(Command::new("kdialog").args([
                "--title",
                title,
                "--getopenfilename",
                &initial,
                "Zip archives (*.zip)",
            ]))
        }),
        DialogFilter::Docx => run_output(Command::new("zenity").args([
            "--file-selection",
            "--title",
            title,
            "--filename",
            &initial,
            "--file-filter",
            "Word documents | *.docx",
        ]))
        .or_else(|| {
            run_output(Command::new("kdialog").args([
                "--title",
                title,
                "--getopenfilename",
                &initial,
                "Word documents (*.docx)",
            ]))
        }),
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn pick_save_file(initial: Option<&Path>, title: &str, filter: DialogFilter) -> Option<String> {
    let initial = path_arg(initial);
    let zenity_filter = match filter {
        DialogFilter::Zip => "Zip archives | *.zip",
        DialogFilter::Docx => "Word documents | *.docx",
    };
    let kdialog_filter = match filter {
        DialogFilter::Zip => "Zip archives (*.zip)",
        DialogFilter::Docx => "Word documents (*.docx)",
    };
    run_output(Command::new("zenity").args([
        "--file-selection",
        "--save",
        "--confirm-overwrite",
        "--title",
        title,
        "--filename",
        &initial,
        "--file-filter",
        zenity_filter,
    ]))
    .or_else(|| {
        run_output(Command::new("kdialog").args([
            "--title",
            title,
            "--getsavefilename",
            &initial,
            kdialog_filter,
        ]))
    })
}

fn run_output(command: &mut Command) -> Option<String> {
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn path_arg(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| ".".to_string())
}

#[cfg(target_os = "windows")]
fn filter_string(filter: DialogFilter) -> &'static str {
    match filter {
        DialogFilter::Zip => "Zip archives (*.zip)|*.zip|All files (*.*)|*.*",
        DialogFilter::Docx => "Word documents (*.docx)|*.docx|All files (*.*)|*.*",
    }
}

#[cfg(target_os = "windows")]
fn powershell_escape(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(target_os = "macos")]
fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
