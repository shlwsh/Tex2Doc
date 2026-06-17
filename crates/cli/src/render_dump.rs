use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use doc_semantic_ast::{DocxRenderTree, MappingRegistry};

use crate::ast_dump;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RenderDumpFormat {
    Md,
    Json,
}

#[derive(Debug, Args)]
pub struct RenderDumpArgs {
    /// Project root containing the main TeX file.
    #[arg(long)]
    pub root: PathBuf,
    /// Main TeX path, absolute or relative to --root.
    #[arg(long)]
    pub main_tex: PathBuf,
    /// Format profile id used for AST metadata and mapping lookup.
    #[arg(long, default_value = "jos-2025")]
    pub profile: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = RenderDumpFormat::Md)]
    pub format: RenderDumpFormat,
    /// Output path.
    #[arg(long)]
    pub out: PathBuf,
}

pub fn run(a: RenderDumpArgs) -> Result<()> {
    let standard = ast_dump::build_standard_document(&a.root, &a.main_tex, a.profile.clone())?;
    let registry = MappingRegistry::for_profile(&a.profile);
    let render = DocxRenderTree::from_standard(&standard, &registry);
    write_render_dump(&render, a.format, &a.out)?;
    tracing::info!("写入 DOCX render dump：{}", a.out.display());
    Ok(())
}

fn write_render_dump(
    render: &DocxRenderTree,
    format: RenderDumpFormat,
    out: &PathBuf,
) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建输出目录失败：{}", parent.display()))?;
    }
    match format {
        RenderDumpFormat::Md => {
            std::fs::write(out, render.to_markdown())
                .with_context(|| format!("写 render Markdown 失败：{}", out.display()))?;
        }
        RenderDumpFormat::Json => {
            let json = serde_json::to_string_pretty(render)?;
            std::fs::write(out, json)
                .with_context(|| format!("写 render JSON 失败：{}", out.display()))?;
        }
    }
    Ok(())
}
