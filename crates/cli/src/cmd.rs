//! `convert` + `build` 子命令。

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use doc_core::{options::ConvertOptions, PageSetup};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PageSetupKind {
    /// US Letter 12240×15840 twips（V1 默认）
    Letter,
    /// A4 11906×16838 twips
    A4,
    /// JOS 18.40cm × 26.00cm 模板（=10433×14742 twips + 567/850/850/850 margins + 1 col）
    JosPaper3,
}

impl PageSetupKind {
    pub fn to_page_setup(self) -> Option<PageSetup> {
        match self {
            PageSetupKind::Letter => None, // V1 默认；不传 page_setup
            PageSetupKind::A4 => Some(PageSetup {
                width_twips: 11906,
                height_twips: 16838,
                margin_top: None,
                margin_right: None,
                margin_bottom: None,
                margin_left: None,
                margin_header: None,
                margin_footer: None,
                cols_space: None,
                cols_num: None,
                header_text: None,
                footer_text: None,
                first_header_text: None,
                first_footer_text: None,
                even_header_text: None,
                first_footer_indent_twips: None,
            }),
            PageSetupKind::JosPaper3 => Some(PageSetup::jos_paper3()),
        }
    }
}

#[derive(Debug, Args)]
pub struct ConvertArgs {
    /// 包含 .tex 的 zip 路径
    #[arg(long)]
    pub zip: PathBuf,
    /// zip 内的主 .tex 相对路径（POSIX）
    #[arg(long)]
    pub main_tex: String,
    /// 输出 docx 路径
    #[arg(long)]
    pub out: PathBuf,
    /// 页面设置：letter / a4 / jos-paper3
    #[arg(long, value_enum, default_value_t = PageSetupKind::Letter)]
    pub page_setup: PageSetupKind,
    /// V2：自定义页眉文本（支持多行 \\n + 占位符 `{{PAGE}}` / `{{NUMPAGES}}`）
    #[arg(long)]
    pub header_text: Option<String>,
    /// V2：自定义页脚文本（占位符同 header_text）
    #[arg(long)]
    pub footer_text: Option<String>,
    /// V2：首页页眉（覆盖 header_text）
    #[arg(long)]
    pub first_header_text: Option<String>,
    /// V2：首页页脚（覆盖 footer_text）
    #[arg(long)]
    pub first_footer_text: Option<String>,
}

pub fn run_convert(a: ConvertArgs) -> Result<()> {
    let bytes =
        std::fs::read(&a.zip).with_context(|| format!("读取 zip 失败：{}", a.zip.display()))?;
    let mut options = ConvertOptions::default();
    let mut ps = a.page_setup.to_page_setup();
    if let Some(ps_mut) = ps.as_mut() {
        ps_mut.header_text = a.header_text.clone();
        ps_mut.footer_text = a.footer_text.clone();
        ps_mut.first_header_text = a.first_header_text.clone();
        ps_mut.first_footer_text = a.first_footer_text.clone();
    } else {
        // 即便 page_setup 选 Letter（None），也允许自定义 header/footer：
        // 这里把 header/footer 套到 default() 上。
        if a.header_text.is_some()
            || a.footer_text.is_some()
            || a.first_header_text.is_some()
            || a.first_footer_text.is_some()
        {
            let mut ps2 = doc_docx_writer::PageSetup::default();
            ps2.header_text = a.header_text.clone();
            ps2.footer_text = a.footer_text.clone();
            ps2.first_header_text = a.first_header_text.clone();
            ps2.first_footer_text = a.first_footer_text.clone();
            ps = Some(ps2);
        }
    }
    options.page_setup = ps;
    let r =
        doc_core::convert_zip(&bytes, &a.main_tex, &options).map_err(|e| anyhow::anyhow!("{e}"))?;
    if let Some(parent) = a.out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&a.out, &r.docx)
        .with_context(|| format!("写 docx 失败：{}", a.out.display()))?;
    tracing::info!("写入 docx：{} ({} bytes)", a.out.display(), r.docx.len());
    Ok(())
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[arg(long)]
    pub zip: PathBuf,
    #[arg(long)]
    pub main_tex: String,
    #[arg(long)]
    pub outdir: PathBuf,
    #[arg(long, default_value_t = false)]
    pub skip_visual: bool,
    /// `--latex-main` 在 zip 内相对路径；缺省 = `--main-tex`
    #[arg(long)]
    pub latex_main: Option<String>,
    /// 页面设置：letter / a4 / jos-paper3
    #[arg(long, value_enum, default_value_t = PageSetupKind::Letter)]
    pub page_setup: PageSetupKind,
    /// V2：自定义页眉文本
    #[arg(long)]
    pub header_text: Option<String>,
    /// V2：自定义页脚文本
    #[arg(long)]
    pub footer_text: Option<String>,
    /// V2：首页页眉
    #[arg(long)]
    pub first_header_text: Option<String>,
    /// V2：首页页脚
    #[arg(long)]
    pub first_footer_text: Option<String>,
}

