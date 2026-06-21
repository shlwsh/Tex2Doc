# Semantic TeX Engine P5/P9 Desktop Update 开发进展报告

**时间戳**：20260622-015611  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P5 Slint Desktop MVP、P9 自动升级与三平台分发  
**本轮目标**：把已有 P9 release manifest 校验能力接入桌面端，使客户端具备可操作的“检查更新”入口，同时保持安装和签名发布仍为后续阶段。

---

## 一、当前结论

本轮完成了桌面端更新检查 preview 闭环：

```text
Slint Updates UI
  -> 输入/默认 release channel
  -> 调用 commercial API releases/{channel}
  -> 转换为 desktop updater manifest
  -> validate_manifest
  -> verify_manifest_signature placeholder
  -> 版本比较
  -> UI 展示 latest/current、sha256、签名状态和 release notes
```

当前 P9 从：

```text
只有 updater.rs manifest 校验工具和服务端 release manifest preview endpoint
```

推进到：

```text
桌面端已有可点击的 Check 更新入口，并能调用商业 API manifest 进行校验和结果展示
```

仍需强调：这不是完整自动升级。当前没有下载 artifact，没有执行安装器，也没有真实 Ed25519/minisign/codesign 签名校验。

---

## 二、实现内容

### 2.1 新增 desktop update adapter

新增文件：

```text
crates/desktop-slint/src/desktop_update.rs
```

公开类型：

```rust
DesktopUpdateCheck
DesktopUpdateError
```

公开函数：

```rust
check_update_blocking(base_url, channel, current_version)
update_status_line(check)
```

实现行为：

- 使用 `doc-commercial-api-client::ApiClient::release_manifest(channel)` 拉取 release manifest。
- 手动转换为 `crates/desktop-slint/src/updater.rs` 内的 `ReleaseManifest`。
- 调用 `updater::validate_manifest()` 检查 version/channel/download_url/sha256。
- 调用 `updater::verify_manifest_signature()` 获取当前 placeholder 签名状态。
- 调用 `updater::is_newer_version()` 判断是否有新版本。
- 输出 UI 友好的状态行。
- release channel 会写入 `Settings.release_channel`，下次启动继续使用上次选择。

channel 策略：

```text
空 channel -> stable
非空 channel -> 原样使用
```

### 2.2 Slint UI 接入

修改文件：

```text
crates/desktop-slint/src/ui/main.slint
```

新增属性：

```text
update-channel
update-status
```

新增 callback：

```text
check-update-clicked(string, string)
```

新增 UI 区块：

```text
Updates
  Release channel
  Check
  update-status
```

### 2.3 main.rs 回调接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增模块：

```rust
mod desktop_update;
```

新增初始化：

```rust
ui.set_update_channel("stable")
ui.set_update_status("--")
```

新增回调：

```rust
ui.on_check_update_clicked(...)
```

回调行为：

- 后台线程执行更新检查，避免阻塞 UI。
- 使用当前 `env!("CARGO_PKG_VERSION")` 作为本地版本。
- 成功后显示：
  - channel
  - current version
  - latest version
  - 是否有新版本
  - sha256
  - signature status
  - release notes
- 失败时显示明确错误。

---

## 三、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint desktop_update -- --nocapture
cargo check -p doc-desktop-slint
cargo test -p doc-desktop-slint -- --nocapture
git diff --check
```

结果：

| 命令 | 结果 |
|---|---|
| `cargo fmt -p doc-desktop-slint` | PASS，仍有项目既有 nightly rustfmt 配置 warning |
| `cargo test -p doc-desktop-slint desktop_update -- --nocapture` | PASS，2 tests |
| `cargo check -p doc-desktop-slint` | PASS |
| `cargo test -p doc-desktop-slint -- --nocapture` | PASS，15 tests |
| `git diff --check` | PASS |

新增测试：

```text
desktop_update::tests::empty_channel_defaults_to_stable
desktop_update::tests::update_status_mentions_current_and_latest_versions
```

GitNexus 检查：

- 对 `crates/desktop-slint/src/main.rs::main` 执行修改前影响分析：upstream impacted count = 0，风险 LOW。
- 对全量 dirty worktree 执行 `detect_changes(scope=all)`：风险 HIGH。
- HIGH 来源于当前 P5-P9 未提交工作区整体横跨 desktop、commercial API、server、worker、docs 等模块，不代表本次 updater UI 单点变更本身是高风险。

---

## 四、当前边界

本轮没有完成：

- 真实 artifact 下载。
- 下载内容 SHA256 校验。
- Ed25519/minisign/sigstore manifest 签名校验。
- Windows MSI/MSIX 安装执行。
- macOS DMG/pkg 安装执行与 notarization。
- Linux AppImage/deb/rpm 安装执行。
- 更新失败回滚。
- GUI 手工点击验收。

---

## 五、下一步建议

P9 下一步建议：

1. 为 `GET /v1/releases/:channel` 增加可配置 release manifest 源，而不是固定 preview JSON。
2. 新增 artifact 下载 endpoint 或使用对象存储 signed URL。
3. 接入 `download_and_verify_from_bytes` 的真实下载路径。
4. 增加 manifest 签名字段规范：
   - signing key id
   - algorithm
   - signature
   - signed payload canonicalization
5. 选择签名方案：
   - preview/Beta：minisign 或 Ed25519。
   - GA：平台代码签名 + manifest 签名双层校验。
6. 增加平台 installer runner：
   - Windows：MSI/MSIX。
   - macOS：DMG/pkg。
   - Linux：AppImage/deb/rpm。

P5 下一步建议：

1. 对 Updates UI 做一次真实 GUI 操作验收。
2. 继续补拖拽 project/zip。
3. 将 recent jobs 持久化。
4. 增加诊断包导出。

P6/P7 下一步建议：

1. release manifest 改为生产配置驱动。
2. artifact 转对象存储。
3. 为 release endpoint 增加缓存、签名和 channel 权限策略。
