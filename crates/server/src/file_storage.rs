//! File storage for session artifacts under `sessions/YYYY/MM/DD/{job_id}/`.
//!
//! Files stored:
//! - `source.zip` — user-uploaded project ZIP
//! - `result.docx` — conversion output
//! - `conversion.log` — structured conversion log

use std::fs;
use std::io;
use std::path::PathBuf;

use chrono::{Datelike, Local};

/// Session file storage with date-based directory hierarchy.
#[derive(Clone)]
pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    /// Create a new FileStorage with the given root directory.
    /// The root will be created if it does not exist.
    pub fn new(root: PathBuf) -> io::Result<Self> {
        if !root.exists() {
            fs::create_dir_all(&root)?;
        }
        Ok(Self { root })
    }

    /// Build the session directory path for a given job_id, using today's date.
    /// Format: `{root}/sessions/{YYYY}/{MM}/{DD}/{job_id}/`
    pub fn session_dir(&self, job_id: &str) -> PathBuf {
        let now = Local::now();
        self.root
            .join("sessions")
            .join(format!("{:04}", now.year()))
            .join(format!("{:02}", now.month()))
            .join(format!("{:02}", now.day()))
            .join(sanitize_id(job_id))
    }

    /// Store bytes to a file inside the session directory.
    /// Creates the directory hierarchy if needed.
    pub fn store(&self, job_id: &str, filename: &str, bytes: &[u8]) -> io::Result<PathBuf> {
        let dir = self.session_dir(job_id);
        fs::create_dir_all(&dir)?;
        let path = dir.join(fixed_filename(filename));
        fs::write(&path, bytes)?;
        Ok(path)
    }

    /// Load bytes from a file inside the session directory.
    pub fn load(&self, job_id: &str, filename: &str) -> io::Result<Vec<u8>> {
        let dir = self.session_dir(job_id);
        fs::read(dir.join(fixed_filename(filename)))
    }

    /// Load bytes by a previously persisted relative object key.
    pub fn load_key(&self, key: &str) -> io::Result<Vec<u8>> {
        fs::read(self.root.join(key))
    }

    /// Returns true if the given file exists in the session directory.
    #[allow(dead_code)]
    pub fn exists(&self, job_id: &str, filename: &str) -> bool {
        self.session_dir(job_id)
            .join(fixed_filename(filename))
            .is_file()
    }

    /// Build a relative key for the stored file, e.g.
    /// `sessions/2026/06/24/conv_0001/source.zip`
    pub fn file_key(&self, job_id: &str, filename: &str) -> String {
        let dir = self.session_dir(job_id);
        dir.strip_prefix(&self.root)
            .unwrap_or(&dir)
            .join(fixed_filename(filename))
            .display()
            .to_string()
            .replace('\\', "/")
    }

    /// Returns the absolute path for a file (for direct serving).
    #[allow(dead_code)]
    pub fn absolute_path(&self, job_id: &str, filename: &str) -> PathBuf {
        self.session_dir(job_id).join(fixed_filename(filename))
    }

    /// Build a conversion log from structured data.
    pub fn build_conversion_log(
        job_id: &str,
        user_id: &str,
        upload_id: &str,
        main_tex: &str,
        profile: &str,
        quality: &str,
        engine: &str,
        status: &str,
        docx_bytes: Option<usize>,
        error: Option<&str>,
    ) -> String {
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let date_part = now.format("%Y-%m-%d").to_string();

        let mut log = format!(
            "=== Conversion Job {job_id} ===\n\
             Started:   {timestamp}\n\
             Date:      {date_part}\n\
             User:      {user_id}\n\
             Upload:    {upload_id}\n\
             Main TeX:  {main_tex}\n\
             Profile:   {profile}\n\
             Quality:   {quality}\n\
             Engine:    {engine}\n\
             ---\n"
        );

        let stage = match status {
            "queued" => "Queued",
            "normalizing" => "Normalizing",
            "detecting" => "Detecting",
            "analyzing" => "Analyzing",
            "compiling" => "Compiling",
            "rendering" => "Rendering",
            "verifying" => "Verifying",
            "completed" => "Completed",
            "failed" => "Failed",
            "expired" => "Expired",
            other => other,
        };

        log.push_str(&format!("[{timestamp}] Status: {stage}\n"));

        if status == "completed" {
            if let Some(bytes) = docx_bytes {
                log.push_str(&format!(
                    "[{timestamp}] Output: result.docx ({bytes} bytes)\n"
                ));
            }
        }

        if let Some(err) = error {
            log.push_str(&format!("[{timestamp}] Error: {err}\n"));
        }

        log.push_str("===\n");
        log
    }
}

/// Only allow safe filenames — reject anything with path separators or suspicious chars.
fn fixed_filename(name: &str) -> String {
    match name {
        "source.zip" | "result.docx" | "conversion.log" => name.to_string(),
        _ => {
            if name.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_') {
                name.to_string()
            } else {
                "file".to_string()
            }
        }
    }
}

/// Strip `..` and other path traversal components from an ID.
fn sanitize_id(id: &str) -> String {
    id.replace("..", "")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_id() {
        assert_eq!(sanitize_id("conv_00000001"), "conv_00000001");
        assert_eq!(sanitize_id("../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_id("../../../root"), "root");
    }

    #[test]
    fn test_fixed_filename() {
        assert_eq!(fixed_filename("source.zip"), "source.zip");
        assert_eq!(fixed_filename("result.docx"), "result.docx");
        assert_eq!(fixed_filename("conversion.log"), "conversion.log");
        assert_eq!(fixed_filename("evil/../../../etc"), "file");
    }

    #[test]
    fn test_build_conversion_log() {
        let log = FileStorage::build_conversion_log(
            "conv_001",
            "user_abc",
            "upload_001",
            "main.tex",
            "auto",
            "standard",
            "semantic-engine",
            "completed",
            Some(12345),
            None,
        );
        assert!(log.contains("conv_001"));
        assert!(log.contains("Completed"));
    }
}
