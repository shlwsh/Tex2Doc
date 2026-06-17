//! include 拓扑（Pass-1）
//!
//! 算法：
//! 1. 在 VFS 中查找 `.tex` 主文件。
//! 2. 词法扫描 `\include{...}` / `\input{...}` 指令。
//! 3. 通过 [`doc_utils::PathResolver`] 解析相对路径。
//! 4. 递归构建 DAG，遇环报错 [`IncludeError::Cycle`]。
//! 5. 输出**按拓扑序**拼接的 Token 流（每 token 携带 `SourceId`）。

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use doc_utils::{parse_graphics_path, DocError, DocResult, PathResolver, VirtualFs};
use thiserror::Error;

use doc_semantic_ast::SourceId;

/// include 阶段错误。
#[derive(Debug, Error)]
pub enum IncludeError {
    #[error("include 循环：{0:?}")]
    Cycle(Vec<PathBuf>),

    #[error("include 目标未找到：{0}")]
    NotFound(PathBuf),
}

impl From<IncludeError> for DocError {
    fn from(err: IncludeError) -> Self {
        DocError::InvalidPath(err.to_string())
    }
}

/// include 图。
#[derive(Debug, Default, Clone)]
pub struct IncludeGraph {
    /// SourceId → 路径
    pub sources: Vec<PathBuf>,
    /// 路径 → SourceId
    pub by_path: HashMap<PathBuf, SourceId>,
    /// 邻接表：SourceId → [SourceId]
    pub edges: HashMap<SourceId, Vec<SourceId>>,
    /// 全局 `\graphicspath{}` 路径集合
    pub graphics_paths: Vec<PathBuf>,
}

impl IncludeGraph {
    /// 构造空图。
    pub fn new() -> Self {
        Self::default()
    }

    /// 入口：从主 `.tex` 构建拓扑。
    pub fn build(vfs: &VirtualFs, main: &Path) -> DocResult<Self> {
        let mut g = Self::new();
        let main_norm = normalize(main);
        g.add_node(main_norm.clone());
        let mut stack: Vec<(PathBuf, HashSet<PathBuf>)> = vec![(main_norm.clone(), {
            let mut s = HashSet::new();
            s.insert(main_norm.clone());
            s
        })];
        let mut queue: VecDeque<PathBuf> = VecDeque::new();
        queue.push_back(main_norm);

        while let Some(file) = queue.pop_front() {
            let body = vfs.read(&file)?.to_vec();
            let text = std::str::from_utf8(&body).map_err(|e| {
                DocError::InvalidPath(format!("非 UTF-8 源文件 {}: {e}", file.display()))
            })?;
            let mut resolver = PathResolver::new();
            resolver.base_dir = file.parent().map(Path::to_path_buf);
            resolver.graphics_paths = g.graphics_paths.clone();
            for (cmd, target) in scan_includes(text) {
                if matches!(cmd, "graphicspath") {
                    if let Ok(paths) = parse_graphics_path(&target) {
                        for p in paths {
                            g.graphics_paths.push(p);
                        }
                    }
                    continue;
                }
                if let Some(hit) = resolver.resolve(vfs, &target) {
                    let hit = normalize(&hit);
                    if stack.iter().any(|(_, set)| set.contains(&hit)) {
                        return Err(IncludeError::Cycle(vec![hit]).into());
                    }
                    if !g.by_path.contains_key(&hit) {
                        g.add_node(hit.clone());
                        let mut set = HashSet::new();
                        set.insert(hit.clone());
                        stack.push((hit.clone(), set));
                        queue.push_back(hit.clone());
                    }
                    let from = g.by_path[&file];
                    let to = g.by_path[&hit];
                    g.edges.entry(from).or_default().push(to);
                } else {
                    return Err(IncludeError::NotFound(PathBuf::from(&target)).into());
                }
            }
        }
        Ok(g)
    }

    fn add_node(&mut self, path: PathBuf) {
        let id = SourceId(self.sources.len() as u32);
        self.by_path.insert(path.clone(), id);
        self.sources.push(path);
    }

