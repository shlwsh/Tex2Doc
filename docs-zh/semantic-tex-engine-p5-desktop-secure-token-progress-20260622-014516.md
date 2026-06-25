# Semantic TeX Engine P5 Desktop Secure Token 开发进展报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**时间戳**：20260622-014516  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P5 Slint Desktop MVP、P6 账号/订阅/用量 API  
**本轮目标**：补齐桌面端账号 session 的安全存储边界，避免 refresh token 写入 `settings.json`，让客户端重启后具备恢复登录的基础能力。

---

## 一、当前结论

本轮完成了 P5 桌面端 refresh token 安全存储 preview 能力：

- 新增 `credential_store` adapter。
- `settings.json` 继续只保存 API base URL、email、路径、profile、quality，不保存 token。
- 登录、注册、刷新成功后会尝试把 refresh token 写入平台安全存储。
- 启动时如果存在 `last_login_email`，会尝试加载 refresh token。
- 如果加载成功，UI 显示 `Stored session found. Click Refresh.`，用户可点击 `Refresh` 换取新的 access token 和 usage。
- 退出登录时会清理内存态 session，并尝试删除平台安全存储中的 refresh token。
- 如果平台安全存储不可用，不阻断登录，只提示 session 是内存态。

这让 P5 从：

```text
token 只存在内存，重启即丢
```

推进到：

```text
refresh token 不落 settings.json；
优先进入平台凭据存储；
重启后可通过 Refresh 恢复 session。
```

---

## 二、实现内容

### 2.1 新增 credential store adapter

新增文件：

```text
crates/desktop-slint/src/credential_store.rs
```

公开入口：

```rust
store_refresh_token(api_base_url, email, refresh_token)
load_refresh_token(api_base_url, email)
delete_refresh_token(api_base_url, email)
```

账号 key 策略：

```text
tex2doc-desktop-refresh-<sha256(api_base_url + email)>
```

说明：

- email 会做小写归一化。
- API base URL 参与 key，避免不同环境 token 混用。
- key 中不直接暴露 email。

### 2.2 平台实现策略

| 平台 | 当前实现 |
|---|---|
| macOS | `security add-generic-password/find-generic-password/delete-generic-password` |
| Linux | `secret-tool store/lookup/clear` |
| Windows | PowerShell `ConvertFrom-SecureString` + 当前用户 DPAPI 保护文件 |

边界说明：

- Linux 依赖系统安装 `secret-tool` 和可用 Secret Service。
- Windows preview 实现不是 Credential Manager，而是当前用户 DPAPI 保护文件；后续 Beta/GA 可替换为 Windows Credential Manager 或 `keyring` crate。
- 任一平台凭据存储失败不会中断登录，UI 会提示 `Session is memory-only`。

### 2.3 AppState 增强

修改文件：

```text
crates/desktop-slint/src/app_state.rs
```

新增方法：

```rust
set_refresh_token(refresh_token)
```

用途：

- 启动时从凭据存储加载 refresh token 后写入 `AppState`。
- 用户点击 `Refresh` 时可直接调用商业 API `auth/refresh`。

### 2.4 main.rs 接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增行为：

- 启动阶段：
  - 如果 settings 中存在 `last_login_email`，尝试加载 refresh token。
  - 成功后设置 UI：`Stored session found. Click Refresh.`
  - 失败后显示安全 token 加载失败原因。
- 登录/注册/刷新成功后：
  - 调用 `credential_store::store_refresh_token(...)`。
  - 成功时显示 `Session stored securely.`
  - 失败时显示 `Session is memory-only: ...`
- 退出登录：
  - 调用 `credential_store::delete_refresh_token(...)`。
  - 清理 `AppState` 内存态 token/user/quota。

---

## 三、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint credential_store -- --nocapture
cargo check -p doc-desktop-slint
cargo test -p doc-desktop-slint -- --nocapture
git diff --check
```

结果：

| 命令 | 结果 |
|---|---|
| `cargo fmt -p doc-desktop-slint` | PASS，仍有项目既有 nightly rustfmt 配置 warning |
| `cargo test -p doc-desktop-slint credential_store -- --nocapture` | PASS，2 tests |
| `cargo check -p doc-desktop-slint` | PASS |
| `cargo test -p doc-desktop-slint -- --nocapture` | PASS，13 tests |
| `git diff --check` | PASS |

新增测试：

```text
credential_store::tests::account_key_is_stable_for_same_account
credential_store::tests::account_key_rejects_missing_context
```

---

## 四、当前边界

本轮仍是 preview 级实现：

- Linux 环境如果没有 `secret-tool`，不会持久保存 refresh token。
- Windows 当前是 DPAPI 保护文件，不是 Credential Manager。
- 没有实现 token 过期时间、设备管理、refresh token 轮换历史。
- 服务端仍是 demo refresh token，不是生产 JWT/refresh-token hash。
- 没有 GUI 自动 Refresh；当前是启动后提示用户点击 Refresh。

---

## 五、下一步建议

P5 后续建议：

1. 增加 GUI 手工验收矩阵，覆盖 Linux 下登录、刷新、退出、重启恢复。
2. 增加拖拽 project/zip。
3. 接入 updater UI。
4. 如果允许新增依赖，评估 `keyring` crate 替换当前命令式 adapter。

P6 后续建议：

1. 服务端改为 JWT access token。
2. refresh token 只保存 hash，并支持轮换/撤销。
3. 增加设备维度 session 表。
4. 增加 usage event ledger。

P7 后续建议：

1. 把 cloud conversion 上传和产物迁移到对象存储。
2. 引入 sandbox runner 和任务资源限制。
