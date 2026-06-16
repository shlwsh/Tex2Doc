//! `tex-compile` 子命令：把 zip 内的 TeX 项目编译为 oracle PDF。
//!
//! 设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md` + `05-implementation-roadmap.md` §5.3。
//!
//! 流程：
//! 1. 解 zip 到临时目录
//! 2. `TexProject::from_main(主文件)`
//! 3. `TexFacade::probe().compile_to_pdf(&proj)`
//! 4. 把产物 PDF 拷到 `--out`

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use doc_tex_facade::{EngineKind, TexFacade, TexProject};

#[derive(Debug, Args)]
pub struct TexCompileArgs {
    #[arg(long)]
    pub zip: PathBuf,
    #[arg(long)]
    pub main_tex: String,
    #[arg(long)]
    pub out: PathBuf,
    /// 指定引擎：xelatex / tectonic / latexmk；缺省自动探测
    #[arg(long)]
    pub engine: Option<String>,
    /// 编译最大轮数（含 bibtex）；默认 2
    #[arg(long, default_value_t = 2u32)]
    pub max_passes: u32,
}

pub async fn run(a: TexCompileArgs) -> Result<()> {
    // 1. 解 zip 到 temp
    let tmp = tempfile::tempdir().context("建临时目录失败")?;
    let workdir = tmp.path().to_path_buf();
    extract_zip(&a.zip, &workdir)?;

    let main_abs = workdir.join(&a.main_tex);
    if !main_abs.is_file() {
        anyhow::bail!("主文件不存在：{}", main_abs.display());
    }

    // 2. 构造项目
    let mut proj = TexProject::from_main(&main_abs).with_max_passes(a.max_passes);
    if let Some(name) = a.out.file_name().and_then(|s| s.to_str()) {
        proj.output_name = Some(name.to_string());
    }
    if let Some(s) = a.engine.as_deref() {
        proj.preferred = Some(match s {
            "xelatex" => EngineKind::Xelatex,
            "tectonic" => EngineKind::Tectonic,
            "latexmk" => EngineKind::Latexmk,
            other => anyhow::bail!("未知引擎：{other}"),
        });
    }

    // 3. 编译
    let facade = TexFacade::probe(&proj)
        .await
        .map_err(|e| anyhow::anyhow!("tex-facade 探测失败：{e}"))?;
    let pdf = facade
        .compile_to_pdf(&proj)
        .await
        .map_err(|e| anyhow::anyhow!("编译失败：{e}"))?;
    tracing::info!("TeX 编译完成：pdf={}", pdf.display());

    // 4. 拷到 out
    if let Some(parent) = a.out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::copy(&pdf, &a.out)
        .with_context(|| format!("拷到 {} 失败", a.out.display()))?;
    tracing::info!("已写入 oracle PDF：{}", a.out.display());
    Ok(())
}

fn extract_zip(zip: &std::path::Path, outdir: &std::path::Path) -> Result<()> {
    use std::io::Read;
    let bytes = std::fs::read(zip)
        .with_context(|| format!("读 zip 失败：{}", zip.display()))?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| anyhow::anyhow!("zip 解析失败：{e}"))?;
    for i in 0..archive.len() {
        let mut f = archive
            .by_index(i)
            .map_err(|e| anyhow::anyhow!("zip 第 {i} 项失败：{e}"))?;
        let name = f.name().to_string();
        if name.contains("..") {
            continue;
        }
        let rel = name.replace('\\', "/");
        let dst = outdir.join(&rel);
        if f.is_dir() {
            std::fs::create_dir_all(&dst).ok();
            continue;
        }
        if let Some(p) = dst.parent() {
            std::fs::create_dir_all(p).ok();
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf)?;
        std::fs::write(&dst, &buf)?;
    }
    Ok(())
}
