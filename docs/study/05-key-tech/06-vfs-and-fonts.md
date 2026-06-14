# 关键技术 6：VFS 抽象与字体探测

> 本节深入解析 `doc-utils` 的核心抽象——虚拟文件系统（VFS）和字体探测（FontDetector）。解决的核心问题：让 LaTeX 解析器在「单文件流」与「多文件工程（zip / 真实目录）」之间无感切换；同时正确处理 CTeX 字体到 Office 字体的映射。

---

## 1. 虚拟文件系统（`vfs.rs`）

### 1.1 数据结构

```rust
#[derive(Debug, Default, Clone)]
pub struct VirtualFs {
    files: BTreeMap<PathBuf, Vec<u8>>,
}
```

* `BTreeMap`：有序迭代；键为规范化路径（POSIX 风格）。
* `Vec<u8>`：原始字节流。
* 完整实现：~250 行。

### 1.2 核心方法

```rust
impl VirtualFs {
    pub fn new() -> Self;
    pub fn insert<P: Into<PathBuf>>(&mut self, path: P, bytes: Vec<u8>);
    pub fn remove<P: AsRef<Path>>(&mut self, path: P) -> bool;
    pub fn read<P: AsRef<Path>>(&self, path: P) -> DocResult<&[u8]>;
    pub fn contains<P: AsRef<Path>>(&self, path: P) -> bool;
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf>;
    pub fn mount_dir(&mut self, root: &Path) -> io::Result<usize>;
    pub fn first_existing<'a, I, P>(&self, candidates: I) -> Option<PathBuf>;
}
```

### 1.3 路径规范化

```rust
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
```

* 统一正斜杠。
* 折叠 `./`。
* 剥离前缀 `./`。
* **大小写敏感**（保留源文件大小写信息）。

### 1.4 `mount_dir` —— 真实目录挂载

```rust
pub fn mount_dir(&mut self, root: &Path) -> io::Result<usize> {
    let mut count = 0usize;
    for entry in walk_dir(root)? {
        let rel = entry.strip_prefix(root)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let bytes = fs::read(&entry)?;
        let key = normalize_path(rel.to_path_buf());
        self.files.insert(key, bytes);
        count += 1;
    }
    Ok(count)
}

fn walk_dir(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let p = entry.path();
            let ft = entry.file_type()?;
            if ft.is_dir() { stack.push(p); }
            else if ft.is_file() { out.push(p); }
        }
    }
    Ok(out)
}
```

* 显式栈（非递归），避免深目录栈溢出。
* 跳 `.git` / `node_modules` 之类由调用方过滤（`mount_dir` 自身不过滤）。
* 全部 `fs::read` 后插入 VFS。

### 1.5 `first_existing` —— 候选路径查找

```rust
pub fn first_existing<'a, I, P>(&self, candidates: I) -> Option<PathBuf>
where I: IntoIterator<Item = &'a P>, P: AsRef<Path> + ?Sized + 'a {
    for c in candidates {
        let p = normalize_path(c.as_ref().to_path_buf());
        if self.files.contains_key(&p) { return Some(p); }
    }
    None
}
```

* 第一个命中即返回。
* 与 `PathResolver` 配合做 `\input` / `\include` / `\includegraphics` 的多级查找。

### 1.6 测试

* `insert_and_read`：基本读写。
* `missing_returns_error`：缺失返 `DocError::VfsMissing`。
* `normalize_windows_sep`：`a\b\c.tex` → `a/b/c.tex`。
* `first_existing_returns_first`：按顺序返回第一个命中。
* `remove_returns_bool`：删除返回 bool。

### 1.7 在 `doc-core` 中的使用

