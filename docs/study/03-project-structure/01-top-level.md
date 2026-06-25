# 第三章 · 工程目录详解

> 本章是项目最权威的「地图」。每个目录都给出：作用、关键文件、注意事项。

---

## 1. 仓库根目录

```
E:\work\Tex2Doc\
├── .agent/                      # Cursor Agent 技能源
│   └── skills/
│       ├── README.md
│       ├── makeskill/           # 创建/规范项目技能
│       ├── mygit/               # git 提交脚本与模板
│       └── scholar-search/      # 文献搜索技能
├── .claude/                     # Claude / GitNexus 技能（镜像）
│   └── skills/gitnexus/...      # gitnexus 全套
├── .cursor/                     # Cursor IDE 配置
│   ├── rules/                   # 项目级规则
│   │   ├── project-language.mdc
│   │   └── project-skills.mdc
│   └── skills/                  # 符号链接到 .agent/skills
├── .githooks/                   # Git 钩子
│   └── post-commit              # 自动 push
├── .github/                     # GitHub 配置
│   └── workflows/
│       └── ci.yml               # Rust CI（ubuntu/windows/macos）
├── .gitnexus/                   # GitNexus 索引（自动生成）
│   ├── run.cjs                  # 索引运行器
│   └── ...
├── crates/                      # Rust workspace（15 个 crate）
├── docs/                        # 项目文档（已有）
│   ├── *.md                     # 技术方案 / 任务清单 / 进展报告
│   └── study/                   # 【本目录】学习文档
├── docs-zh/                     # 中文专题方案，如 Semantic TeX Engine 实现方案
├── examples/                    # 示例项目 + 端到端夹具
│   └── paper3/                  # 主要示例（8 千行 LaTeX）
├── extension/                   # Chrome MV3 扩展
├── flutter_app/                 # Flutter 多端工程
├── node_modules/                # npm 依赖（.gitignore）
├── scripts/                     # Bash + Python + Node + PowerShell 脚本
├── target/                      # Cargo 编译产物（.gitignore）
├── tests/                       # 跨 crate 共享夹具
│   └── fixtures/
├── .env copy.mygit              # 模板
├── .env.mygit                   # 模板
├── .gitignore
├── .rustfmt.toml                # 格式化配置
├── AGENTS.md                    # GitNexus Agent 规则
├── Cargo.lock                   # 锁文件（已入仓）
├── Cargo.toml                   # Workspace 顶层配置
├── CLAUDE.md                    # GitNexus Claude 规则
├── clippy.toml                  # clippy 策略
├── deny.toml                    # cargo-deny 许可证白名单
├── package-lock.json            # npm 锁文件
├── package.json                 # npm 顶层
├── README.md                    # 项目自述
└── rust-toolchain.toml          # 工具链固定
```

---

## 2. 关键根文件说明

### 2.1 `Cargo.toml`（workspace 顶层）

* 已在 [02-tech-stack/01-rust-stack.md](../02-tech-stack/01-rust-stack.md) 详述。

### 2.2 `package.json`

* 已在 [02-tech-stack/03-web-extension-stack.md](../02-tech-stack/03-web-extension-stack.md) 详述。

### 2.3 `rust-toolchain.toml`

* 固定 `stable` channel + `rustfmt` / `clippy` / `rust-src` 三个组件。
* `rust-src` 供 `rust-analyzer` 在 IDE 中跳转源码。

### 2.4 `.gitignore`

主要规则：

* Rust：`/target/` / `*.rs.bk` / `Cargo.lock.bak`
* Flutter：`build/` / `.dart_tool/` / `flutter_*.png` / `linked_*.ds` / `unlinked.ds` / `unlinked_spec.ds` / `flutter_app/.flutter-plugins` / `flutter_app/.flutter-plugins-dependencies` / `flutter_app/pubspec.lock`
* Node：`node_modules/` / `dist/` / `wasm/pkg/` / `.cache/`
* 编辑器：`.idea/` / `.vscode/` / `.DS_Store` / `Thumbs.db`
* 测试：`tests/output/` / `tests/snapshots/*-new*` / `tests/snapshots/*-pending-snap` / `*.docx`（除 `crates/docx-writer/tests/fixtures/*.docx`）
* LaTeX 编译产物：`*.aux` / `*.log` / `*.bbl` / `*.blg` / `*.out` / `*.synctex.gz` / `*.pdf`
* examples 输出：`examples/**/output/` / `examples/**/latex/output/`

