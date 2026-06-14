//! include 路径解算
//!
//! 按以下优先级在 VFS 中查找 `target`：
//! 1. 原样（已规范化）
//! 2. 相对 `base_dir` 的同目录
//! 3. `\graphicspath{}` 声明路径列表
//! 4. 全局根 `/`

use std::path::{Path, PathBuf};

use crate::error::{DocError, DocResult};
use crate::vfs::VirtualFs;

/// 路径解算上下文。
#[derive(Debug, Default, Clone)]
pub struct PathResolver {
    /// 基础目录（来自当前 `.tex` 文件位置）
    pub base_dir: Option<PathBuf>,
    /// 来自 `\graphicspath{}` 的附加搜索路径
    pub graphics_paths: Vec<PathBuf>,
}

impl PathResolver {
    /// 创建空解析器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加 `\graphicspath{}` 路径。
    pub fn push_graphics_path<P: Into<PathBuf>>(&mut self, p: P) {
        self.graphics_paths.push(p.into());
    }

    /// 在 VFS 中按优先级查找 `target`，找不到返回 `None`。
    pub fn resolve<P: AsRef<Path>>(&self, vfs: &VirtualFs, target: P) -> Option<PathBuf> {
        let target = target.as_ref();
        let candidates = self.candidates(target);
        vfs.first_existing(candidates.iter())
    }

    /// 在真实文件系统按同样优先级查找（用于 CLI 直接读盘场景）。
    pub fn resolve_real<P: AsRef<Path>>(&self, target: P) -> Option<PathBuf> {
        let target = target.as_ref();
        self.candidates(target)
            .into_iter()
            .find(|cand| cand.exists())
    }

    fn candidates(&self, target: &Path) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();

        // 1. 原样
        out.push(normalize(target.to_path_buf()));
        // 1b. 自动补 .tex 扩展（LaTeX `\input{file}` 约定）
        push_with_tex_ext(&mut out, target);

        // 2. 相对 base_dir
        if let Some(base) = &self.base_dir {
            let joined = base.join(target);
            out.push(normalize(joined.clone()));
            push_with_tex_ext(&mut out, &joined);
        }

        // 3. graphicspath
        for gp in &self.graphics_paths {
            let joined = gp.join(target);
            out.push(normalize(joined.clone()));
            push_with_tex_ext(&mut out, &joined);
        }

        out
    }
}

fn normalize(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy().replace('\\', "/");
    PathBuf::from(s)
}

/// 若 `target` 不带 LaTeX 识别的扩展名（`.tex` / `.ltx` / `.cls` / `.sty` / `.bib`），
/// 追加 `.tex` 作为候选，模拟 LaTeX `\input{file}` 的默认行为。
fn push_with_tex_ext(out: &mut Vec<PathBuf>, target: &Path) {
    let has_known_ext = target
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            e == "tex" || e == "ltx" || e == "cls" || e == "sty" || e == "bib"
        })
        .unwrap_or(false);
    if !has_known_ext {
        let mut p = target.to_path_buf();
        let cur = p.extension().map(|e| e.to_os_string()).unwrap_or_default();
        if !cur.is_empty() {
            // 已有未知扩展：保留并追加 .tex（不覆盖）
            let mut s = p.into_os_string();
            s.push(".tex");
            p = s.into();
        } else {
            p.set_extension("tex");
        }
        out.push(normalize(p));
    }
}

/// 从 `\graphicspath{{a/{b}c}}` 这类花括号嵌套中提取路径列表。
///
/// V1 仅支持花括号顶级直接列表，不展开宏。
pub fn parse_graphics_path(body: &str) -> DocResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in body.chars() {
        match ch {
            '{' => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
                cur.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 && !cur.is_empty() {
                    out.push(PathBuf::from(cur.trim()));
                    cur.clear();
                    continue;
                }
                cur.push(ch);
            }
            _ if depth >= 1 => cur.push(ch),
            _ => {}
        }
    }
    if depth != 0 {
        return Err(DocError::InvalidPath(format!(
            "graphicspath 花括号未闭合：{body}"
        )));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphics_path_simple() {
        let paths = parse_graphics_path("{a/}{b/}{c}").unwrap();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("a/"),
                PathBuf::from("b/"),
                PathBuf::from("c/")
            ]
        );
    }

    #[test]
    fn graphics_path_unbalanced_err() {
        assert!(parse_graphics_path("{a/}{b").is_err());
    }

    #[test]
    fn resolve_via_base_dir() {
        let mut vfs = VirtualFs::new();
        vfs.insert("proj/sub/inc.tex", b"x".to_vec());
        let mut r = PathResolver::new();
        r.base_dir = Some(PathBuf::from("proj/sub"));
        let hit = r.resolve(&vfs, "inc.tex").unwrap();
        assert_eq!(hit, PathBuf::from("proj/sub/inc.tex"));
        assert!(vfs.contains(&hit));
    }

    #[test]
    fn resolve_via_graphics_path() {
        let mut vfs = VirtualFs::new();
        vfs.insert("fig/a.pdf", b"x".to_vec());
        let mut r = PathResolver::new();
        r.push_graphics_path("fig");
        let hit = r.resolve(&vfs, "a.pdf").unwrap();
        assert_eq!(hit, PathBuf::from("fig/a.pdf"));
    }
}
