# 关键技术 1：多文件 LaTeX 拓扑与拼接

> 本节深入解析 `doc-latex-reader::include` 模块（Pass-1）。解决的核心问题：真实 LaTeX 工程是「多文件 + 互相 include + 图片散落各处」，转换器必须先解析出完整的源码图，才能做下游解析。

---

## 1. 核心数据结构

```rust
// crates/latex-reader/src/include.rs
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

pub struct JoinedStream {
    pub text: String,                     // 拼接后的单流文本
    pub source_map: Vec<SourceId>,        // 每个字符的源文件 ID
}
```

* `SourceId(u32)`：`doc-semantic-ast` 定义；`IncludeGraph` 按发现顺序分配（`0..N`）。
* `source_map`：长度等于 `text.len()`；用于错误信息定位「这段文本来自哪个文件」。

---

## 2. Pass-1 算法

### 2.1 入口

```rust
pub fn build(vfs: &VirtualFs, main: &Path) -> DocResult<Self> {
    let mut g = Self::new();
    let main_norm = normalize(main);
    g.add_node(main_norm.clone());
    let mut stack: Vec<(PathBuf, HashSet<PathBuf>)> = vec![(main_norm.clone(), /* 当前路径集合 */)];
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(main_norm);

    while let Some(file) = queue.pop_front() {
        let body = vfs.read(&file)?.to_vec();
        let text = std::str::from_utf8(&body)
            .map_err(|e| DocError::InvalidPath(format!("非 UTF-8 源文件 {}: {e}", file.display())))?;
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
                // 环检测
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
```

### 2.2 关键步骤

1. **BFS 队列**：`queue` 跟踪待扫描文件；`stack` 跟踪当前递归路径（环检测用）。
2. **节点添加**：`add_node` 分配 `SourceId`。
3. **include 扫描**：`scan_includes(text)` 字符级识别 `\include` / `\input` / `\subfile` / `\graphicspath`。
4. **路径解析**：`PathResolver::resolve(vfs, target)` 找到第一个存在的候选。
5. **环检测**：`stack` 中所有 `set` 都不含 `hit` 才合法。
6. **边添加**：`from → to`（from 是当前文件，to 是被 include 的文件）。

### 2.3 错误类型

```rust
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
```

* `Cycle` → `DocError::InvalidPath` → `CoreError::Parse`。
* `NotFound` → 同上。

---

## 3. include 扫描器（`scan_includes`）

### 3.1 算法

```rust
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
                        if depth == 0 { break; }
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
```

### 3.2 关键性质

* **字节级**：`bytes[i]`，不做 UTF-8 解码。
* **ASCII 字母**：`is_ascii_alphabetic() || b'@'`。
* **括号配对**：depth 计数，支持嵌套。
* **不闭合 → 跳出**（不致命）。
* **不展开宏**：仅识别命令名。

### 3.3 与 LaTeX 真实语义的差异

| 真实 LaTeX | V1 处理 |
|------------|---------|
| `\input file`（无花括号） | ❌ 不支持（必须 `{}`） |
| `\include{file}` | ✅ |
| `\subfile{file}` | ✅ |
| `\graphicspath{{a/}}` | ✅（但花括号内的花括号是 `body` 的一部分） |
| 命令名含数字 | ❌（仅字母 + @） |
| 注释 `%` 阻断命令 | ❌（简单字符扫描，可能误判） |

---

## 4. 路径解析（`PathResolver`）

### 4.1 候选路径优先级

```rust
// crates/utils/src/path.rs
fn candidates(&self, target: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    // 1. 原样
    out.push(normalize(target.to_path_buf()));
    // 1b. 自动补 .tex 扩展
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
```

### 4.2 `.tex` 扩展补全

```rust
fn push_with_tex_ext(out: &mut Vec<PathBuf>, target: &Path) {
    let has_known_ext = target.extension()
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
```

* 已知扩展（`.tex` / `.ltx` / `.cls` / `.sty` / `.bib`）：不补。
* 未知扩展：追加 `.tex`。
* 无扩展：设扩展为 `.tex`。

### 4.3 graphicspath 解析（`parse_graphics_path`）

```rust
pub fn parse_graphics_path(body: &str) -> DocResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in body.chars() {
        match ch {
            '{' => {
                depth += 1;
                if depth == 1 { continue; }  // 跳过最外层花括号
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
        return Err(DocError::InvalidPath(format!("graphicspath 花括号未闭合：{body}")));
    }
    Ok(out)
}
```