### 2.5 `AGENTS.md` / `CLAUDE.md`

* 内容一致：GitNexus 索引说明 + 强制规则（MUST run `impact` before edit；MUST run `detect_changes` before commit）。
* 由 `gitnexus start/end` 注释包裹，**不要手工修改**该标记区段。

### 2.6 `README.md`

* 项目门面：技术栈 + 仓库结构 + 构建/测试命令 + Git 流程。

---

## 3. `crates/` —— Rust Workspace

```
crates/
├── core/                        # doc-core：FFI/WASM 统一门面
├── compiler-engine/             # doc-compiler-engine：Semantic TeX Engine facade
├── utils/                       # doc-utils：通用工具库
├── semantic-ast/                # doc-semantic-ast：核心语义块模型
├── latex-reader/                # doc-latex-reader：LaTeX 解析器
├── docx-writer/                 # doc-docx-writer：OOXML 序列化
├── bib/                         # doc-bib：BibLaTeX 解析
├── mathml/                      # doc-mathml：公式管道
├── wasm/                        # doc-wasm：WASM 桥接
├── native/                      # doc-native：原生 cdylib
├── server/                      # doc-server：HTTP 服务
├── tex-facade/                  # doc-tex-facade：TeX oracle 封装
├── docx-pdf/                    # doc-docx-pdf：DOCX -> PDF
├── quality/                     # doc-quality：结构/文本/视觉质量对比
└── cli/                         # doc-engine：统一 CLI
```

> 详细到文件级的说明见 [02-rust-crates.md](./02-rust-crates.md)。

---

## 4. `flutter_app/` —— Flutter 多端工程

```
flutter_app/
├── .metadata                    # Flutter 元信息
├── .flutter-plugins             # （运行时生成，.gitignore）
├── .flutter-plugins-dependencies
├── .idea/                       # IDEA 工程文件
│   ├── modules.xml
│   ├── workspace.xml
│   └── libraries/
│       ├── Dart_SDK.xml
│       └── KotlinJavaRuntime.xml
├── analysis_options.yaml        # 静态分析规则
├── doc_engine.iml               # IDEA 模块文件
├── pubspec.yaml                 # Dart 依赖
├── pubspec.lock                 # Dart 锁文件（已入仓）
├── README.md                    # Flutter 子工程说明
│
├── bin/
│   └── native_smoke.dart        # 桌面端端到端冒烟
│
├── lib/                         # Dart 源代码
│   ├── main.dart                # 入口
│   ├── workspace_app.dart       # 共享 UI
│   ├── bridge.dart              # 条件 import 聚合
│   ├── bridge_stub.dart         # 桌面端桥接
│   ├── bridge_web.dart          # Web 端桥接
│   ├── native_bridge.dart       # FFI 实现
│   └── wasm_bridge.dart         # JS interop 实现
│
├── test/                        # widget / 桥接测试
│   ├── widget_test.dart
│   └── bridge_smoke_test.dart
│
├── wasm/                        # WASM 产物（由 wasm-pack 生成）
│   └── pkg/
│       ├── doc_engine.js
│       ├── doc_engine.d.ts
│       ├── doc_engine_bg.wasm
│       ├── doc_engine_bg.wasm.d.ts
│       └── package.json
│
├── web/                         # Web 入口与资源
│   ├── favicon.png
│   ├── index.html
│   ├── manifest.json
│   ├── icons/
│   │   ├── Icon-192.png
│   │   ├── Icon-512.png
│   │   ├── Icon-maskable-192.png
│   │   └── Icon-maskable-512.png
│   └── wasm/
│       ├── doc_engine.js        # 与 wasm/pkg/ 内容一致
│       └── doc_engine_bg.wasm
│
├── windows/                     # Windows 桌面端
│   ├── CMakeLists.txt           # 关键：自动调 cargo build
│   ├── flutter/
│   │   ├── CMakeLists.txt
│   │   ├── generated_plugins.cmake
│   │   ├── generated_plugin_registrant.cc
│   │   ├── generated_plugin_registrant.h
│   │   └── ephemeral/           # Flutter 引擎 dll + ICU（gitignore 候选）
│   └── runner/
│       ├── CMakeLists.txt
│       ├── Runner.rc            # Windows 资源
│       ├── flutter_window.cpp
│       ├── flutter_window.h
│       ├── main.cpp
│       ├── resource.h
│       ├── runner.exe.manifest
│       ├── utils.cpp
│       ├── utils.h
│       ├── win32_window.cpp
│       ├── win32_window.h
│       └── resources/
│           └── app_icon.ico
│
└── build/                       # （编译产物，.gitignore）
    ├── .last_build_id
    ├── flutter_assets/          # Web 编译产物的源
    ├── web/                     # `flutter build web` 输出
    │   ├── index.html
    │   ├── main.dart.js
    │   ├── manifest.json
    │   ├── canvaskit/           # CanvasKit 引擎
    │   ├── icons/
    │   ├── assets/
    │   ├── fonts/
    │   ├── shaders/
    │   └── wasm/                # （与 web/wasm 一致）
    ├── native_assets/windows/
    ├── test_cache/
    ├── unit_test_assets/
    └── windows/x64/             # CMake 编译产物
```

