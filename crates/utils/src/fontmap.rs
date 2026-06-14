//! CTeX 字体 → Office 字体映射
//!
//! V1 默认映射（见方案 §4.5）。允许通过 [`FontMapBuilder`] 扩展。

use std::collections::HashMap;

/// Office 端字体标识（与 styles.xml 中 `w:rFonts` 兼容）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OfficeFont {
    /// ascii / hAnsi 默认字体（拉丁字符）
    pub ascii: String,
    /// 东亚字符默认字体
    pub east_asia: String,
}

impl OfficeFont {
    /// 构造单名字体（ascii 与 east_asia 相同）。
    pub fn single(name: impl Into<String>) -> Self {
        let n = name.into();
        Self {
            ascii: n.clone(),
            east_asia: n,
        }
    }

    /// 构造双名字体。
    pub fn pair(ascii: impl Into<String>, east_asia: impl Into<String>) -> Self {
        Self {
            ascii: ascii.into(),
            east_asia: east_asia.into(),
        }
    }
}

/// 字体映射表。
#[derive(Debug, Clone, Default)]
pub struct FontMap {
    map: HashMap<String, OfficeFont>,
}

impl FontMap {
    /// 创建空映射表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一条映射。
    pub fn insert(&mut self, latex: impl Into<String>, office: OfficeFont) {
        self.map.insert(latex.into(), office);
    }

    /// 查找映射。
    pub fn get(&self, latex: &str) -> Option<&OfficeFont> {
        self.map.get(latex)
    }
}

/// V1 默认映射表构建器。
pub fn default_map() -> FontMap {
    use OfficeFont as OF;

    let mut m = FontMap::new();
    m.insert("songti", OF::single("SimSun"));
    m.insert("SimSun", OF::single("SimSun"));
    m.insert("宋体", OF::single("SimSun"));

    m.insert("heiti", OF::single("SimHei"));
    m.insert("SimHei", OF::single("SimHei"));
    m.insert("黑体", OF::single("SimHei"));

    m.insert("fangsong", OF::single("FangSong"));
    m.insert("FangSong", OF::single("FangSong"));
    m.insert("仿宋", OF::single("FangSong"));

    m.insert("kaishu", OF::single("KaiTi"));
    m.insert("KaiTi", OF::single("KaiTi"));
    m.insert("楷体", OF::single("KaiTi"));

    // 常用西文
    m.insert("rm", OF::single("Times New Roman"));
    m.insert("tt", OF::single("Consolas"));

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_map_contains_core() {
        let m = default_map();
        assert!(m.get("songti").is_some());
        assert!(m.get("SimSun").is_some());
        assert!(m.get("heiti").is_some());
    }

    #[test]
    fn custom_override() {
        let mut m = default_map();
        m.insert("songti", OfficeFont::single("Source Han Serif SC"));
        assert_eq!(m.get("songti").unwrap().ascii, "Source Han Serif SC");
    }
}
