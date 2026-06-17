use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use doc_latex_reader::{lower_to_standard_document, parse_tex, IncludeGraph};
use doc_semantic_ast::{SourceBundle, SourceFile, StandardDocument};
use doc_utils::VirtualFs;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AstDumpFormat {
    Md,
    Json,
}

#[derive(Debug, Args)]
pub struct AstDumpArgs {
    /// Project root containing the main TeX file.
    #[arg(long)]
    pub root: PathBuf,
    /// Main TeX path, absolute or relative to --root.
    #[arg(long)]
    pub main_tex: PathBuf,
    /// Format profile id used for AST metadata.
    #[arg(long, default_value = "jos-2025")]
    pub profile: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = AstDumpFormat::Md)]
    pub format: AstDumpFormat,
    /// Output path.
    #[arg(long)]
    pub out: PathBuf,
}

pub fn run(a: AstDumpArgs) -> Result<()> {
    let standard = build_standard_document(&a.root, &a.main_tex, a.profile.clone())?;
    write_standard_dump(&standard, a.format, &a.out)?;
    tracing::info!("写入 AST dump：{}", a.out.display());
    Ok(())
}

pub fn build_standard_document(
    root_arg: &Path,
    main_tex_arg: &Path,
    profile: String,
) -> Result<StandardDocument> {
    let root = root_arg
        .canonicalize()
        .with_context(|| format!("解析项目根目录失败：{}", root_arg.display()))?;
    let main_abs = if main_tex_arg.is_absolute() {
        main_tex_arg.to_path_buf()
    } else {
        root.join(main_tex_arg)
    };
    let main_rel = relative_to_root(&root, &main_abs)?;
    let main_posix = main_rel.to_string_lossy().replace('\\', "/");

    let mut vfs = VirtualFs::new();
    vfs.mount_dir(&root)
        .with_context(|| format!("挂载项目目录失败：{}", root.display()))?;
    let source_bytes = vfs
        .read(&main_posix)
        .with_context(|| format!("读取主文件失败：{main_posix}"))?;
    String::from_utf8(source_bytes.to_vec())
        .with_context(|| format!("主文件非 UTF-8：{main_posix}"))?;
    let graph = IncludeGraph::build(&vfs, Path::new(&main_posix))
        .map_err(|e| anyhow::anyhow!("构建 include 拓扑失败：{e}"))?;
    let joined = graph
        .join(&vfs)
        .map_err(|e| anyhow::anyhow!("拼接 include 流失败：{e}"))?;
    let parse = parse_tex(&joined.text);
    let source_bundle = SourceBundle {
        main_path: main_posix,
        files: collect_source_files(&vfs)?,
    };
    Ok(lower_to_standard_document(
        &parse,
        Some(&joined),
        source_bundle,
        profile,
    ))
}

pub fn write_standard_dump(
    standard: &StandardDocument,
    format: AstDumpFormat,
    out: &Path,
) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建输出目录失败：{}", parent.display()))?;
    }
    match format {
        AstDumpFormat::Md => {
            std::fs::write(out, standard.to_markdown())
                .with_context(|| format!("写 AST Markdown 失败：{}", out.display()))?;
        }
        AstDumpFormat::Json => {
            let json = serde_json::to_string_pretty(&standard)?;
            std::fs::write(out, json)
                .with_context(|| format!("写 AST JSON 失败：{}", out.display()))?;
        }
    }
    Ok(())
}

fn relative_to_root(root: &Path, path: &Path) -> Result<PathBuf> {
    let abs = path
        .canonicalize()
        .with_context(|| format!("解析路径失败：{}", path.display()))?;
    abs.strip_prefix(root)
        .map(PathBuf::from)
        .with_context(|| format!("{} 不在项目根目录 {} 下", abs.display(), root.display()))
}

fn collect_source_files(vfs: &VirtualFs) -> Result<Vec<SourceFile>> {
    vfs.paths()
        .filter(|path| path.to_string_lossy().ends_with(".tex"))
        .map(|path| {
            let path_text = path.to_string_lossy().replace('\\', "/");
            let bytes = vfs
                .read(&path_text)
                .with_context(|| format!("读取源文件 hash 失败：{path_text}"))?;
            Ok(SourceFile {
                path: path_text,
                hash: Some(format!("blake3:{}", blake3::hash(bytes).to_hex())),
            })
        })
        .collect()
}