> Flutter 工程的 [详细说明](./03-flutter-app.md)。

---

## 5. `extension/` —— Chrome MV3 扩展

```
extension/
├── manifest.json                # MV3 清单
├── background.js                # Service Worker
├── README.md                    # 扩展说明
│
├── content/
│   └── content.js               # Overleaf/arXiv 选区监听
│
├── popup/
│   ├── popup.html
│   ├── popup.css
│   ├── popup.js                 # WASM 加载 + 文件选择 + 转换
│   └── wasm/
│       ├── doc_engine.js
│       └── doc_engine_bg.wasm
│
└── icons/
    ├── icon16.png
    ├── icon48.png
    └── icon128.png
```

> 详细说明见 [04-extension-scripts-tests.md](./04-extension-scripts-tests.md)。

---

## 6. `examples/` —— 示例项目

```
examples/
└── paper3/                      # 主示例（完整学术论文）
    ├── upload.zip               # 由 build_paper3_zip.mjs 生成
    └── latex/
        ├── .latexmkrc           # latexmk 配置
        ├── rjthesis.cls         # 期刊 class 文件
        ├── main-jos.tex         # 主源（英）—— e2e 入口
        ├── main-zh.tex          # 主源（中）
        ├── references.bib       # BibTeX 数据库（19 KB）
        ├── chk.tex / chk.aux / chk.log / chk.pdf   # 校验用
        ├── main-jos.{aux,bbl,blg,log,out,pdf}      # LaTeX 编译产物（gitignore）
        ├── main-zh.{aux,bbl,blg,log,out,pdf}       # 同上
        ├── test_spacing.{aux,log,pdf}              # 间距测试产物
        └── sections/zh/         # 各章节子文件
            ├── 00_abstract.tex
            ├── 01_intro.tex
            ├── 02_related.tex
            ├── 03_system.tex
            ├── 04_algorithms.tex
            ├── 05_implementation.tex
            ├── 06_experiments.tex
            └── 07_conclusion.tex
```

* `upload.zip` 由 `node scripts/build_paper3_zip.mjs` 生成，包含 latex/ 全部内容。
* `*.aux` / `*.log` / `*.pdf` 等 LaTeX 编译产物**部分已入仓**（被 `.gitignore` 规则覆盖），但历史中提交过若干；CI 时会被重新生成。

---

## 7. `scripts/` —— 工具脚本

```
scripts/
├── build_paper3_zip.mjs         # 把 examples/paper3/latex → upload.zip
├── commit_push.ps1              # 自动 commit + push
├── e2e_extension.mjs            # Playwright 验证 Chrome 扩展
├── e2e_paper3.mjs               # Playwright 验证 Flutter Web
├── e2e_server.mjs               # 验证 doc-server HTTP
├── install_commit_push_hook.ps1 # 启用 post-commit 钩子
├── link_cursor_skills.sh        # 链接 .cursor/skills 到 .agent/skills
├── mygit.ps1 / mygit.sh / mygit.py  # 通用 mygit 工具
├── serve_flutter_web.mjs        # 静态服务器（端口 2627）
├── test_proxy.py                # Python 测试代理
├── verify_install.mjs           # 环境自检
├── verify_paper3.mjs            # 旧版 verify（保留）
└── verify_paper3.ps1            # PowerShell 版 verify
```

---

## 8. `tests/` —— 跨 crate 共享夹具

```
tests/
└── fixtures/
    └── ieee/
        ├── ieee_simple.tex      # 简单 IEEE 模板
        └── ieee_nested.tex      # 嵌套结构（itemize/enumerate/表格/公式）
```

