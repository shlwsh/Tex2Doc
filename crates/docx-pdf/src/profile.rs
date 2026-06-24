//! LibreOffice 临时 user-profile 目录。
//!
//! 设计见 `docs/study/08-pdf-pipeline/03-docx-to-pdf.md` §3.5.2。
//!
//! soffice 是单实例锁敏感的——必须每次独立 `--user-profile`，否则并发 2 个会卡死。

use std::path::PathBuf;

use anyhow::{Context, Result};

/// 建一个临时 user-profile 目录，路径形如 `<temp>/doc-docx-pdf/lo-profile-<pid>-<nanos>`。
#[allow(dead_code)]
pub fn temp_user_profile() -> Result<PathBuf> {
    let base = std::env::temp_dir().join("doc-docx-pdf");
    std::fs::create_dir_all(&base)
        .with_context(|| format!("创建 profile 根目录失败：{}", base.display()))?;
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = base.join(format!("lo-profile-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("创建 profile 目录失败：{}", dir.display()))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_distinct_profile_dirs() {
        let a = temp_user_profile().unwrap();
        let b = temp_user_profile().unwrap();
        assert_ne!(a, b, "两次调用应产生不同目录");
        assert!(a.is_dir() && b.is_dir());
    }
}
