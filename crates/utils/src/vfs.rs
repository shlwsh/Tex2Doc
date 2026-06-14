//! 虚拟文件系统（VFS）
//!
//! 以 `BTreeMap<PathBuf, Vec<u8>>` 为核心；支持：
//! - 内存写入（`insert` / `remove`）。
//! - 真实目录挂载（`mount_dir`，递归读取）。
//!
//! 设计目标：
//! - 让 LaTeX 解析器在「单文件流」与「多文件工程（zip / 真实目录）」之间无感切换。
//! - 路径查找采用大小写敏感（保留源文件的大小写信息）。

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::{DocError, DocResult};

/// 虚拟文件系统。
#[derive(Debug, Default, Clone)]
pub struct VirtualFs {
    files: BTreeMap<PathBuf, Vec<u8>>,
}

impl VirtualFs {
    /// 创建空 VFS。
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入一个内存文件。
    pub fn insert<P: Into<PathBuf>>(&mut self, path: P, bytes: Vec<u8>) {
        let path = normalize_path(path.into());
        self.files.insert(path, bytes);
    }

    /// 删除一个文件；若不存在返回 `false`。
    pub fn remove<P: AsRef<Path>>(&mut self, path: P) -> bool {
        let path = normalize_path(path.as_ref().to_path_buf());
        self.files.remove(&path).is_some()
    }

    /// 读取一个文件；不存在时返回 [`DocError::VfsMissing`]。
    pub fn read<P: AsRef<Path>>(&self, path: P) -> DocResult<&[u8]> {
        let path = normalize_path(path.as_ref().to_path_buf());
        self.files
            .get(&path)
            .map(Vec::as_slice)
            .ok_or(DocError::VfsMissing(path))
    }

    /// 探测一个文件是否存在。
    pub fn contains<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = normalize_path(path.as_ref().to_path_buf());
        self.files.contains_key(&path)
    }

    /// 列出所有路径（不可变迭代）。
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.files.keys()
    }

    /// 递归挂载真实目录到 VFS（相对路径取自 `root`）。
    pub fn mount_dir(&mut self, root: &Path) -> io::Result<usize> {
        let mut count = 0usize;
        for entry in walk_dir(root)? {
            let rel = entry
                .strip_prefix(root)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            let bytes = fs::read(&entry)?;
            // 以「相对 root」作为 key，与 LaTeX include 解析（相对路径）保持一致。
            let key = normalize_path(rel.to_path_buf());
            self.files.insert(key, bytes);
            count += 1;
        }
        Ok(count)
    }

    /// 探测候选路径集合，按顺序返回第一个存在的。
    pub fn first_existing<'a, I, P>(&self, candidates: I) -> Option<PathBuf>
    where
        I: IntoIterator<Item = &'a P>,
        P: AsRef<Path> + ?Sized + 'a,
    {
        for c in candidates {
            let p = normalize_path(c.as_ref().to_path_buf());
            if self.files.contains_key(&p) {
                return Some(p);
            }
        }
        None
    }
}

fn walk_dir(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let p = entry.path();
            let ft = entry.file_type()?;
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() {
                out.push(p);
            }
        }
    }
    Ok(out)
}

/// 路径归一化：统一使用正斜杠、剔除空段，便于跨平台比较。
pub(crate) fn normalize_path(p: PathBuf) -> PathBuf {
    let mut s = p.to_string_lossy().replace('\\', "/");
    // 折叠 "./"
    while s.contains("/./") {
        s = s.replace("/./", "/");
    }
    if s.starts_with("./") {
        s.drain(..2);
    }
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_read() {
        let mut vfs = VirtualFs::new();
        vfs.insert("a/b.tex", b"hello".to_vec());
        assert_eq!(vfs.read("a/b.tex").unwrap(), b"hello");
    }

    #[test]
    fn missing_returns_error() {
        let vfs = VirtualFs::new();
        let err = vfs.read("nope.tex").unwrap_err();
        matches!(err, DocError::VfsMissing(_));
    }

    #[test]
    fn normalize_windows_sep() {
        let p = normalize_path(PathBuf::from("a\\b\\c.tex"));
        assert_eq!(p, PathBuf::from("a/b/c.tex"));
    }

    #[test]
    fn first_existing_returns_first() {
        let mut vfs = VirtualFs::new();
        vfs.insert("a.tex", b"x".to_vec());
        let hit = vfs.first_existing(["missing.tex", "a.tex", "b.tex"]);
        assert_eq!(hit.as_deref(), Some(std::path::Path::new("a.tex")));
    }

    #[test]
    fn remove_returns_bool() {
        let mut vfs = VirtualFs::new();
        vfs.insert("a.tex", b"x".to_vec());
        assert!(vfs.remove("a.tex"));
        assert!(!vfs.remove("a.tex"));
    }
}
