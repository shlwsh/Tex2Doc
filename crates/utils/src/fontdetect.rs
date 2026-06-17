//! 字体探测（系统 vs 嵌入）
//!
//! 给定一个字体名（LaTeX / Office），探测：
//! 1. 系统是否已安装（Windows/Mac/Linux 各自的查找路径）。
//! 2. 若未安装，是否可以通过 Office 嵌入（fallback）。
//! 3. 给出最终的 Office 字体名 + 嵌入建议。

use std::path::PathBuf;

/// 字体可用性状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStatus {
    /// 系统已安装，可直接使用
    Available,
    /// 系统未安装，需要在 docx 中显式声明（嵌入到文档）
    Embed,
    /// 完全未找到，需要 fallback
    Fallback,
}

/// 探测结果。
#[derive(Debug, Clone)]
pub struct FontProbe {
    /// 原始请求的字体名
    pub name: String,
    /// 状态
    pub status: FontStatus,
    /// 推荐的 Office 字体名（fallback）
    pub recommended: String,
    /// 系统字体路径（若找到）
    pub system_path: Option<PathBuf>,
}

impl FontProbe {
    /// 是否需要嵌入
    pub fn needs_embed(&self) -> bool {
        matches!(self.status, FontStatus::Embed)
    }
    /// 是否需要 fallback
    pub fn needs_fallback(&self) -> bool {
        matches!(self.status, FontStatus::Fallback)
    }
}

/// 字体探测器。
pub struct FontDetector {
    /// 系统字体目录列表（自动探测）
    system_dirs: Vec<PathBuf>,
    /// fallback 字体（默认 Calibri）
    fallback: String,
    /// Office 字体映射（CTeX/中文 → Office 字体）
    office_map: std::collections::HashMap<String, String>,
}

impl FontDetector {
    /// 创建一个新的 FontDetector，自动检测系统字体目录。
    pub fn new() -> Self {
        let system_dirs = detect_system_font_dirs();
        let office_map = default_office_font_map();
        Self {
            system_dirs,
            fallback: "Calibri".to_string(),
            office_map,
        }
    }

    /// 自定义 fallback 字体。
    pub fn with_fallback(mut self, name: impl Into<String>) -> Self {
        self.fallback = name.into();
        self
    }

    /// 注册 Office 字体映射。
    pub fn register_office_mapping(&mut self, latex: &str, office: &str) {
        self.office_map
            .insert(latex.to_string(), office.to_string());
    }

    /// 探测一个字体。
    pub fn probe(&self, name: &str) -> FontProbe {
        // 1. 直接查系统
        if let Some(path) = self.find_system_font(name) {
            return FontProbe {
                name: name.to_string(),
                status: FontStatus::Available,
                recommended: name.to_string(),
                system_path: Some(path),
            };
        }

        // 2. 检查 Office 映射
        if let Some(office) = self.office_map.get(name) {
            // 找到 Office 替代名，再查系统
            if let Some(path) = self.find_system_font(office) {
                return FontProbe {
                    name: name.to_string(),
                    status: FontStatus::Available,
                    recommended: office.clone(),
                    system_path: Some(path),
                };
            }
            // Office 名也找不到：用 Embed 模式（让 docx 内嵌字体描述）
            return FontProbe {
                name: name.to_string(),
                status: FontStatus::Embed,
                recommended: office.clone(),
                system_path: None,
            };
        }

        // 3. 完全找不到 → Fallback
        FontProbe {
            name: name.to_string(),
            status: FontStatus::Fallback,
            recommended: self.fallback.clone(),
            system_path: None,
        }
    }

    /// 在系统字体目录中查找字体文件。
    fn find_system_font(&self, name: &str) -> Option<PathBuf> {
        // 尝试多种常见扩展名
        let extensions = ["ttf", "otf", "TTF", "OTF"];
        let name_normalized = name.replace(' ', "");

        for dir in &self.system_dirs {
            if !dir.exists() {
                continue;
            }
            // 1. 精确匹配
            for ext in &extensions {
                let p = dir.join(format!("{}.{}", name, ext));
                if p.exists() {
                    return Some(p);
                }
                // 不带空格匹配
                let p2 = dir.join(format!("{}.{}", name_normalized, ext));
                if p2.exists() {
                    return Some(p2);
                }
            }
        }
        None
    }

    /// 探测所有相关字体并返回综合结果。
    pub fn probe_all<I, S>(&self, names: I) -> Vec<FontProbe>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        names.into_iter().map(|n| self.probe(n.as_ref())).collect()
    }
}

impl Default for FontDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// 探测系统字体目录。
fn detect_system_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        }
        dirs.push(PathBuf::from("C:/Windows/Fonts"));
        dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
    }
    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home.clone()).join(".fonts"));
            dirs.push(PathBuf::from(home).join(".local/share/fonts"));
        }
    }
    dirs
}

/// 默认 Office 字体映射（CTeX/中文 → Office）。
fn default_office_font_map() -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    // 中文常用
    m.insert("songti".into(), "SimSun".into());
    m.insert("SimSun".into(), "SimSun".into());
    m.insert("宋体".into(), "SimSun".into());
    m.insert("heiti".into(), "SimHei".into());
    m.insert("SimHei".into(), "SimHei".into());
    m.insert("黑体".into(), "SimHei".into());
    m.insert("kaishu".into(), "KaiTi".into());
    m.insert("KaiTi".into(), "KaiTi".into());
    m.insert("楷体".into(), "KaiTi".into());
    m.insert("fangsong".into(), "FangSong".into());
    m.insert("FangSong".into(), "FangSong".into());
    m.insert("仿宋".into(), "FangSong".into());
    m.insert("lishu".into(), "SimLi".into());
    m.insert("SimLi".into(), "SimLi".into());
    m.insert("隶书".into(), "SimLi".into());
    // 西文
    m.insert("rm".into(), "Times New Roman".into());
    m.insert("tt".into(), "Consolas".into());
    m
}

/// 便捷函数：探测一个字体。
pub fn probe_font(name: &str) -> FontProbe {
    FontDetector::new().probe(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_creates() {
        let det = FontDetector::new();
        assert!(!det.system_dirs.is_empty() || cfg!(target_os = "linux"));
    }

    #[test]
    fn probe_unknown_returns_fallback() {
        let det = FontDetector::new();
        let probe = det.probe("NonExistentFontXYZ");
        assert_eq!(probe.status, FontStatus::Fallback);
        assert_eq!(probe.recommended, "Calibri");
    }

    #[test]
    fn probe_with_office_mapping() {
        let det = FontDetector::new();
        let probe = det.probe("songti");
        // Should map to SimSun
        assert_eq!(probe.recommended, "SimSun");
    }

    #[test]
    fn probe_finds_calibri_if_present() {
        // Calibri exists on most Windows systems
        #[cfg(target_os = "windows")]
        {
            let det = FontDetector::new();
            let probe = det.probe("Calibri");
            // On Windows, Calibri is in C:/Windows/Fonts
            // It may or may not be found depending on system, but status should not be Fallback
            // when the system has any path
            if det.system_dirs.iter().any(|d| d.exists()) {
                // We have font dirs - check either Available or Embed (mapping)
                assert_ne!(probe.status, FontStatus::Fallback);
            }
        }
    }

    #[test]
    fn register_custom_mapping() {
        let mut det = FontDetector::new();
        det.register_office_mapping("MyFont", "SimSun");
        let probe = det.probe("MyFont");
        assert_eq!(probe.recommended, "SimSun");
    }
}