    /// 拓扑序（Kahn）。
    pub fn topo_order(&self) -> DocResult<Vec<SourceId>> {
        let mut indeg: HashMap<SourceId, usize> =
            self.sources.iter().map(|p| (self.by_path[p], 0)).collect();
        for tos in self.edges.values() {
            for t in tos {
                *indeg.get_mut(t).unwrap() += 1;
            }
        }
        let mut q: VecDeque<SourceId> = indeg
            .iter()
            .filter_map(|(k, v)| (*v == 0).then_some(*k))
            .collect();
        let mut out = Vec::new();
        while let Some(n) = q.pop_front() {
            out.push(n);
            if let Some(next) = self.edges.get(&n) {
                for t in next {
                    let e = indeg.get_mut(t).unwrap();
                    *e -= 1;
                    if *e == 0 {
                        q.push_back(*t);
                    }
                }
            }
        }
        if out.len() != self.sources.len() {
            return Err(IncludeError::Cycle(self.sources.clone()).into());
        }
        Ok(out)
    }

    /// 全部图片搜索路径（V1 简化：base_dir + graphicspath）。
    pub fn graphics_resolver(&self) -> PathResolver {
        let mut r = PathResolver::new();
        for p in &self.graphics_paths {
            r.push_graphics_path(p);
        }
        r
    }
}

/// 拓扑序拼接后的「单流文本」，并附 `SourceMap`。
pub struct JoinedStream {
    pub text: String,
    /// 每个字符的 `SourceId`（长度 == text.len()）
    pub source_map: Vec<SourceId>,
    /// VFS 引用（用于 VFS 感知的宏展开）
    pub vfs: doc_utils::VirtualFs,
}

impl IncludeGraph {
    /// 按主文件中的 `\input` / `\include` 位置原位展开为单流。
    pub fn join(&self, vfs: &VirtualFs) -> DocResult<JoinedStream> {
        let mut text = String::new();
        let mut map = Vec::new();
        if let Some(main) = self.sources.first() {
            let mut stack = Vec::new();
            self.append_file_inline(vfs, main, &mut stack, &mut text, &mut map)?;
        }
        Ok(JoinedStream {
            text,
            source_map: map,
            vfs: vfs.clone(),
        })
    }

    fn append_file_inline(
        &self,
        vfs: &VirtualFs,
        path: &Path,
        stack: &mut Vec<PathBuf>,
        text: &mut String,
        map: &mut Vec<SourceId>,
    ) -> DocResult<()> {
        let path = normalize(path);
        if stack.contains(&path) {
            return Err(IncludeError::Cycle(stack.clone()).into());
        }
        let id = *self
            .by_path
            .get(&path)
            .ok_or_else(|| IncludeError::NotFound(path.clone()))?;
        stack.push(path.clone());

        let body = vfs.read(&path)?;
        let s = std::str::from_utf8(body)
            .map_err(|e| DocError::InvalidPath(format!("非 UTF-8 {}: {e}", path.display())))?;
        let mut cursor = 0usize;
        while let Some(inc) = find_next_file_include(s, cursor) {
            push_text_with_source(text, map, id, &s[cursor..inc.start]);

            let mut resolver = PathResolver::new();
            resolver.base_dir = path.parent().map(Path::to_path_buf);
            resolver.graphics_paths = self.graphics_paths.clone();
            let Some(hit) = resolver.resolve(vfs, &inc.target) else {
                return Err(IncludeError::NotFound(PathBuf::from(&inc.target)).into());
            };
            self.append_file_inline(vfs, &hit, stack, text, map)?;
            cursor = inc.end;
        }
        push_text_with_source(text, map, id, &s[cursor..]);
        stack.pop();
        Ok(())
    }
}

struct IncludeCommand {
    start: usize,
    end: usize,
    target: String,
}

fn push_text_with_source(text: &mut String, map: &mut Vec<SourceId>, id: SourceId, chunk: &str) {
    text.push_str(chunk);
    for _ in 0..chunk.len() {
        map.push(id);
    }
}

