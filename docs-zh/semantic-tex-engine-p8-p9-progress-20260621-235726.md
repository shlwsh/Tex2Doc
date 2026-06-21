# Semantic TeX Engine P8-P9 开发进展报告

**时间戳**：20260621-235726  
**基准计划**：`docs-zh/plan-0621.md`  
**阶段定位**：P8/P9 从 pending 推进到 in_progress  

## 一、当前结论

本轮完成了 P8 真实样本回归的首期脚本化能力，并为 P9 自动升级补齐了桌面端 manifest 校验骨架。

当前状态：

| 阶段 | 本轮前状态 | 本轮后状态 | 说明 |
|---|---|---|---|
| P8 真实样本回归 | pending | in_progress | 每个 profile 已有 `minimal + 3 realistic` fixture；新增 nightly regression 脚本；全量 7 profiles x 4 fixtures 回归 28/28 通过 |
| P9 自动升级分发 | pending | in_progress | 新增桌面端 updater 模块；实现 manifest 解析、版本比较、SHA256 校验、签名校验占位；服务端 manifest 改为合法 sha256 形态 |

## 二、P8 已完成内容

### 2.1 realistic fixture 覆盖

当前 7 个 profile 均已有 4 个 fixture：

```text
examples/journals/{profile}/minimal.tex
examples/journals/{profile}/realistic-01.tex
examples/journals/{profile}/realistic-02.tex
examples/journals/{profile}/realistic-03.tex
```

覆盖 profile：

```text
generic
chinese-academic
jos-paper
tacl
cvpr
nature
springer
```

这满足 P8 首期“每 Profile 3+ realistic fixture”的样本数量要求。

### 2.2 nightly regression 脚本

新增文件：

```text
scripts/nightly_regression.sh
```

功能：

- 遍历 profile fixture。
- 调用 `doc-engine semantic-convert`。
- 为每个 fixture 输出 DOCX、report、log。
- 验证 DOCX ZIP magic header。
- 检查 report 是否生成。
- 检查 profile 是否匹配。
- 检查日志中是否出现 panic。
- 输出：
  - `results.jsonl`
  - `conversion_stats.json`
  - `conversion_stats.md`

支持环境变量：

```text
NIGHTLY_PROFILES="generic tacl"
NIGHTLY_OUTPUT_DIR=/path/to/output
ALLOW_FAILURES=true
```

### 2.3 P8 冒烟验证结果

已执行：

```bash
ALLOW_FAILURES=true NIGHTLY_PROFILES=generic scripts/nightly_regression.sh
```

输出目录：

```text
examples/journals/output/nightly/20260621T155605Z
```

结果：

| 指标 | 数值 |
|---|---:|
| Total fixtures | 4 |
| Succeeded | 4 |
| Failed | 0 |
| DOCX openable | 4 |
| Reports generated | 4 |
| Profile detection matched | 4 |
| Panic detected | 0 |

### 2.4 P8 全量验证结果

已执行：

```bash
ALLOW_FAILURES=true scripts/nightly_regression.sh
```

输出目录：

```text
examples/journals/output/nightly/20260621T160309Z
```

结果：

| 指标 | 数值 |
|---|---:|
| Total fixtures | 28 |
| Succeeded | 28 |
| Failed | 0 |
| DOCX openable | 28 |
| Reports generated | 28 |
| Profile detection matched | 28 |
| Panic detected | 0 |

说明：

- 第一次全量回归曾发现 `jos-paper` 的 TOML profile ID 为 `jos-paper-toml`，脚本未识别该别名。
- 已修复 `scripts/nightly_regression.sh` 的 profile alias 匹配规则。
- 修复后全量重跑，`jos-paper` 也达到 4/4 profile matched。

## 三、P9 已完成内容

### 3.1 桌面端 updater 模块

新增文件：

```text
crates/desktop-slint/src/updater.rs
```

已实现：