* 与 `crates/*/tests/fixtures/` 各自 crate 的本地夹具不同；本目录是 workspace 级别共享。
* 当前主要由 `crates/core/tests/end_to_end.rs` / `ieee_fixtures.rs` / `paper3_e2e.rs` 使用。

---

## 9. `docs/` —— 项目文档

```
docs/
├── study/                       # 【本目录】
│   ├── README.md                # 学习文档索引
│   ├── 01-overview/             # 项目概览
│   ├── 02-tech-stack/           # 技术栈
│   ├── 03-project-structure/    # 工程目录（本目录父级）
│   ├── 04-architecture/         # 技术架构
│   ├── 05-key-tech/             # 关键技术
│   ├── 06-user-guide/           # 使用说明
│   └── 07-deployment/           # 部署手册
│
├── Doc-engine_LaTeX-to-DOCX_技术方案_v2.0_20260614.md
├── Doc-engine_LaTeX-to-DOCX_任务清单_v2.0_20260614.md
├── Doc-engine_后期开发进展报告_v1.1_20260614.md
├── Doc-engine_后期开发进展报告_v1.2_20260614.md
├── Doc-engine_后期开发进展报告_v1.3_20260614.md
├── Doc-engine_任务清单完成度补丁_v1.0_20260614.md
├── Doc-engine_任务清单完成度补丁_v1.3_20260614.md
├── Doc-engine_V1.3_计划与实施归档_20260614.md
└── Doc-engine：LaTeX → DOCX 纯 Rust 核心 + Flutter 全平台转换工具完整技术实现方案（V1）.md
```

---

## 10. 工具与配置目录

### 10.1 `.agent/`

Cursor Agent 技能源目录。

* `makeskill/`：创建/规范项目技能的 SKILL + examples + resources。
* `mygit/`：git 提交脚本与模板。
* `scholar-search/`：文献搜索（PDF / BibTeX 下载）。
* 通过 `scripts/link_cursor_skills.sh` 链接到 `.cursor/skills/`。

### 10.2 `.cursor/`

* `rules/project-language.mdc`：项目语言规则（中文回复等）。
* `rules/project-skills.mdc`：技能加载规则（本仓库强制要求）。
* `skills/{makeskill,mygit,scholar-search}/`：符号链接到 `.agent/skills/...`。

### 10.3 `.claude/`

* `.claude/skills/gitnexus/...`：gitnexus 全套技能（explore / impact / refactor / debugging / cli / guide / pr-review）。

### 10.4 `.githooks/`

* `post-commit`：自动 push（已 [01-overview/02-quick-tour.md](../01-overview/02-quick-tour.md) 详述）。

### 10.5 `.github/`

* `workflows/ci.yml`：Rust CI 三平台矩阵。

### 10.6 `.gitnexus/`

* 索引数据 + 运行器。每次 `cargo build` 后自动重建。

---

## 11. 不入仓目录速查

| 目录 | 入仓？ | 说明 |
|------|:------:|------|
| `target/` | ❌ | Cargo 编译产物 |
| `node_modules/` | ❌ | npm 依赖 |
| `flutter_app/.dart_tool/` | ❌ | Dart 工具缓存 |
| `flutter_app/build/` | ❌ | Flutter 编译产物 |
| `flutter_app/.flutter-plugins*` | ❌ | 插件元信息 |
| `flutter_app/pubspec.lock` | ⚠️ 视情况 | 当前**已入仓**（应用项目惯例） |
| `flutter_app/windows/flutter/ephemeral/` | ❌ | Flutter 引擎 dll |
| `examples/*/output/` | ❌ | e2e 验证产物（脚本会重建） |
| `examples/*/latex/*.aux` 等 | ⚠️ 部分入仓 | 由 `.gitignore` 决定 |
| `tests/output/` | ❌ | 测试产物 |
| `tests/snapshots/*-new*` | ❌ | insta 待审快照 |
| `*.docx` | ❌ | docx（除 `docx-writer/tests/fixtures/`） |

---

## 12. 进一步阅读

* [02-rust-crates.md](./02-rust-crates.md) — `crates/` 详尽到文件级
* [03-flutter-app.md](./03-flutter-app.md) — Flutter 工程详尽
* [04-extension-scripts-tests.md](./04-extension-scripts-tests.md) — 扩展 / 脚本 / 测试
