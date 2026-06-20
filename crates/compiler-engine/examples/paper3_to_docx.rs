use std::path::PathBuf;

use doc_compiler_engine::{CompileOptions, EngineProfile, SemanticBackendKind, SemanticTexEngine};

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
        semantic_backend: args.semantic_backend,
        allow_backend_fallback: args.allow_backend_fallback,
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
    println!(
        "compatibility-score: {}",
        artifact.report.compatibility.score
    );
    println!(
        "compatibility-unsupported: {}",
        artifact.report.compatibility.unsupported.len()
    );
    println!(
        "compatibility-warnings: {}",
        artifact.report.compatibility.warnings.len()
    );
    println!(
        "compatibility-custom-macros: {}",
        artifact.report.compatibility.custom_macro_count
    );
    println!(
        "reference-labels: {}",
        artifact.report.reference_label_count
    );
    println!("reference-edges: {}", artifact.report.reference_edge_count);
    println!("citations: {}", artifact.report.citation_count);
    println!(
        "unresolved-references: {}",
        artifact.report.unresolved_reference_count
    );
    println!("bookmarks: {}", artifact.report.bookmark_count);
    println!("hyperlinks: {}", artifact.report.hyperlink_count);
    println!("omml-equations: {}", artifact.report.omml_equation_count);
    println!(
        "omml-equation-fallbacks: {}",
        artifact.report.omml_equation_fallback_count
    );
    println!(
        "backend-requested: {}",
        artifact.report.backend.requested.id()
    );
    println!(
        "backend-selected: {}",
        artifact.report.backend.selected.id()
    );
    if let Some(fallback_from) = artifact.report.backend.fallback_from {
        println!("backend-fallback-from: {}", fallback_from.id());
    }
    println!("backend-reason: {}", artifact.report.backend.reason);
    println!("profile-id: {}", artifact.report.profile_spec.id);
    println!(
        "profile-page-setup: {}",
        artifact.report.profile_spec.default_page_setup
    );
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
    semantic_backend: SemanticBackendKind,
    allow_backend_fallback: bool,
    no_standard_ast: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut project_root = None;
        let mut main_tex = None;
        let mut out = None;
        let mut profile = EngineProfile::JosPaper;
        let mut semantic_backend = SemanticBackendKind::Auto;
        let mut allow_backend_fallback = true;
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
                "--semantic-backend" => {
                    semantic_backend =
                        parse_semantic_backend(&next_string(&mut args, "--semantic-backend")?)?;
                }
                "--no-backend-fallback" => allow_backend_fallback = false,
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
            semantic_backend,
            allow_backend_fallback,
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

fn parse_semantic_backend(raw: &str) -> Result<SemanticBackendKind, String> {
    match raw {
        "auto" => Ok(SemanticBackendKind::Auto),
        "rule" | "rule-based" => Ok(SemanticBackendKind::RuleBased),
        "xelatex" | "xelatex-hook" => Ok(SemanticBackendKind::XeLaTeXHook),
        "luatex" | "lualatex" | "luatex-node" => Ok(SemanticBackendKind::LuaTeXNode),
        other => Err(format!(
            "unsupported semantic backend: {other}\n\n{}",
            usage()
        )),
    }
}

fn usage() -> String {
    "usage: cargo run -p doc-compiler-engine --example paper3_to_docx -- \\
  --project-root examples/paper3/latex \\
  --main-tex examples/paper3/latex/main-jos.tex \\
  --out examples/paper3/output/to-docx/paper3-compiler-engine.docx \\
  [--profile jos-paper] \\
  [--semantic-backend auto|rule-based|xelatex-hook|luatex-node] \\
  [--no-backend-fallback] \\
  [--no-standard-ast]"
        .to_string()
}
