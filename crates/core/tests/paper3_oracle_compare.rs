//! V2 集成对比测试：调用 `scripts/compare_paper3.sh` 端到端对比。
//!
//! 运行方式：
//! ```text
//! cargo test -p doc-core --test paper3_oracle_compare -- --nocapture
//! ```
//!
//! 前置：
//! 1. 已 `cargo build --release -p doc-engine`
//! 2. `pdftotext` / `soffice` 在 PATH
//! 3. `examples/paper3/upload_full.zip` 存在
//! 4. `examples/paper3/output/main-jos-oracle.pdf` 存在
//!
//! 测试逻辑：
//! 1. 调用 bash 脚本生成 V2 docx → pdf
//! 2. 与 oracle PDF 做 pdftotext 字符级 + 关键 token + LaTeX 漏出对比
//! 3. 把脚本退出码作为 test 退出码（0=全通过）

use std::path::PathBuf;
use std::process::Command;

fn scripts_dir() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(root.pop(), "CARGO_MANIFEST_DIR 解析失败");
    assert!(root.pop(), "CARGO_MANIFEST_DIR 解析失败");
    root.push("scripts");
    root
}

fn paper3_paths() -> (PathBuf, PathBuf) {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(root.pop());
    assert!(root.pop());
    let zip = root.join("examples/paper3/upload_full.zip");
    let oracle_pdf = root.join("examples/paper3/output/main-jos-oracle.pdf");
    (zip, oracle_pdf)
}

fn precheck_or_skip() -> Option<String> {
    let script = scripts_dir().join("compare_paper3.sh");
    if !script.exists() {
        return Some(format!("脚本不存在：{}", script.display()));
    }
    let (zip, oracle_pdf) = paper3_paths();
    if !zip.exists() {
        return Some(format!("zip 不存在：{}", zip.display()));
    }
    if !oracle_pdf.exists() {
        return Some(format!("oracle PDF 不存在：{}", oracle_pdf.display()));
    }
    // 工具检查
    for tool in ["pdftotext", "soffice", "bash"] {
        if Command::new("which")
            .arg(tool)
            .output()
            .map(|o| !o.status.success())
            .unwrap_or(true)
        {
            return Some(format!("工具缺失：{tool}"));
        }
    }
    // doc-engine 可执行
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    let doc_engine = root.join("target/release/doc-engine");
    if !doc_engine.exists() {
        return Some(format!("doc-engine 未构建：{}", doc_engine.display()));
    }
    None
}

#[test]
fn paper3_v2_vs_oracle() {
    if let Some(reason) = precheck_or_skip() {
        eprintln!("⏭ 跳过 paper3_v2_vs_oracle：{reason}");
        return;
    }

    let script = scripts_dir().join("compare_paper3.sh");
    eprintln!("▶ 运行 {}", script.display());

    let output = Command::new("bash")
        .arg(&script)
        .arg("--no-cargo")
        .output()
        .expect("运行 compare_paper3.sh 失败");

    // 把脚本输出原样打出
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    let code = output.status.code().unwrap_or(2);
    assert_eq!(
        code, 0,
        "compare_paper3.sh 失败（exit={code}）：脚本输出见上方"
    );
}