pub fn run_build(a: BuildArgs) -> Result<()> {
    use crate::{docx2pdf, pdf_verify, tex_compile};
    std::fs::create_dir_all(&a.outdir).ok();

    // ── 生成带版本号 + 时间戳的统一文件名 ──
    // 格式：<main_tex-stem>__v<version>__<yyyymmdd-hhmmss>
    let pkg_version = env!("CARGO_PKG_VERSION");
    let stem = std::path::Path::new(&a.main_tex)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("doc")
        .to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            // UTC+8 (CST) 简单换算
            let secs_cst = secs + 8 * 3600;
            let days = secs_cst / 86400;
            let day_secs = secs_cst % 86400;
            let hh = day_secs / 3600;
            let mm = (day_secs % 3600) / 60;
            let ss = day_secs % 60;
            // 1970-01-01 + days
            let (y, m, d) = days_to_ymd(days);
            format!("{:04}{:02}{:02}-{:02}{:02}{:02}", y, m, d, hh, mm, ss)
        })
        .unwrap_or_else(|_| "00000000-000000".to_string());
    let base = format!("{stem}__v{pkg_version}__{now}");

    let sh_docx = a.outdir.join(format!("{base}-sh.docx"));
    let rust_docx = a.outdir.join(format!("{base}-rust.docx"));
    let oracle_pdf = a.outdir.join(format!("{base}.oracle.pdf"));
    let rust_pdf = a.outdir.join(format!("{base}-rust.pdf"));
    let report_md = a.outdir.join(format!("{base}.quality-report.md"));
    let report_json = a.outdir.join(format!("{base}.quality-report.json"));

    tracing::info!("output basename: {base}");
    tracing::info!("sh docx: {}", sh_docx.display());
    tracing::info!("rust docx: {}", rust_docx.display());

    // 1. zip → docx（sh / rust 同步输出）
    run_convert(ConvertArgs {
        zip: a.zip.clone(),
        main_tex: a.main_tex.clone(),
        out: sh_docx.clone(),
        page_setup: PageSetupKind::Letter,
        header_text: None,
        footer_text: None,
        first_header_text: None,
        first_footer_text: None,
    })?;
    run_convert(ConvertArgs {
        zip: a.zip.clone(),
        main_tex: a.main_tex.clone(),
        out: rust_docx.clone(),
        page_setup: a.page_setup,
        header_text: a.header_text.clone(),
        footer_text: a.footer_text.clone(),
        first_header_text: a.first_header_text.clone(),
        first_footer_text: a.first_footer_text.clone(),
    })?;

    // 2. tex → oracle PDF
    let tex_a = tex_compile::TexCompileArgs {
        zip: a.zip.clone(),
        main_tex: a.latex_main.clone().unwrap_or_else(|| a.main_tex.clone()),
        out: oracle_pdf.clone(),
        engine: None,
        max_passes: 2,
    };
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(tex_compile::run(tex_a))?;

    // 3. docx → PDF（以 rust 版本为准）
    let d2p = docx2pdf::DocxToPdfArgs {
        docx: rust_docx.clone(),
        outdir: a.outdir.clone(),
    };
    rt.block_on(docx2pdf::run(d2p))?;
    let produced_pdf = a.outdir.join(format!(
        "{}.pdf",
        rust_docx
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("doc")
    ));
    if produced_pdf != rust_pdf && produced_pdf.exists() {
        std::fs::rename(&produced_pdf, &rust_pdf).ok();
    }

    // 4. 验证（rust DOCX vs PDF）
    let v = pdf_verify::VerifyPdfArgs {
        docx: rust_docx.clone(),
        rust_pdf: rust_pdf.clone(),
        oracle_pdf: oracle_pdf.clone(),
        report: report_md,
        json_report: report_json,
        diff_outdir: Some(a.outdir.join("diff")),
        skip_visual: a.skip_visual,
        thresholds: None,
    };
    rt.block_on(pdf_verify::run(v))?;
    Ok(())
}

/// 把自 1970-01-01 起的天数换算为 (year, month, day)（公历）。
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    let mut remaining = days;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let dy = if leap { 366 } else { 365 };
        if remaining < dy {
            break;
        }
        remaining -= dy;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let months = [
        31u64,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1u64;
    for &dm in &months {
        if remaining < dm {
            return (y, m, remaining + 1);
        }
        remaining -= dm;
        m += 1;
    }
    (y, 12, 31)
}