```rust
// crates/core/src/convert.rs
pub fn convert_dir(project_root: &Path, main_tex: &Path, options: &ConvertOptions) -> Result<ConvertResult, CoreError> {
    let mut vfs = VirtualFs::new();
    vfs.mount_dir(project_root).map_err(|e| CoreError::Io(e.to_string()))?;
    // 扫描 PNG/JPEG → image_assets
    for path in vfs.paths() {
        let p_lower = path.to_string_lossy().to_lowercase();
        if p_lower.ends_with(".png") || p_lower.ends_with(".jpg") || p_lower.ends_with(".jpeg") {
            if let Ok(bytes) = vfs.read(path) {
                image_assets.insert(p_str.to_string(), bytes.to_vec());
            }
        }
    }
    // 装载主文件
    let main_posix = main_rel.to_string_lossy().replace('\\', "/");
    let source_bytes = vfs.read(&main_posix)?.to_vec();
    let source = String::from_utf8(source_bytes)?;
    // Pass-1/2/3
    let doc = parse_tex_with_vfs(&main_posix, &source, &mut vfs)?;
    // Pass-4
    let docx = doc_docx_writer::pack_with_assets(&doc, options.template_bytes.as_deref(), Some(&image_assets))?;
    Ok(ConvertResult { docx, warnings: vec![] })
}
```

### 1.8 zip 模式（`convert_zip`）

```rust
pub fn convert_zip(zip_bytes: &[u8], main_tex_path: &str, options: &ConvertOptions) -> Result<ConvertResult, CoreError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(zip_io_to_core)?;
    let mut entries: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| CoreError::Io(format!("读取 zip 索引 {i} 失败：{e}")))?;
        if f.is_dir() { continue; }
        let name = f.name().to_string();
        if name.contains("..") {
            return Err(CoreError::Parse(format!("zip 包含不安全路径：{name}")));
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).map_err(|e| CoreError::Io(format!("读取 zip 条目 {name} 失败：{e}")))?;
        entries.insert(PathBuf::from(name.replace('\\', "/")), buf);
    }
    let main_norm = main_tex_path.replace('\\', "/");
    let source = String::from_utf8(entries[PathBuf::from(&main_norm)].clone())?;
    let mut vfs = VirtualFs::new();
    for (p, bytes) in &entries {
        vfs.insert(p, bytes.clone());
    }
    // 收集图片资产（同上）
    // Pass-1/2/3 + Pass-4
}
```

* `..` 拒绝：防 zip slip。
* 全部条目一次性装入内存（VFS 简化）。
* main 路径从 `entries` 取字节。

---

## 2. 路径解析（`path.rs`）

### 2.1 `PathResolver`

```rust
pub struct PathResolver {
    pub base_dir: Option<PathBuf>,
    pub graphics_paths: Vec<PathBuf>,
}

impl PathResolver {
    pub fn resolve<P: AsRef<Path>>(&self, vfs: &VirtualFs, target: P) -> Option<PathBuf>;
    pub fn resolve_real<P: AsRef<Path>>(&self, target: P) -> Option<PathBuf>;
}
```

* 候选路径优先级：
  1. 原样
  2. 自动补 `.tex` 扩展
  3. 相对 `base_dir` 同目录
  4. `\graphicspath{}` 声明路径
  5. 全局根 `/`
* VFS 查找 / 真实文件系统查找（两套 API）。

### 2.2 `parse_graphics_path`

```rust
pub fn parse_graphics_path(body: &str) -> DocResult<Vec<PathBuf>> {
    // 字符级扫描 `body`（去掉最外层花括号）
    // 匹配 {a/}{b/} 等花括号块
    // 返回 PathBuf 列表
}
```

* 输入：`\graphicspath{{a/}{b/}}` 接收 `{a/}{b/}`（去除外层花括号）。
* 输出：`["a/", "b/"]`。
* 不支持宏展开。

### 2.3 测试

* `graphics_path_simple`：基本解析。
* `graphics_path_unbalanced_err`：未闭合报错。
* `resolve_via_base_dir`：相对 `base_dir` 解析。
* `resolve_via_graphics_path`：graphicspath 解析。

---

## 3. 图片处理（`image.rs`）

### 3.1 `SupportedFormat`

```rust
pub enum SupportedFormat { Png, Jpeg }
impl SupportedFormat {
    pub fn probe(bytes: &[u8]) -> DocResult<Self>;        // 探测
    pub fn to_image_format(self) -> ImageFormat;
}
```

* V1 仅支持 PNG / JPEG。
* 其它格式 → `DocError::Unsupported`。

### 3.2 `ImageMeta` + `read_meta`

```rust
pub struct ImageMeta { pub width: u32, pub height: u32, pub format: SupportedFormat }
pub fn read_meta(bytes: &[u8]) -> DocResult<ImageMeta>;
```