* 输入：`\graphicspath{{a/}{b/}{c}}` 接收的是 `{a/}{b/}{c}`（去除外层花括号）。
* 输出：`["a/", "b/", "c/"]`。
* 不支持宏展开（V1 简化）。

### 4.4 VFS 查找

```rust
impl VirtualFs {
    pub fn first_existing<'a, I, P>(&self, candidates: I) -> Option<PathBuf>
    where I: IntoIterator<Item = &'a P>, P: AsRef<Path> + ?Sized + 'a,
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
```

---

## 5. 拓扑排序

### 5.1 Kahn 算法

```rust
pub fn topo_order(&self) -> DocResult<Vec<SourceId>> {
    let mut indeg: HashMap<SourceId, usize> =
        self.sources.iter().map(|p| (self.by_path[p], 0)).collect();
    for tos in self.edges.values() {
        for t in tos {
            *indeg.get_mut(t).unwrap() += 1;
        }
    }
    let mut q: VecDeque<SourceId> = indeg.iter()
        .filter_map(|(k, v)| (*v == 0).then_some(*k)).collect();
    let mut out = Vec::new();
    while let Some(n) = q.pop_front() {
        out.push(n);
        if let Some(next) = self.edges.get(&n) {
            for t in next {
                let e = indeg.get_mut(t).unwrap();
                *e -= 1;
                if *e == 0 { q.push_back(*t); }
            }
        }
    }
    if out.len() != self.sources.len() {
        return Err(IncludeError::Cycle(self.sources.clone()).into());
    }
    Ok(out)
}
```

* 时间复杂度：O(V + E)。
* 环检测：输出节点数 ≠ 总节点数 → 有环。

### 5.2 拼接（`join`）

```rust
pub fn join(&self, vfs: &VirtualFs) -> DocResult<JoinedStream> {
    let order = self.topo_order()?;
    let mut text = String::new();
    let mut map = Vec::new();
    for id in order {
        let path = &self.sources[id.0 as usize];
        let body = vfs.read(path)?;
        let s = std::str::from_utf8(body)
            .map_err(|e| DocError::InvalidPath(format!("非 UTF-8 {}: {e}", path.display())))?;
        text.push_str(s);
        for _ in 0..s.len() {
            map.push(id);
        }
        text.push('\n');
        map.push(id);
    }
    Ok(JoinedStream { text, source_map: map })
}
```

* 每个文件末尾追加 `\n` + 对应 `SourceId`（保证 `source_map` 长度 == `text.len()`）。

---

## 6. 在 `doc-core` 中的串联

```rust
// crates/core/src/convert.rs
fn parse_tex_with_vfs(
    main_tex: &str,
    _source: &str,
    vfs: &mut VirtualFs,
) -> Result<Document, CoreError> {
    let graph = IncludeGraph::build(vfs, Path::new(main_tex))?;
    let joined = graph.join(vfs)?;
    let parse = parse_tex(&joined.text);
    Ok(lower_to_document(&parse, Some(&joined)))
}
```

* `graph` 包含完整源码图（含 graphicspath）。
* `joined` 包含拼接后的单流文本 + source_map。
* `parse_tex` 在单流上做 Logos 词法 + Rowan 解析。
* `lower_to_document` 把 CST 降级为 `Document`（可访问 `source_map` 做错误定位）。

---

## 7. 已知限制与 V2 方向

| 当前限制 | 影响 | V2 方向 |
|----------|------|---------|
| 不展开宏 | `\input{\foo}` 不识别 | 宏展开后再扫 |
| 不处理注释 | `% \input{x}` 会被误扫 | 加 `// % 阻断` 逻辑 |
| 不识别 `\input file`（无括号） | LaTeX 罕见语法 | V2 加 |
| 不解析 `\IfFileExists{...}{...}{...}` | 条件 include 失效 | V2 加 |
| 不展开 `\subimport{path}{file}` | KOMA-Script 模板 | V2 加 |
| graphicspath 仅顶级 | 不支持嵌套 | 评估 |

---

## 8. 进一步阅读

* [02-lexer-and-cst.md](./02-lexer-and-cst.md) — Pass-2 词法 + 语法
* [03-semantic-lowering.md](./03-semantic-lowering.md) — Pass-3 降级
* [06-vfs-and-fonts.md](./06-vfs-and-fonts.md) — VFS 抽象
