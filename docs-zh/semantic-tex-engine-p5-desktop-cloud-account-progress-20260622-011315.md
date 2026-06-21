# Semantic TeX Engine P5 Desktop Cloud Account 开发进展报告

**时间戳**：20260622-011315  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P5 Slint Desktop MVP、P6 账号/订阅/用量 API  
**本轮目标**：将 Slint 桌面端从“本地转换 MVP”推进到“可接入商业 API 的账号与用量入口”，为后续云端上传/转换闭环铺路。

---

## 一、当前结论

本轮完成了 P5 与 P6 的第一段串联：

- 桌面端新增 `doc-commercial-api-client` 依赖。
- 桌面端新增 `cloud_account` 适配模块。
- Slint UI 新增 Account 区域：
  - API base URL
  - email
  - password
  - Login
  - Usage
  - account status
  - usage status
- 登录后会调用商业 API：
  - `POST auth/login`
  - `GET usage`
- 登录成功后会保存 access token、refresh token、user name、quota remaining 到 `AppState`。
- `Settings` 新增 `api_base_url` 和 `last_login_email`，不会保存密码和 token。
- `cargo check -p doc-desktop-slint` 通过。

当前 P5 仍未完成，因为云端上传、创建 conversion、轮询、下载 DOCX/report 尚未接入 UI。

---

## 二、实现内容

### 2.1 新增依赖

修改文件：

```text
crates/desktop-slint/Cargo.toml
```

新增：

```toml
doc-commercial-api-client = { path = "../commercial-api-client" }
url = "2"
tokio = { version = "1", features = ["net", "rt", "sync", "time"] }
```

说明：

- `doc-commercial-api-client` 提供商业 API SDK。
- `url` 用于构造可配置 API base URL。
- `tokio` 增加 `net/time`，保证 reqwest async client 在桌面端阻塞桥接中可正常运行。

### 2.2 新增 cloud account 适配层

新增文件：

```text
crates/desktop-slint/src/cloud_account.rs
```

关键类型：

```rust
CloudAccountSession
CloudAccountError
```

关键函数：

```rust
login_and_fetch_usage_blocking(base_url, email, password)
fetch_usage_blocking(base_url, access_token)
usage_line(usage)
```

设计说明：

- Slint callback 是同步入口，商业 API client 是 async。
- `cloud_account` 使用 current-thread tokio runtime 做阻塞式桥接。
- UI 层不直接感知 reqwest 和 API request/response 细节。
- 后续云端上传/转换可沿用同一 adapter 模式。

### 2.3 AppState 账号状态

修改文件：

```text
crates/desktop-slint/src/app_state.rs
```

变更：

- `auth_token` 从 `Option<String>` 改为 `RwLock<Option<String>>`。
- 新增 `refresh_token: RwLock<Option<String>>`。
- `user_name` 从 `Option<String>` 改为 `RwLock<Option<String>>`。
- `quota_remaining` 从 `Option<usize>` 改为 `RwLock<Option<usize>>`。

新增方法：

```rust
set_account_session(access_token, refresh_token, user_name, quota_remaining)
auth_token()
```

说明：

- 这让 background thread 可以安全更新账号状态。
- refresh token 已进入状态结构，为 P6 refresh token 轮换做准备。

### 2.4 Settings 持久化

修改文件：

```text
crates/desktop-slint/src/settings.rs
```

新增字段：

```rust
api_base_url: String
last_login_email: Option<String>
```

默认值：

```text
https://api.tex2doc.cn/v1/
```

说明：

- 只持久化 API base URL 和 email。
- 不持久化密码。
- 不持久化 token，后续应接入系统 keychain/credential store。

### 2.5 Slint UI 扩展

修改文件：

```text
crates/desktop-slint/src/ui/main.slint
```

新增属性：

```text
api-base-url
login-email
login-password
account-status
usage-status
```

新增 callback：

```text
login-clicked(string, string, string)
refresh-usage-clicked(string)
```

新增 Account 区域：

```text
API base URL
Email
Password
Login
Usage
Account status
Usage status
```

### 2.6 main.rs 接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增行为：

- 启动时从 `Settings` 恢复 API base URL 和 last login email。
- `Login` callback：
  - 保存 API base URL 和 email。
  - background thread 调用 `login_and_fetch_usage_blocking`。
  - 成功后更新 `AppState` 与 UI。
  - 失败后在 UI 显示错误。
- `Usage` callback：
  - 从 `AppState` 读取 access token。
  - background thread 调用 `fetch_usage_blocking`。
  - 未登录时提示先登录。

---

## 三、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint cloud_account -- --nocapture
cargo check -p doc-desktop-slint
```

结果：

```text
PASS
```

具体结果：

| 命令 | 结果 |
|---|---|
| `cargo fmt -p doc-desktop-slint` | PASS，仍输出项目历史 nightly rustfmt 配置 warning |
| `cargo test -p doc-desktop-slint cloud_account -- --nocapture` | PASS，1 test |
| `cargo check -p doc-desktop-slint` | PASS |

当前仍有 warning：

- `CommandError::OutputWriteFailed` / `ReportParseFailed` 尚未使用。
- `Settings::set_last_project` 尚未使用。
- `updater` 多数函数仍未接 UI。

这些 warning 与 P5/P9 后续工作未完成一致。

---

## 四、当前边界

本轮没有完成：

- 用户注册 UI。
- refresh token 自动续期。
- token 安全存储。
- 云端项目上传。
- 创建云端 conversion。
- 轮询 conversion 状态。
- 下载 DOCX/report。
- billing checkout/portal UI。

本轮没有修改：

- 旧 Rust rule-based 引擎。
- SemanticTexEngine 核心编译管线。
- server API 合约。
- P7 worker。

---

## 五、下一步计划

建议下一步继续 P5-P7 串联，按以下顺序实现：

1. 桌面端增加 Cloud Convert mode：
   - local convert
   - cloud convert
2. 实现 zip 打包或选择项目 zip：
   - 当前 `upload_project_zip` 需要 zip bytes。
   - 初期可要求用户选择 `.zip`，后续再自动打包目录。
3. 调用：
   - `upload_project_zip`
   - `create_conversion`
   - `get_conversion`
   - `download_conversion_docx`
   - `get_conversion_report`
4. 将 conversion status 写入 recent jobs。
5. 下载 DOCX 到 output path，report 写入相邻 JSON 文件。

完成上述内容后，P5 才能从“本地桌面 MVP + 账号入口”推进到“桌面商业闭环 MVP”。