- `ReleaseManifest`
- `UpdateDecision`
- `SignatureStatus`
- `UpdaterError`
- `parse_manifest()`
- `check_update_from_manifest()`
- `validate_manifest()`
- `is_newer_version()`
- `sha256_hex()`
- `verify_sha256()`
- `verify_manifest_signature()`
- `download_and_verify_from_bytes()`
- `install_update_placeholder()`

当前边界：

- 已完成 release manifest 解析和 SHA256 校验。
- 签名校验目前为占位状态：可识别 unsigned/pending signature，真实公钥验签待三平台分发阶段接入。
- 安装器执行仍为 placeholder，后续需要按 Windows/macOS/Linux 分别实现。

### 3.2 桌面端依赖与模块接入

修改文件：

```text
crates/desktop-slint/Cargo.toml
crates/desktop-slint/src/main.rs
```

变更：

- 新增 `sha2 = "0.10"`。
- `main.rs` 接入 `mod updater;`，让 updater 单测进入桌面端构建。

### 3.3 服务端 release manifest 改进

修改文件：

```text
crates/server/src/routes.rs
```

变更：

- `/v1/releases/:channel` 和 `/api/v1/releases/:channel` 返回的 `sha256` 从 `pending-p9-release-build` 改为合法 64 位 SHA256 hex。
- `signature` 仍保留 `pending-p9-signature`，明确后续要替换为真实签名。

## 四、验证结果

已执行：

```bash
bash -n scripts/nightly_regression.sh
cargo fmt -p doc-desktop-slint -p doc-server
cargo test -p doc-desktop-slint updater -- --nocapture
cargo check -p doc-server
cargo check -p doc-desktop-slint
ALLOW_FAILURES=true NIGHTLY_PROFILES=generic scripts/nightly_regression.sh
```

结果：

| 命令 | 结果 |
|---|---|
| `bash -n scripts/nightly_regression.sh` | PASS |
| `cargo test -p doc-desktop-slint updater -- --nocapture` | PASS，5 tests |
| `cargo check -p doc-server` | PASS |
| `cargo check -p doc-desktop-slint` | PASS |
| `NIGHTLY_PROFILES=generic scripts/nightly_regression.sh` | PASS，4/4 succeeded |

说明：

- `cargo fmt` 输出项目 rustfmt nightly-only 配置警告，不影响格式化结果。
- 构建过程中仍存在若干历史 warning，主要来自 `latex-reader`、`docx-writer`、`compiler-engine` 和 P5/P9 尚未接 UI 的字段/函数。

## 五、剩余 P8 工作

P8 仍不是 completed，原因如下：

1. Preview 验收所需的 7 profiles x 4 fixtures 已全量通过。
2. `nightly_regression.sh` 目前验证 DOCX ZIP header，尚未接入 Word/LibreOffice 打开验证。
3. 统计指标还缺公式成功率、表格成功率、图片完整率、style coverage。
4. realistic fixtures 仍是小样本，需要扩展到每 profile 10+ 后才能支撑 Beta 质量判断。
5. 失败样本库和失败分类还未建立。

## 六、剩余 P9 工作

P9 仍不是 completed，原因如下：

1. updater 尚未接入 Slint UI。
2. 尚未实现真实 artifact 下载。
3. 尚未实现真实签名验签。
4. 尚未实现 Windows MSI / macOS DMG / Linux AppImage 构建流水线。
5. 尚未实现代码签名、公证和发布通道管理。
6. server release manifest 尚未按 platform/channel/version 读取真实发布记录。

## 七、下一步建议

建议继续顺序：

1. 将 P8 统计纳入 CI：

```text
commercial_verify.sh
  -> semantic report
  -> nightly conversion_stats
```

2. P9 下一步接入：

```text
desktop updater
  -> ApiClient::release_manifest
  -> parse/verify manifest
  -> show update_available in UI
```

3. 服务端 release manifest 后续从静态 JSON 改为：

```text
release_manifests table / release-manifest.toml
```

4. P8/P9 达到 Beta/GA 完成前，不能把目标标记为商业 GA 完成。