* 读图片头部元信息（不解码像素）。

### 3.3 `renormalize`

```rust
pub fn renormalize(bytes: &[u8]) -> DocResult<(SupportedFormat, Vec<u8>)>;
```

* 重新编码为原格式（保证 `word/media/*` 字节合法）。
* 转换时间 = 几 ms / 图片。

### 3.4 `ImageAssets`

```rust
pub struct ImageAssets { inner: HashMap<String, Vec<u8>> }
impl ImageAssets {
    pub fn new() -> Self;
    pub fn insert(&mut self, path: String, bytes: Vec<u8>);
    pub fn get(&self, path: &str) -> Option<&[u8]>;
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

* 路径为 VFS 中的规范路径（如 `figs/a.png`），不含 `word/media/` 前缀。
* 由 `doc-core` 扫描 VFS 填充。
* 由 `doc-docx-writer::pack_with_assets` 消费。

### 3.5 测试

* `probe_png`：PNG 探测。
* `read_meta_size`：尺寸。
* `renormalize_roundtrip`：重新编码往返。

---

## 4. 字体探测（`fontdetect.rs`）

### 4.1 数据结构

```rust
pub enum FontStatus { Available, Embed, Fallback }
pub struct FontProbe {
    pub name: String,
    pub status: FontStatus,
    pub recommended: String,
    pub system_path: Option<PathBuf>,
}
pub struct FontDetector {
    system_dirs: Vec<PathBuf>,
    fallback: String,
    office_map: HashMap<String, String>,
}
```

### 4.2 探测流程

```rust
pub fn probe(&self, name: &str) -> FontProbe {
    // 1. 直接查系统
    if let Some(path) = self.find_system_font(name) {
        return FontProbe { name, status: Available, recommended: name, system_path: Some(path) };
    }
    // 2. 检查 Office 映射
    if let Some(office) = self.office_map.get(name) {
        if let Some(path) = self.find_system_font(office) {
            return FontProbe { name, status: Available, recommended: office, system_path: Some(path) };
        }
        return FontProbe { name, status: Embed, recommended: office, system_path: None };
    }
    // 3. 完全找不到 → Fallback
    FontProbe { name, status: Fallback, recommended: self.fallback.clone(), system_path: None }
}
```

### 4.3 系统字体目录探测

```rust
fn detect_system_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(target_os = "windows")] {
        if let Ok(windir) = std::env::var("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        }
        dirs.push(PathBuf::from("C:/Windows/Fonts"));
        dirs.push(PathBuf::from(r"C:\Windows/Fonts"));
    }
    #[cfg(target_os = "macos")] {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
    }
    #[cfg(target_os = "linux")] {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".fonts"));
            dirs.push(PathBuf::from(home).join(".local/share/fonts"));
        }
    }
    dirs
}
```

### 4.4 字体查找

```rust
fn find_system_font(&self, name: &str) -> Option<PathBuf> {
    let extensions = ["ttf", "otf", "TTF", "OTF"];
    let name_normalized = name.replace(' ', "");
    for dir in &self.system_dirs {
        if !dir.exists() { continue; }
        for ext in &extensions {
            let p = dir.join(format!("{}.{}", name, ext));
            if p.exists() { return Some(p); }
            let p2 = dir.join(format!("{}.{}", name_normalized, ext));
            if p2.exists() { return Some(p2); }
        }
    }
    None
}
```

* 精确匹配 + 去空格匹配。

### 4.5 Office 字体映射

```rust
fn default_office_font_map() -> HashMap<String, String> {
    let mut m = HashMap::new();
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
```

### 4.6 在 `doc-docx-writer` 中的应用

`styles.rs::apply_font_probes`：

```rust
pub fn apply_font_probes(styles_xml: &mut Vec<u8>, probes: &[FontProbe]) {
    if probes.is_empty() { return; }
    let xml_str = String::from_utf8_lossy(styles_xml).to_string();
    let mut modified = xml_str;
    for probe in probes {
        if probe.needs_fallback() {
            for attr in &["w:ascii", "w:hAnsi", "w:eastAsia", "w:cs"] {
                let pattern = format!("{}=\"{}\"", attr, probe.name);
                let replacement = format!("{}=\"{}\"", attr, probe.recommended);
                if modified.contains(&pattern) {
                    modified = modified.replace(&pattern, &replacement);
                }
            }
        }
    }
    *styles_xml = modified.into_bytes();
}
```

* V1 仅在 `Fallback` 状态下替换；`Embed` 状态仅在 docx 中声明（不嵌入字形）。

---

## 5. 字体映射（`fontmap.rs`）

### 5.1 数据结构

```rust
pub struct OfficeFont { pub ascii: String, pub east_asia: String }
pub struct FontMap { map: HashMap<String, OfficeFont> }
pub fn default_map() -> FontMap;
```

### 5.2 默认映射

| LaTeX 名 | Office ASCII | Office East Asia |
|----------|--------------|------------------|
| `songti` / `SimSun` / `宋体` | `SimSun` | `SimSun` |
| `heiti` / `SimHei` / `黑体` | `SimHei` | `SimHei` |
| `fangsong` / `FangSong` / `仿宋` | `FangSong` | `FangSong` |
| `kaishu` / `KaiTi` / `楷体` | `KaiTi` | `KaiTi` |
| `rm` | `Times New Roman` | `Times New Roman` |
| `tt` | `Consolas` | `Consolas` |

### 5.3 自定义

```rust
let mut m = default_map();
m.insert("songti", OfficeFont::single("Source Han Serif SC"));
m.insert("MyFont", OfficeFont::pair("Arial", "Source Han Serif SC"));
```

* 通过 `FontMap::insert` 覆盖默认映射。

---

## 6. 错误处理

```rust
// crates/utils/src/error.rs
#[derive(Debug, Error)]
pub enum DocError {
    #[error("VFS 路径未挂载：{0}")]
    VfsMissing(PathBuf),
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("路径解析失败：{0}")]
    InvalidPath(String),
    #[error("图片解码失败：{0}")]
    ImageDecode(String),
    #[error("不支持的操作：{0}")]
    Unsupported(String),
}
pub type DocResult<T> = std::result::Result<T, DocError>;
```

* 全部内部用 `DocResult`。
* `DocError` → `CoreError`（via `From`）。

---

## 7. 在三种入口的装载策略

| 入口 | 装载方式 | 备注 |
|------|----------|------|
| `convert_dir` | `vfs.mount_dir(project_root)` | 真实目录递归 |
| `convert_zip` | 逐 zip 条目 `vfs.insert(path, bytes)` | 拒绝 `..` |
| `convert_sync` | `vfs.insert(main_tex, source.as_bytes())` | 仅主文件 |

---

## 8. 测试覆盖

| 文件 | 覆盖 |
|------|------|
| `src/vfs.rs::tests` | insert/read/missing/normalize/first_existing/remove |
| `src/path.rs::tests` | graphicspath 解析、相对 base_dir、graphicspath 解析 |
| `src/image.rs::tests` | PNG 探测、尺寸、重编码 |
| `src/fontmap.rs::tests` | 默认映射、自定义覆盖 |
| `src/fontdetect.rs::tests` | detector 创建、未知字体 fallback、Office 映射命中、Calibri 在 Windows 探测 |
| `tests/proptest.rs` | VFS 属性测试 |

---

## 9. 已知限制与 V2 方向

| 当前限制 | 影响 | V2 方向 |
|----------|------|---------|
| VFS 全部在内存 | 大项目（>100 MB）OOM | V2 内存映射文件 |
| `image` 仅支持 PNG/JPEG | EPS/PDF 报错 | V2 加 |
| 字体探测仅 ttf/otf | .ttc 集合不支持 | V2 加 |
| `apply_font_probes` 仅在 `Fallback` 替换 | `Embed` 状态不嵌字形 | V2 嵌字体文件 |
| Office 字体映射硬编码 | 自定义难 | V2 用户可配 |
| 字体回退链不完善 | Fallback 仅 Calibri | V2 多级回退 |

---

## 10. 进一步阅读

* [01-include-topology.md](./01-include-topology.md) — `PathResolver` 在 include 拓扑中的使用
* [04-docx-serialization.md](./04-docx-serialization.md) — 字体探测应用
* [04-architecture/01-end-to-end-pipeline.md](../../04-architecture/01-end-to-end-pipeline.md) — 端到端数据流