fn find_next_file_include(text: &str, from: usize) -> Option<IncludeCommand> {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }
        let cmd_start = i + 1;
        let mut j = cmd_start;
        while j < bytes.len() && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'@') {
            j += 1;
        }
        if j == cmd_start {
            i += 1;
            continue;
        }
        let cmd = &text[cmd_start..j];
        if !matches!(cmd, "include" | "input" | "subfile") {
            i = j;
            continue;
        }
        let mut k = j;
        while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
            k += 1;
        }
        if k >= bytes.len() || bytes[k] != b'{' {
            i = j;
            continue;
        }
        let mut depth = 1i32;
        let body_start = k + 1;
        let mut m = body_start;
        while m < bytes.len() && depth > 0 {
            match bytes[m] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            m += 1;
        }
        if depth != 0 {
            return None;
        }
        return Some(IncludeCommand {
            start: i,
            end: m + 1,
            target: text[body_start..m].to_string(),
        });
    }
    None
}

fn normalize(p: &Path) -> PathBuf {
    PathBuf::from(p.to_string_lossy().replace('\\', "/"))
}

/// 极简 include 扫描：识别 `\include{file}` / `\input{file}` / `\graphicspath{...}`。
///
/// 返回 `(命令名, 路径内容)`；忽略 `{}` 内嵌分组（V1 简化）。
fn scan_includes(text: &str) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // 命令名
            let cmd_start = i + 1;
            let mut j = cmd_start;
            while j < bytes.len() && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'@') {
                j += 1;
            }
            if j == cmd_start {
                i += 1;
                continue;
            }
            let cmd = &text[cmd_start..j];
            // 跳过可选空白
            let mut k = j;
            while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                k += 1;
            }
            if k >= bytes.len() || bytes[k] != b'{' {
                i = j;
                continue;
            }
            // 配对 `}`
            let mut depth = 1i32;
            let body_start = k + 1;
            let mut m = body_start;
            while m < bytes.len() && depth > 0 {
                match bytes[m] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                m += 1;
            }
            if depth != 0 {
                // 不闭合：跳出（不致命）
                break;
            }
            let body = text[body_start..m].to_string();
            match cmd {
                "include" | "input" | "subfile" => out.push(("include", body)),
                "graphicspath" => out.push(("graphicspath", body)),
                _ => {}
            }
            i = m + 1;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_includes_basic() {
        let s = r"\input{a.tex} \include{b} \graphicspath{{figs/}}";
        let v = scan_includes(s);
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], ("include", "a.tex".into()));
        assert_eq!(v[1], ("include", "b".into()));
        assert_eq!(v[2], ("graphicspath", "{figs/}".into()));
    }

    #[test]
    fn build_topology() {
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", b"\\input{sub.tex}".to_vec());
        vfs.insert("sub.tex", b"hello".to_vec());
        let g = IncludeGraph::build(&vfs, std::path::Path::new("main.tex")).unwrap();
        assert_eq!(g.sources.len(), 2);
        let order = g.topo_order().unwrap();
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn join_inlines_inputs_at_source_position() {
        let mut vfs = VirtualFs::new();
        vfs.insert(
            "main.tex",
            b"before\n\\input{sections/sub.tex}\nafter".to_vec(),
        );
        vfs.insert("sections/sub.tex", b"middle".to_vec());
        let g = IncludeGraph::build(&vfs, std::path::Path::new("main.tex")).unwrap();
        let joined = g.join(&vfs).unwrap();

        assert!(joined.text.contains("before\nmiddle\nafter"));
        assert!(!joined.text.contains("\\input"));
        assert_eq!(joined.text.len(), joined.source_map.len());
    }

    #[test]
    fn cycle_detected() {
        let mut vfs = VirtualFs::new();
        vfs.insert("a.tex", b"\\input{b.tex}".to_vec());
        vfs.insert("b.tex", b"\\input{a.tex}".to_vec());
        let err = IncludeGraph::build(&vfs, std::path::Path::new("a.tex")).unwrap_err();
        assert!(matches!(err, DocError::InvalidPath(_)));
    }
}
