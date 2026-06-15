//! 内容寻址缓存。
//!
//! 设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.6。
//!
//! 缓存键：`blake3("v2-tex-facade:v1:" + main + 所有 \input/\include 子文件 + figures 媒体)`。
//! 缓存目录结构：
//! ```text
//! <root>/v2-tex-facade/<engine>/<blake3-hex>/
//! ├── input.snapshot.json
//! ├── output.pdf
//! └── build.log
//! ```
//!
//! **关键不变量**（§2.6.2）：
//! - 缓存 key **只用字节不掺路径**——避免 Windows `\Linux` 分隔符差异；
//! - 编译失败不写缓存（避免把一次失败的 PDF 当缓存）；
//! - 缓存根目录可通过 `DOC_TEX_CACHE` 环境变量覆盖（默认 `<workdir>/.cache/tex-facade/`）。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use blake3::Hasher;
use walkdir::WalkDir;

use crate::backend::{EngineKind, TexProject};

/// 缓存键：32 字节 blake3。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey(pub [u8; 32]);

impl CacheKey {
    /// 缓存键的 hex 字符串（64 字符），用作缓存目录名。
    pub fn hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// 缓存根目录解析：`<workdir>/.cache/tex-facade/`，可由 `DOC_TEX_CACHE` 覆盖。
pub fn default_cache_root(workdir: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("DOC_TEX_CACHE") {
        return PathBuf::from(p);
    }
    workdir.join(".cache").join("tex-facade")
}

/// 缓存对象：封装根目录 + 单一读 / 写入口。
#[derive(Debug, Clone)]
pub struct Cache {
    /// 缓存根目录。
    pub root: PathBuf,
}

impl Cache {
    /// 以 `workdir` 默认根目录构造（`DOC_TEX_CACHE` 仍生效）。
    pub fn for_workdir(workdir: &Path) -> Self {
        Self {
            root: default_cache_root(workdir),
        }
    }

    /// 显式指定根目录。
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    /// 缓存命中检查：返回 `Some(pdf_path)` 表示命中，`None` 表示未命中。
    pub fn lookup(&self, engine: EngineKind, key: CacheKey) -> Option<PathBuf> {
        let dir = self.dir(engine, key);
        let pdf = dir.join("output.pdf");
        if pdf.is_file() {
            Some(pdf)
        } else {
            None
        }
    }

    /// 缓存目录路径（不一定存在）。
    pub fn dir(&self, engine: EngineKind, key: CacheKey) -> PathBuf {
        self.root
            .join("v2-tex-facade")
            .join(engine.as_str())
            .join(key.hex())
    }

    /// 写入缓存：`output.pdf` + `build.log` + `input.snapshot.json`。
    ///
    /// 失败模式：根目录不可写 → 抛 `TexError::CacheUnwritable`。
    pub async fn store(
        &self,
        engine: EngineKind,
        key: CacheKey,
        pdf_src: &Path,
        log: &str,
    ) -> Result<PathBuf> {
        let dir = self.dir(engine, key);
        tokio::fs::create_dir_all(&dir)
            .await
            .with_context(|| format!("无法创建缓存目录：{}", dir.display()))?;

        let target_pdf = dir.join("output.pdf");
        tokio::fs::copy(pdf_src, &target_pdf)
            .await
            .with_context(|| {
                format!(
                    "复制 PDF 到缓存失败：src={} dst={}",
                    pdf_src.display(),
                    target_pdf.display()
                )
            })?;

        let log_path = dir.join("build.log");
        tokio::fs::write(&log_path, log.as_bytes())
            .await
            .with_context(|| format!("写入 build.log 失败：{}", log_path.display()))?;

        // 输入快照（用 `cache_key_input` 模块；此处只放占位）
        let snap = dir.join("input.snapshot.json");
        let _ = tokio::fs::write(
            &snap,
            format!(
                "{{\"key_hex\":\"{}\",\"engine\":\"{}\"}}",
                key.hex(),
                engine.as_str()
            )
            .as_bytes(),
        )
        .await;

        Ok(target_pdf)
    }
}

/// 扫描 `.tex` 中 `\input{...}` / `\include{...}` 顶层引用（**单层**，V2 不解析递归）。
///
/// 见 §2.6.1 第 1 条。
pub fn referenced_tex_files(main_file: &Path) -> Result<Vec<PathBuf>> {
    let workdir = main_file.parent().unwrap_or_else(|| Path::new("."));
    let content = std::fs::read_to_string(main_file)
        .with_context(|| format!("读取主 .tex 失败：{}", main_file.display()))?;

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in content.lines() {
        let line = line.trim_start();
        for keyword in ["\\input", "\\include", "\\subfile"] {
            if let Some(rest) = line.strip_prefix(keyword) {
                if let Some(name) = extract_braced(rest) {
                    if seen.insert(name.clone()) {
                        // 解析路径：先当 workdir 子文件，再尝试同名 .tex
                        let direct = workdir.join(&name);
                        let with_tex = workdir.join(format!("{name}.tex"));
                        if direct.is_file() {
                            out.push(direct);
                        } else if with_tex.is_file() {
                            out.push(with_tex);
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

/// 计算缓存键（见 §2.6.1）。
pub fn compute_key(project: &TexProject) -> Result<CacheKey> {
    let mut hasher = Hasher::new();
    hasher.update(b"v2-tex-facade:v1:");

    // 主文件
    let main_bytes = std::fs::read(&project.main_file)
        .with_context(|| format!("读取主 .tex 失败：{}", project.main_file.display()))?;
    hasher.update(blake3::hash(&main_bytes).as_bytes());

    // \input/\include 子文件
    for include in referenced_tex_files(&project.main_file)? {
        if let Ok(bytes) = std::fs::read(&include) {
            hasher.update(blake3::hash(&bytes).as_bytes());
        }
    }

    // figures/ 下 png / jpg / pdf
    let figures = project.workdir.join("figures");
    if figures.is_dir() {
        for entry in WalkDir::new(&figures)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let ext_ok = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| {
                    matches!(
                        s.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "pdf"
                    )
                })
                .unwrap_or(false);
            if ext_ok {
                if let Ok(bytes) = std::fs::read(path) {
                    hasher.update(blake3::hash(&bytes).as_bytes());
                }
            }
        }
    }

    Ok(CacheKey(*hasher.finalize().as_bytes()))
}

/// 从 `\input{abc}` 之后到第一个空白或行注释的串中提取 `{abc}` 里的 `abc`。
fn extract_braced(s: &str) -> Option<String> {
    let s = s.trim_start();
    if !s.starts_with('{') {
        return None;
    }
    let close = s.find('}')?;
    Some(s[1..close].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_braced_basic() {
        assert_eq!(extract_braced("{01_intro}").as_deref(), Some("01_intro"));
        assert_eq!(extract_braced("{a/b.tex}").as_deref(), Some("a/b.tex"));
        assert!(extract_braced("abc").is_none());
    }
}
