use std::path::PathBuf;

use doc_compiler_engine::{CompileOptions, EngineProfile, SemanticTexEngine};

fn main() {
    if let Err(err) = run() {
        eprintln!("paper3 compiler-engine conversion failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    std::fs::create_dir_all(
        args.out
            .parent()
            .ok_or_else(|| format!("output path has no parent: {}", args.out.display()))?,
    )?;

    let mut options = CompileOptions {
        profile: args.profile,
        page_setup: Some(doc_docx_writer::PageSetup::jos_paper3()),
        ..CompileOptions::default()
    };
    if args.no_standard_ast {
        options.collect_standard_ast = false;
    }

    let engine = SemanticTexEngine::new();
    let artifact = engine.compile_dir_to_docx(&args.project_root, &args.main_tex, &options)?;
    std::fs::write(&args.out, &artifact.docx)?;

    println!("docx: {}", args.out.display());
    println!("bytes: {}", artifact.docx.len());
    println!("blocks: {}", artifact.report.block_count);
    println!("image-assets: {}", artifact.report.image_asset_count);
    for stage in artifact.report.stages {
        println!("stage: {:?} {:?}", stage.stage, stage.status);
    }
    Ok(())
}

#[derive(Debug)]
struct Args {
    project_root: PathBuf,
    main_tex: PathBuf,
    out: PathBuf,
    profile: EngineProfile,
    no_standard_ast: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut project_root = None;
        let mut main_tex = None;
        let mut out = None;
        let mut profile = EngineProfile::JosPaper;
        let mut no_standard_ast = false;

        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--project-root" => project_root = Some(next_path(&mut args, "--project-root")?),
                "--main-tex" => main_tex = Some(next_path(&mut args, "--main-tex")?),
                "--out" => out = Some(next_path(&mut args, "--out")?),
                "--profile" => {
                    profile = parse_profile(&next_string(&mut args, "--profile")?)?;
                }
                "--no-standard-ast" => no_standard_ast = true,
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("unknown argument: {other}\n\n{}", usage())),
            }
        }

        Ok(Self {
            project_root: project_root.ok_or_else(usage)?,
            main_tex: main_tex.ok_or_else(usage)?,
            out: out.ok_or_else(usage)?,
            profile,
            no_standard_ast,
        })
    }
}

fn next_path(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    next_string(args, flag).map(PathBuf::from)
}

fn next_string(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {flag}\n\n{}", usage()))
}

fn parse_profile(raw: &str) -> Result<EngineProfile, String> {
    match raw {
        "generic" | "generic-article" => Ok(EngineProfile::GenericArticle),
        "chinese" | "chinese-academic" => Ok(EngineProfile::ChineseAcademic),
        "jos" | "jos-paper" => Ok(EngineProfile::JosPaper),
        "medical" | "medical-journal" => Ok(EngineProfile::MedicalJournal),
        other => Err(format!("unsupported profile: {other}\n\n{}", usage())),
    }
}

fn usage() -> String {
    "usage: cargo run -p doc-compiler-engine --example paper3_to_docx -- \\
  --project-root examples/paper3/latex \\
  --main-tex examples/paper3/latex/main-jos.tex \\
  --out examples/paper3/output/to-docx/paper3-compiler-engine.docx"
        .to_string()
}
