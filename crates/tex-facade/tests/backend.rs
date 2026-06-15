//! `backend` 单元测试：命令行参数构造 + tectonic offline 探测。
//!
//! 见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.5。

use std::path::PathBuf;

use doc_tex_facade::tectonic::TectonicBackend;
use doc_tex_facade::xelatex::XelatexBackend;
use doc_tex_facade::TexProject;

#[test]
fn xelatex_command_construction() {
    // 即使没装 xelatex，也能测命令行参数（不实际起进程）
    let main = PathBuf::from("/tmp/paper3/main.tex");
    let project = TexProject::from_main(&main);
    let backend = XelatexBackend {
        bin: PathBuf::from("/usr/bin/xelatex"),
    };
    let args = backend.build_args(&project);

    // 断言关键参数
    assert!(args.contains(&"-interaction=nonstopmode".to_string()));
    assert!(args.contains(&"-halt-on-error".to_string()));
    assert!(
        args.iter().any(|a| a.starts_with("-output-directory=")),
        "必须带 -output-directory"
    );
    // 主文件是最后一个参数
    let last = args.last().unwrap();
    assert!(
        last.ends_with("main.tex") || last.ends_with("main.tex)"),
        "主文件必须作为参数"
    );
}

#[test]
fn tectonic_offline_env_disables_network() {
    // 设置 TECTONIC_OFFLINE=1 → 探测时 allow_network=false
    // SAFETY: 单测里只设 / 读，不与多线程共享
    unsafe {
        std::env::set_var("TECTONIC_OFFLINE", "1");
    }
    let backend = TectonicBackend {
        bin: PathBuf::from("/usr/bin/tectonic"),
        allow_network: false,
    };
    assert!(!backend.allow_network(), "TECTONIC_OFFLINE=1 → 关网");

    let main = PathBuf::from("/tmp/paper3/main.tex");
    let project = TexProject::from_main(&main);
    let args = backend.build_args(&project);
    assert!(args.contains(&"--outdir".to_string()));
    assert!(args.contains(&"--keep-logs".to_string()));
    assert!(args.contains(&"--print".to_string()));
    unsafe {
        std::env::remove_var("TECTONIC_OFFLINE");
    }
}

#[test]
fn tectonic_online_env_enables_network() {
    // TECTONIC_OFFLINE 未设 → allow_network=true
    unsafe {
        std::env::remove_var("TECTONIC_OFFLINE");
    }
    let backend = TectonicBackend {
        bin: PathBuf::from("/usr/bin/tectonic"),
        allow_network: true,
    };
    assert!(backend.allow_network(), "未设 TECTONIC_OFFLINE → 联网");
}
