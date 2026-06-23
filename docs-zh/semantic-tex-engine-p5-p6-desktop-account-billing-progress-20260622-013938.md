# Semantic TeX Engine P5/P6 Desktop Account Billing 开发进展报告

**时间戳**：20260622-013938  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P5 Slint Desktop MVP、P6 账号/订阅/用量 API  
**本轮目标**：在桌面端已有 Login/Usage 基础上，补齐注册、刷新登录、退出、套餐查询、checkout 和 billing portal 入口，继续推进商业客户端闭环。

---

## 一、当前结论

本轮完成了桌面端账号与账单入口的 preview 级接入：

- `Register`：调用商业 API `auth/register`，成功后拉取 usage。
- `Refresh`：使用内存态 refresh token 调用 `auth/refresh`，成功后更新 access token、refresh token 和 usage。
- `Logout`：清理桌面端内存态账号 session。
- `Plans`：调用 `plans` 并在 UI 中显示套餐摘要。
- `Checkout`：调用 `billing/checkout`，返回 URL 后尝试打开系统浏览器。
- `Portal`：调用 `billing/portal`，返回 URL 后尝试打开系统浏览器。

当前 P5/P6 状态从：

```text
Login + Usage
```

推进到：

```text
Register/Login/Refresh/Logout
  + Usage
  + Plans
  + Checkout
  + Billing Portal
```

这仍是 preview 级商业闭环：服务端 token 和 billing URL 仍是 demo/mock，token 也尚未接系统 keychain。

---

## 二、实现内容

### 2.1 AppState 账号状态增强

修改文件：

```text
crates/desktop-slint/src/app_state.rs
```

新增方法：

```rust
refresh_token()
clear_account_session()
```

用途：

- `refresh_token()` 供桌面端 `Refresh` 按钮调用 `auth/refresh`。
- `clear_account_session()` 供 `Logout` 清理 access token、refresh token、user name、quota remaining。

### 2.2 cloud_account API adapter 扩展

修改文件：

```text
crates/desktop-slint/src/cloud_account.rs
```

新增入口：

```rust
register_and_fetch_usage_blocking(base_url, email, password)
refresh_and_fetch_usage_blocking(base_url, refresh_token)
fetch_plans_blocking(base_url)
create_checkout_blocking(base_url, access_token, plan_id)
create_billing_portal_blocking(base_url, access_token)
plans_line(plans)
```

设计说明：

- 保持 Slint UI 同步 callback 与 async API client 的阻塞桥接模式一致。
- 注册、登录、刷新成功后统一转换为 `CloudAccountSession`。
- checkout/portal 返回 `BillingSession`，由 UI 层负责打开 URL 和显示结果。
- 未登录时 checkout/portal 返回明确错误，而不是静默失败。

### 2.3 Slint UI 扩展

修改文件：

```text
crates/desktop-slint/src/ui/main.slint
```

新增属性：

```text
billing-plan-id
billing-status
```

新增 callback：

```text
register-clicked(string, string, string)
refresh-login-clicked(string)
logout-clicked()
show-plans-clicked(string)
checkout-clicked(string, string)
billing-portal-clicked(string)
```

新增按钮：

- Account 区域：
  - `Register`
  - `Refresh`
  - `Logout`
- Billing 区域：
  - `Plans`
  - `Checkout`
  - `Portal`

### 2.4 main.rs 回调接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增行为：

- `on_register_clicked`
- `on_refresh_login_clicked`
- `on_logout_clicked`
- `on_show_plans_clicked`
- `on_checkout_clicked`
- `on_billing_portal_clicked`

新增 helper：

```rust
apply_account_session(app, ui, session)
open_external_url(url)
```

其中：

- `apply_account_session` 复用登录、注册、刷新成功后的 UI 更新与 `AppState` 更新。
- `open_external_url` 复用 Windows/macOS/Linux 系统命令打开 checkout/portal URL。

---

## 三、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint cloud_account -- --nocapture
cargo check -p doc-desktop-slint
cargo test -p doc-desktop-slint -- --nocapture
git diff --check
```

结果：

| 命令 | 结果 |
|---|---|
| `cargo fmt -p doc-desktop-slint` | PASS，仍有项目既有 nightly rustfmt 配置 warning |
| `cargo test -p doc-desktop-slint cloud_account -- --nocapture` | PASS，2 tests |
| `cargo check -p doc-desktop-slint` | PASS |
| `cargo test -p doc-desktop-slint -- --nocapture` | PASS，11 tests |
| `git diff --check` | PASS |

新增测试：

```text
cloud_account::tests::plans_line_formats_plan_summaries
```

---

## 四、当前边界

本轮没有把 P6 推到生产级，仍存在：

- 服务端 auth 仍是 demo token。
- refresh token 没有持久化、轮换、撤销。
- 桌面端 token 只在内存态，未接系统 keychain。
- checkout/portal URL 仍由 preview server 返回 mock billing 地址。
- 没有 Stripe 或等价支付 provider。
- 没有 webhook、幂等、订阅状态同步。
- 没有 usage event ledger。

---

## 五、下一步建议

P5 建议继续：

1. 加入 token keychain 存储，至少覆盖 macOS Keychain、Windows Credential Manager、Linux Secret Service 的适配方案。
2. 增加 GUI 真实操作验收脚本/清单。
3. 增加拖拽目录/zip。
4. 将 updater UI 接入主窗口。

P6 建议继续：

1. 新增 PostgreSQL schema 与 migration。
2. 用 Argon2id 替换 demo password 处理。
3. 用 JWT access token 替换 demo-access token。
4. 增加 refresh token hash 存储、轮换和撤销。
5. 引入 usage event ledger 和额度预占/返还机制。

P7 建议继续：

1. 把 uploads/jobs/docx/report 从内存迁移到数据库 + 对象存储。
2. 增加 sandbox runner，限制 CPU、内存、磁盘、进程数和运行时长。
