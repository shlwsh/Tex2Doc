//! Word 兼容性检查器
//!
//! 使用 LibreOffice headless 将 DOCX 转换为 PDF，检查转换是否成功。
//! 这是一个轻量级的兼容性检查，不依赖 Windows Word。

use std::path::Path;
use std::process::Command;

/// Word 兼容性检查结果。
#[derive(Debug, Clone)]
pub struct WordCompatibilityResult {
    /// passed / warnings / failed
    pub status: CompatibilityStatus,
    /// LibreOffice 转换错误信息
    pub error_message: Option<String>,
    /// 转换后的 PDF 路径（如果成功）
    pub pdf_path: Option<String>,
    /// 检查耗时（毫秒）
    pub elapsed_ms: u64,
}

/// 兼容性状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatus {
    Passed,
    Warnings,
    Failed,
}

/// Word 兼容性检查器。
pub struct WordCompatibilityChecker {
    /// LibreOffice 可执行文件路径（可选，默认使用系统路径）
    libreoffice_path: Option<String>,
}

impl WordCompatibilityChecker {
    pub fn new() -> Self {
        Self {
            libreoffice_path: None,
        }
    }

    /// 设置 LibreOffice 路径。
    pub fn with_libreoffice_path(mut self, path: impl Into<String>) -> Self {
        self.libreoffice_path = Some(path.into());
        self
    }

    /// 检查 DOCX 文件的 Word 兼容性。
    ///
    /// 使用 LibreOffice headless 转换，验证 DOCX 能否被正确解析。
    pub fn check(&self, docx_path: &Path) -> WordCompatibilityResult {
        let start = std::time::Instant::now();

        // 检查文件是否存在
        if !docx_path.exists() {
            return WordCompatibilityResult {
                status: CompatibilityStatus::Failed,
                error_message: Some(format!("文件不存在: {}", docx_path.display())),
                pdf_path: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
            };
        }

        // 查找 LibreOffice
        let libreoffice = self.find_libreoffice();
        let libreoffice = match libreoffice {
            Some(p) => p,
            None => {
                return WordCompatibilityResult {
                    status: CompatibilityStatus::Warnings,
                    error_message: Some(
                        "LibreOffice 未找到，跳过 Word 兼容性检查".to_string(),
                    ),
                    pdf_path: None,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        // 创建临时输出目录
        let output_dir = docx_path.parent().unwrap_or(Path::new("."));
        let output_base = docx_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "output".to_string());

        // 执行 LibreOffice 转换
        let result = Command::new(&libreoffice)
            .args([
                "--headless",
                "--convert-to",
                "pdf",
                "--outdir",
                output_dir.to_str().unwrap_or("."),
                docx_path.to_str().unwrap_or(""),
            ])
            .output();

        let elapsed = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) if output.status.success() => {
                let pdf_path = output_dir.join(format!("{}.pdf", output_base));
                WordCompatibilityResult {
                    status: if pdf_path.exists() {
                        CompatibilityStatus::Passed
                    } else {
                        CompatibilityStatus::Warnings
                    },
                    error_message: None,
                    pdf_path: if pdf_path.exists() {
                        Some(pdf_path.to_string_lossy().to_string())
                    } else {
                        None
                    },
                    elapsed_ms: elapsed,
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                WordCompatibilityResult {
                    status: CompatibilityStatus::Failed,
                    error_message: Some(format!("LibreOffice 转换失败: {}", stderr)),
                    pdf_path: None,
                    elapsed_ms: elapsed,
                }
            }
            Err(e) => WordCompatibilityResult {
                status: CompatibilityStatus::Failed,
                error_message: Some(format!("执行 LibreOffice 失败: {}", e)),
                pdf_path: None,
                elapsed_ms: elapsed,
            },
        }
    }

    fn find_libreoffice(&self) -> Option<String> {
        // 优先使用配置的路径
        if let Some(ref path) = self.libreoffice_path {
            if Path::new(path).exists() {
                return Some(path.clone());
            }
        }

        // 尝试常见路径
        #[cfg(target_os = "windows")]
        let candidates = [
            "soffice.exe",
            "C:\\Program Files\\LibreOffice\\program\\soffice.exe",
            "C:\\Program Files (x86)\\LibreOffice\\program\\soffice.exe",
        ];

        #[cfg(not(target_os = "windows"))]
        let candidates = [
            "soffice",
            "/usr/bin/soffice",
            "/usr/local/bin/soffice",
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        ];

        for candidate in candidates {
            if Path::new(candidate).exists() {
                return Some(candidate.to_string());
            }
        }

        // 尝试在 PATH 中查找
        if let Ok(output) = Command::new("which").arg("soffice").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                return Some(path.trim().to_string());
            }
        }

        None
    }
}

impl Default for WordCompatibilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_check_nonexistent_file() {
        let checker = WordCompatibilityChecker::new();
        let result = checker.check(Path::new("/nonexistent/file.docx"));
        assert_eq!(result.status, CompatibilityStatus::Failed);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn test_check_empty_file() {
        let checker = WordCompatibilityChecker::new();
        let temp = NamedTempFile::with_extension("docx").unwrap();
        let result = checker.check(Path::new(temp.path()));
        // 空文件会失败或警告，但不是 passed
        assert_ne!(result.status, CompatibilityStatus::Passed);
    }
}
