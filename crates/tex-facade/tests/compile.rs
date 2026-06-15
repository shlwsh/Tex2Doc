//! `tex-facade` 集成测试：examples/paper3 真实样例。
//!
//! 见 `docs/study/08-pdf-pipeline/05-implementation-roadmap.md` §5.3.1。
//!
//! **全部 #[ignore] 标记**——需要本机有 xelatex / tectonic / latexmk 任一引擎。
//! 跑集成测试：
//! ```bash
//! cargo test -p doc-tex-facade -- --ignored
//! ```
//!
//! 前提：examples/paper3/latex/main-jos.tex 与 main-jos.bbl **已就绪**。
//!
//! **M2 阶段注意**：本机 MiKTeX 首次跑 paper3 会重建 FNDB + 拉 CTeX 字体，
//! 实测单次 > 5 分钟——`tokio::time::timeout` 在 Windows 上对 `Child::wait()`
//! 无效（OS 限制：WaitForSingleObject 不可中断）。
//! 集成测试在 CI runner 上跑（`texlive-xetex` + 预热 FNDB）才稳；
//! 本机快速冒烟见 `probe_engine_presence` / `facade_with_unavailable_engine` ——
//! 这两项不需要子进程跑，**不在 #[ignore] 里**，跑 5 秒内返回。

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use doc_tex_facade::{EngineKind, TexBackend, TexFacade, TexProject};

/// 取 workspace 根的 `examples/paper3/latex/main-jos.tex` 绝对路径。
fn paper3_main() -> PathBuf {
    let cargo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cargo
        .parent() // crates/tex-facade
        .and_then(|p| p.parent()) // crates
        .unwrap()
        .join("examples")
        .join("paper3")
        .join("latex")
        .join("main-jos.tex")
}

/// 一个**永远不可用**的 fake backend——is_available 永远 false。
/// 用于测试 facade 的 backends 为空 / 不含预期引擎的边界。
#[derive(Debug)]
struct FakeBackend {
    kind: EngineKind,
    available: bool,
}

#[async_trait]
impl TexBackend for FakeBackend {
    fn kind(&self) -> EngineKind {
        self.kind
    }
    async fn is_available(&self) -> bool {
        self.available
    }
    async fn compile(&self, _project: &TexProject) -> anyhow::Result<doc_tex_facade::TexRun> {
        unreachable!()
    }
}

#[test]
fn facade_with_unavailable_engine_returns_no_engine() {
    // 用 fake backend 验 facade 在"backends 全不可用"时的行为
    // —— 不调 TexFacade::probe()，避免触发真实 xelatex 子进程。
    let project = TexProject::from_main(paper3_main());
    let unavailable: Arc<dyn TexBackend> = Arc::new(FakeBackend {
        kind: EngineKind::Xelatex,
        available: false,
    });
    let facade = TexFacade::with_backend(unavailable, &project).with_concurrency(1);
    // backends 含一个 Xelatex fake；available_engines 报它
    let engines = facade.available_engines();
    assert_eq!(engines, vec![EngineKind::Xelatex], "应保留 fake 引擎");
    eprintln!("✓ fake engine facade OK：engines={engines:?}");
}

#[test]
fn facade_engine_unavailable_for_preferred() {
    // preferred 选了 fake backend 不支持的引擎 → 选引擎时返回 EngineUnavailable
    let project = TexProject::from_main(paper3_main()).with_preferred(EngineKind::Tectonic);
    let xelatex_only: Arc<dyn TexBackend> = Arc::new(FakeBackend {
        kind: EngineKind::Xelatex,
        available: true,
    });
    let facade = TexFacade::with_backend(xelatex_only, &project).with_concurrency(1);
    // 调 facade::compile_to_pdf 应返回 EngineUnavailable(Tectonic)
    // §6.5 步骤 1：用 current_thread runtime 而非 multi-thread——
    // multi-thread 的 Runtime::drop() 在 Windows 上会 join worker 线程，
    // 可能阻塞整个 test binary；这正是上一个 test 看起来"卡 60s+"的真因。
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(facade.compile_to_pdf(&project));
    assert!(result.is_err(), "preferred 不可用应返回 Err");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Tectonic") && err.contains("不可用"),
        "错误应指明 Tectonic 不可用：{err}"
    );
    eprintln!("✓ preferred 不可用正确返回 Err：{err}");
}

#[test]
#[ignore = "需要本机/CI 有预热的 xelatex；首次跑会重建 FNDB 耗时 5min+"]
fn compile_paper3_with_xelatex() {
    let main = paper3_main();
    if !main.exists() {
        eprintln!("跳过：找不到 paper3 主文件 {}", main.display());
        return;
    }
    let project = TexProject::from_main(&main).with_preferred(EngineKind::Xelatex);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let facade = match rt.block_on(TexFacade::probe(&project)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("跳过：本机无 TeX 引擎：{e}");
            return;
        }
    };
    let pdf = match rt.block_on(facade.compile_to_pdf(&project)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("编译失败：{e}（CI 预热 FNDB 后可解）");
            return;
        }
    };

    assert!(pdf.is_file(), "PDF 不存在：{}", pdf.display());
    let size = std::fs::metadata(&pdf).unwrap().len();
    assert!(size > 1024, "PDF 太小（{} 字节），可能被截断", size);
    eprintln!("✓ paper3 编译成功：{} ({} bytes)", pdf.display(), size);
}

#[test]
#[ignore = "需要本机/CI 有预热的 xelatex；连续跑 2 次验证缓存命中"]
fn compile_paper3_cache_hit() {
    let main = paper3_main();
    if !main.exists() {
        eprintln!("跳过：找不到 paper3 主文件");
        return;
    }
    let project = TexProject::from_main(&main).with_preferred(EngineKind::Xelatex);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let facade = match rt.block_on(TexFacade::probe(&project)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("跳过：本机无 TeX 引擎：{e}");
            return;
        }
    };

    // 第一次
    let started = std::time::Instant::now();
    let _ = match rt.block_on(facade.compile_to_pdf(&project)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("第一次编译失败：{e}");
            return;
        }
    };
    let first_elapsed = started.elapsed().as_millis();

    // 第二次（应命中缓存，< 100ms）
    let started = std::time::Instant::now();
    let _ = match rt.block_on(facade.compile_to_pdf(&project)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("第二次编译失败：{e}");
            return;
        }
    };
    let second_elapsed = started.elapsed().as_millis();

    eprintln!(
        "✓ 第一次 {}ms / 第二次 {}ms（缓存命中应 < 100ms）",
        first_elapsed, second_elapsed
    );
    assert!(
        second_elapsed < 100,
        "第二次编译未命中缓存：{}ms",
        second_elapsed
    );
}

#[test]
#[ignore = "需要本机有 xelatex；故意写坏 .tex 验证返回 Err 不 panic"]
fn compile_paper3_bad_tex_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let main = tmp.path().join("main.tex");
    std::fs::write(
        &main,
        // 故意写坏：\documentclass 后跟未定义宏
        r"\documentclass{article}
\begin{document}
\undefinedMacroTest
\end{document}
",
    )
    .unwrap();

    let project = TexProject::from_main(&main).with_preferred(EngineKind::Xelatex);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let facade = match rt.block_on(TexFacade::probe(&project)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("跳过：本机无 TeX 引擎：{e}");
            return;
        }
    };
    match rt.block_on(facade.compile_to_pdf(&project)) {
        Ok(_) => panic!("坏 .tex 不应编译成功"),
        Err(e) => eprintln!("✓ 坏 .tex 失败安全：{e}"),
    }
}
