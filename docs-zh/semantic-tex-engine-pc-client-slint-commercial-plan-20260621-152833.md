# Semantic TeX Engine PC 客户端商业化技术方案（Slint/Rust）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**方案版本**：20260621-152833  
**目标**：为 Tex2Doc / Semantic TeX Engine 新增跨平台 PC 客户端入口，辅助商业化发布  
**技术栈建议**：Rust + Slint  
**目标平台**：Windows、macOS、Linux  
**适用范围**：新增独立桌面客户端，不替换现有 Flutter/Web/Extension 路径  
**状态**：设计方案，待审核后开发  

---

## 一、结论

建议为当前项目新增一个基于 **Rust + Slint** 的 PC 端客户端应用入口。

该客户端应定位为：

```text
Tex2Doc Desktop
面向科研作者、实验室、期刊编辑和企业用户的本地/云端混合 TeX→DOCX 转换工具
```

它承担以下商业化职责：

1. 提供本地文件转换入口。
2. 提供云端高质量转换入口。
3. 支持账号注册、登录和设备授权。
4. 支持套餐订阅和客户门户跳转。
5. 支持用量查看、额度同步和离线宽限。
6. 支持自动升级。
7. 支持转换报告、失败诊断和质量门禁展示。
8. 支持 Windows/macOS/Linux 跨平台分发。

Slint 的可行性判断：

| 维度 | 结论 |
|---|---|
| Rust 原生集成 | 适合，避免 Flutter/Dart FFI 复杂度 |
| 跨平台桌面 | 适合，覆盖 Windows/macOS/Linux |
| 商业客户端 UI | 适合，适合工具型桌面应用 |
| 自动升级 | Slint 不内置，需要自建 updater |
| 支付订阅 | Slint 不内置，应走服务端 + 浏览器 Checkout |
| 账号系统 | Slint 不内置，应走商业 API |
| 大型复杂 UI 生态 | 弱于 Electron/Flutter，但当前产品足够 |
| 授权 | 需审查 Slint 官方授权，商业发布建议采购商业许可或满足其授权/署名要求 |

总体建议：

```text
短期：新增 Slint Desktop Preview，承载本地转换与登录
中期：接入订阅、用量、自动升级和云端转换
长期：成为商业桌面端主入口，Flutter 路径保留为 Web/移动/历史兼容
```

---

## 二、与现有工程的关系

当前仓库已有：

- `crates/compiler-engine`：新语义转换引擎。
- `crates/core`：旧 Rust doc 转换核心。
- `crates/native`：面向 Flutter 桌面的 FFI cdylib。
- `crates/server`：MVP HTTP server。
- `crates/cli`：命令行入口。
- `flutter_app`：已有 Flutter Web/Desktop 应用。
- `extension`：Chrome 扩展。

新增 Slint 客户端的原则：

1. 不删除、不替换现有 Flutter 应用。
2. 不反向绑定旧 Rust doc 转换路径。
3. 新客户端优先调用 `doc-compiler-engine` 新语义引擎。
4. 旧 `doc-core` 可作为本地快速转换 fallback。
5. 商业能力通过独立 API client 接入，不把计费逻辑写进转换引擎。

建议新增：

```text
crates/
├── desktop-slint/          # Slint PC 客户端
├── commercial-api-client/  # 商业 API Rust client
├── license-client/         # 授权、设备绑定、离线令牌
└── updater/                # 自动升级抽象，可后续独立
```

也可以采用应用目录形式：

```text
apps/
└── desktop-slint/
```

考虑当前 workspace 已以 `crates/*` 为主，首期建议放在：

```text
crates/desktop-slint
```

---

## 三、Slint 技术可行性评估

### 3.1 适合点

Slint 对当前项目的优势：

1. **Rust 原生**：UI 与转换引擎同语言，避免 Dart FFI、C ABI、内存释放边界。
2. **跨平台桌面**：适合 Windows/macOS/Linux 工具型应用。
3. **轻量**：相对 Electron 更小，内存占用更可控。
4. **声明式 UI**：`.slint` 文件适合构建稳定工具界面。
5. **商业工具体验**：适合转换器、仪表盘、设置页、日志页、订阅页。
6. **本地能力强**：可直接调用 Rust crate、文件系统、keychain、updater、zip、TeX runtime。

### 3.2 不足点

| 问题 | 影响 | 处理 |
|---|---|---|
| 生态小于 Flutter/Electron | 复杂控件和插件少 | 首期保持工具型 UI，不做重交互富媒体 |
| 不内置账号/订阅 | 需要商业 API | 通过 `commercial-api-client` 调用服务端 |
| 不内置自动更新 | 需要 updater 模块 | 自建 manifest + 签名校验 |
| 移动端不是主目标 | 不适合作移动端统一方案 | 移动端继续走 Flutter 或 Web |
| 授权需审查 | 商业发布风险 | 发布前确认 Slint license/commercial plan |

### 3.3 与 Flutter/Electron/Tauri 对比

| 技术 | 优点 | 缺点 | 本项目建议 |
|---|---|---|---|
| Slint | Rust 原生、轻量、跨平台 | 生态较小 | 推荐作为 PC 客户端 |
| Flutter | 已有工程、多端统一 | Rust FFI 边界复杂、包体较大 | 保留，不作为新商业桌面主线 |
| Electron | 生态最大、Web UI 快 | 包体和内存大、Rust 调用需桥接 | 不推荐首选 |
| Tauri | Web UI + Rust 后端 | 仍需前端栈 | 可作为备选，不优先 |
| egui | Rust 原生、快 | 商业 UI 质感弱 | 可用于内部工具，不适合正式客户端 |

结论：

```text
如果目标是商业 PC 客户端，并且希望最大化 Rust 代码复用，Slint 是合理选择。
```

---

## 四、客户端产品定位

### 4.1 用户角色

| 用户 | 需求 |
|---|---|
| 免费用户 | 少量转换、查看兼容性、体验效果 |
| Pro 用户 | 批量转换、更多期刊 Profile、云端高质量转换 |
| Team 用户 | 团队额度、批量任务、历史记录、共享 Profile |
| Enterprise 用户 | 私有化、离线授权、自定义模板、安全审计 |

### 4.2 客户端工作模式

客户端应支持三种模式：

| 模式 | 描述 | 适用 |
|---|---|---|
| 本地快速模式 | 调用本地 Rust 引擎，不上传文件 | 隐私敏感、免费预览 |
| 云端标准模式 | 上传项目到 SaaS worker，生成高质量 DOCX | Pro/Team |
| 企业离线模式 | 本地 license + 本地 TeX runtime + 本地质量验证 | Enterprise |

### 4.3 首期核心功能

1. 拖拽上传 `.tex`、`.zip` 或项目目录。
2. 自动识别主 TeX 文件。
3. 自动检测 Profile。
4. 展示兼容性报告。
5. 选择转换模式：本地 / 云端。
6. 生成 DOCX。
7. 展示转换进度和日志。
8. 展示用量、套餐、剩余额度。
9. 登录/注册/退出。
10. 打开订阅管理页面。
11. 自动检查更新。
12. 下载转换报告。

---

## 五、客户端信息架构

### 5.1 页面结构

```text
MainWindow
├── Sidebar
│   ├── Convert
│   ├── Jobs
│   ├── Reports
│   ├── Usage
│   ├── Subscription
│   └── Settings
├── Header
│   ├── Account status
│   ├── Plan badge
│   └── Update indicator
└── Content
```

### 5.2 页面说明

| 页面 | 功能 |
|---|---|
| Convert | 文件选择、Profile 检测、转换按钮、模式选择 |
| Jobs | 当前任务、历史任务、重试、打开输出目录 |
| Reports | 兼容性报告、质量报告、错误诊断 |
| Usage | 本月额度、已用次数、剩余次数、重置时间 |
| Subscription | 当前套餐、升级、打开客户门户 |
| Settings | 输出目录、隐私设置、runtime 路径、自动更新、日志 |

### 5.3 首屏 Convert 体验

首屏不做营销页，直接是可用工具：

```text
拖拽论文项目到此处
或选择 .zip / .tex / 文件夹

检测结果：
Profile: tacl
Compatibility: 88 / 100
Backend: Auto -> LuaTeXNode

[Convert Locally] [Convert in Cloud]
```

---

## 六、工程结构设计

### 6.1 新 crate：`desktop-slint`

建议结构：

```text
crates/desktop-slint/
├── Cargo.toml
├── build.rs
├── ui/
│   ├── app.slint
│   ├── components/
│   │   ├── sidebar.slint
│   │   ├── convert_panel.slint
│   │   ├── job_list.slint
│   │   ├── usage_meter.slint
│   │   ├── report_view.slint
│   │   └── settings_panel.slint
│   └── theme.slint
└── src/
    ├── main.rs
    ├── app_state.rs
    ├── commands.rs
    ├── models.rs
    ├── local_convert.rs
    ├── cloud_convert.rs
    ├── auth.rs
    ├── billing.rs
    ├── usage.rs
    ├── updater.rs
    ├── storage.rs
    ├── runtime.rs
    └── error.rs
```

### 6.2 Cargo 依赖建议

```toml
[dependencies]
slint = "1"
doc-compiler-engine = { workspace = true }
doc-core = { workspace = true }
doc-utils = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "multipart"] }
directories = "5"
keyring = "3"
open = "5"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { workspace = true }
tracing = { workspace = true }
```

```toml
[build-dependencies]
slint-build = "1"
```

说明：

- `slint`：UI runtime。
- `slint-build`：编译 `.slint` UI 文件。
- `keyring`：保存 refresh token / device token。
- `directories`：跨平台应用数据目录。
- `open`：打开浏览器订阅页或输出目录。
- `reqwest`：商业 API。
- `doc-compiler-engine`：新语义引擎本地转换。
- `doc-core`：旧路径 fallback。

### 6.3 UI 与 Rust 绑定

Slint UI：

```slint
export component AppWindow inherits Window {
    in-out property <string> account_email;
    in-out property <string> plan_name;
    in-out property <int> usage_used;
    in-out property <int> usage_limit;

    callback select_project();
    callback convert_local();
    callback convert_cloud();
    callback login();
    callback open_subscription_portal();
    callback check_update();
}
```

Rust：

```rust
slint::include_modules!();

fn main() -> anyhow::Result<()> {
    let app = AppWindow::new()?;
    let state = AppState::load()?;

    app.on_select_project({
        let app = app.as_weak();
        move || {
            // open file dialog, detect project
        }
    });

    app.on_convert_local({
        let app = app.as_weak();
        move || {
            // spawn background conversion
        }
    });

    app.run()?;
    Ok(())
}
```

### 6.4 后台任务模型

UI 线程不能阻塞。转换任务必须异步：

```rust
pub enum DesktopJobStatus {
    Queued,
    Detecting,
    Analyzing,
    Converting,
    Uploading,
    Downloading,
    Verifying,
    Succeeded,
    Failed,
    Canceled,
}

pub struct DesktopJob {
    pub id: String,
    pub input_path: PathBuf,
    pub profile_id: String,
    pub mode: ConvertMode,
    pub status: DesktopJobStatus,
    pub progress: u8,
    pub output_docx: Option<PathBuf>,
    pub report_path: Option<PathBuf>,
}
```

任务运行方式：

```text
UI callback
  ↓
commands.rs
  ↓
tokio task / worker thread
  ↓
local_convert or cloud_convert
  ↓
send status back to Slint event loop
```

---

## 七、本地转换设计

### 7.1 本地转换路径

```text
User Project
  ↓
ProjectNormalizer
  ↓
JournalDetector
  ↓
CompatibilityAnalyzer
  ↓
SemanticTexEngine
  ↓
DOCX Writer
  ↓
QualityGate
  ↓
Output DOCX + Report
```

### 7.2 本地转换能力分级

| 套餐 | 本地转换 |
|---|---|
| Free | limited，次数限制，generic/chinese preview |
| Pro | 7 Profile 本地转换，云端质量增强 |
| Team | 批量本地转换，团队额度同步 |
| Enterprise | 完整离线转换，离线 license |

### 7.3 本地 TeX Runtime

本地客户端不应强制用户安装 TeXLive，但应支持：

1. 无 TeX runtime：RuleBased 模式。
2. 检测到 XeLaTeX：启用 XeLaTeXHook。
3. 检测到 LuaLaTeX：启用 LuaTeXNode。
4. 用户可在 Settings 中配置 runtime 路径。

Runtime 检测：

```rust
pub struct RuntimeStatus {
    pub xelatex: RuntimeProbe,
    pub lualatex: RuntimeProbe,
    pub tectonic: RuntimeProbe,
}

pub struct RuntimeProbe {
    pub available: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}
```

---

## 八、云端转换设计

### 8.1 为什么需要云端

云端转换承担商业价值：

- 统一 XeLaTeX/LuaLaTeX/TeXLive 环境。
- 支持复杂模板。
- 支持质量验证。
- 支持批量转换。
- 支持用量计费。
- 支持企业审计。

### 8.2 客户端上传流程

```text
Normalize project
  ↓
zip project
  ↓
request upload URL
  ↓
upload to object storage
  ↓
create conversion job
  ↓
poll / subscribe job status
  ↓
download docx/report
```

### 8.3 API

```http
POST /v1/auth/register
POST /v1/auth/login
POST /v1/auth/refresh
POST /v1/devices/activate

GET  /v1/me
GET  /v1/usage
GET  /v1/plans
POST /v1/billing/checkout
POST /v1/billing/portal

POST /v1/uploads
POST /v1/conversions
GET  /v1/conversions/{id}
GET  /v1/conversions/{id}/download/docx
GET  /v1/conversions/{id}/report
DELETE /v1/conversions/{id}

GET  /v1/releases/latest
GET  /v1/releases/{version}/manifest
```

### 8.4 API Client crate

```text
crates/commercial-api-client/
├── src/
│   ├── lib.rs
│   ├── auth.rs
│   ├── billing.rs
│   ├── conversions.rs
│   ├── usage.rs
│   ├── releases.rs
│   └── error.rs
```

核心类型：

```rust
pub struct ApiClient {
    base_url: Url,
    http: reqwest::Client,
    token_store: Arc<dyn TokenStore>,
}

pub struct ConversionJob {
    pub id: String,
    pub status: JobStatus,
    pub profile_id: String,
    pub quality_score: Option<u8>,
    pub output_docx_url: Option<String>,
    pub report_url: Option<String>,
}
```

---

## 九、用户注册与登录设计

### 9.1 登录方式

推荐首期：

1. 邮箱 + 验证码。
2. 浏览器 OAuth/PKCE。
3. 设备授权码。

不建议在客户端直接处理银行卡或支付敏感信息。

### 9.2 Token 存储

使用 OS keychain：

| 平台 | 存储 |
|---|---|
| Windows | Credential Manager |
| macOS | Keychain |
| Linux | Secret Service / KWallet |

Rust crate：

```text
keyring
```

存储内容：

```text
access_token: 短期，可内存保存
refresh_token: keychain
device_id: app data
license_cache: app data + signature
```

### 9.3 登录流程

```text
用户点击登录
  ↓
客户端打开浏览器
  ↓
用户完成登录/注册
  ↓
浏览器回调本地 loopback URL 或复制授权码
  ↓
客户端换取 token
  ↓
保存 refresh token
  ↓
同步 usage 和 plan
```

### 9.4 离线策略

```text
Free: 离线仅允许有限本地 preview
Pro: 允许 7 天离线宽限
Team: 允许 14 天离线宽限
Enterprise: 按离线 license 文件配置
```

离线 license：

```json
{
  "user_id": "usr_xxx",
  "device_id": "dev_xxx",
  "plan": "pro",
  "features": ["local_convert", "journal_profiles"],
  "expires_at": "2026-07-21T00:00:00Z",
  "signature": "ed25519..."
}
```

---

## 十、套餐订阅与用量管控

### 10.1 订阅原则

客户端不直接处理支付。客户端只负责：

- 展示当前套餐。
- 展示用量。
- 打开 Checkout。
- 打开 Billing Portal。
- 收到订阅状态变化后刷新。

支付由服务端和支付平台处理。

### 10.2 套餐建议

| 套餐 | 功能 |
|---|---|
| Free | 每月 10 次，本地 RuleBased，低优先级云端 |
| Pro | 每月 200 次，7 Profile，云端标准质量 |
| Team | 每月 1000 次，批量转换，团队成员，历史记录 |
| Enterprise | 私有化部署，自定义 Profile，离线授权 |

### 10.3 用量模型

```rust
pub struct UsageSnapshot {
    pub plan_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub conversions_limit: u32,
    pub conversions_used: u32,
    pub cloud_seconds_limit: Option<u32>,
    pub cloud_seconds_used: u32,
    pub local_grace_remaining: u32,
}
```

### 10.4 用量扣减

建议：

| 操作 | 是否扣减 |
|---|---|
| Profile 检测 | 不扣 |
| Compatibility analyze | 不扣 |
| 本地 preview 转换 | Free 可扣，Pro 不扣或少扣 |
| 云端转换成功 | 扣 |
| 云端转换失败且非用户问题 | 不扣 |
| 用户取消 | 按阶段决定 |
| 批量转换 | 按文件扣 |

### 10.5 客户端本地管控

客户端只能做提示和弱管控，权威用量必须在服务端。

```text
客户端 usage cache
  ↓
转换前本地预判
  ↓
云端 authoritative check
  ↓
转换完成后同步
```

---

## 十一、自动升级设计

### 11.1 原则

自动升级是商业桌面端必须能力，但必须保证：

1. 安全签名。
2. 可回滚。
3. 分渠道发布。
4. 用户可延迟。
5. 企业版可关闭自动升级。

### 11.2 更新通道

```text
stable
beta
nightly
enterprise
```

### 11.3 Manifest

```json
{
  "version": "0.2.0",
  "channel": "stable",
  "mandatory": false,
  "notes_url": "https://tex2doc.app/releases/0.2.0",
  "platforms": {
    "windows-x86_64": {
      "url": "https://download.tex2doc.app/0.2.0/Tex2Doc-Setup.exe",
      "sha256": "...",
      "signature": "..."
    },
    "macos-aarch64": {
      "url": "https://download.tex2doc.app/0.2.0/Tex2Doc-aarch64.dmg",
      "sha256": "...",
      "signature": "..."
    },
    "linux-x86_64": {
      "url": "https://download.tex2doc.app/0.2.0/Tex2Doc.AppImage",
      "sha256": "...",
      "signature": "..."
    }
  }
}
```

### 11.4 平台策略

| 平台 | 包格式 | 更新策略 |
|---|---|---|
| Windows | MSIX / NSIS / MSI | 下载 installer，签名校验，静默或引导安装 |
| macOS | .app + DMG | Sparkle 或自建更新器，必须 codesign + notarize |
| Linux | AppImage / deb / rpm / Flatpak | AppImageUpdate 或包管理器提示 |

### 11.5 企业版策略

Enterprise 默认：

- 可关闭自动更新。
- 支持内网 update server。
- 支持固定版本。
- 支持管理员统一分发。

---

## 十二、打包与发布

### 12.1 Windows

构建：

```bash
cargo build -p tex2doc-desktop --release
```

打包：

```text
MSIX: 适合 Microsoft Store / 企业部署
NSIS: 适合独立下载安装包
MSI/WiX: 适合企业 IT 分发
```

必须：

- 代码签名证书。
- 安装目录写权限规范。
- 用户数据放 `%APPDATA%/Tex2Doc`。

### 12.2 macOS

打包：

```text
Tex2Doc.app
Tex2Doc.dmg
```

必须：

- Developer ID 签名。
- Hardened runtime。
- Notarization。
- Staple。
- 用户数据放 `~/Library/Application Support/Tex2Doc`。

### 12.3 Linux

打包：

```text
AppImage
deb
rpm
Flatpak
```

用户数据：

```text
~/.local/share/tex2doc
~/.config/tex2doc
```

建议首期：

```text
Linux 首发 AppImage + deb
rpm/Flatpak 后续补齐
```

### 12.4 CI 构建矩阵

```yaml
matrix:
  os:
    - windows-latest
    - macos-13
    - macos-14
    - ubuntu-22.04
  target:
    - x86_64-pc-windows-msvc
    - x86_64-apple-darwin
    - aarch64-apple-darwin
    - x86_64-unknown-linux-gnu
```

---

## 十三、安全与隐私

### 13.1 本地文件安全

客户端必须明确：

- 本地模式不上传文件。
- 云端模式上传前提示。
- 用户可选择是否保存历史。
- 默认只保存路径、Profile、报告摘要，不保存全文。

### 13.2 云端上传提示

上传前显示：

```text
此操作会上传论文项目到 Tex2Doc 云端转换服务。
文件将在转换完成后按套餐保留策略自动删除。
```

### 13.3 日志脱敏

日志禁止保存：

- 全文 TeX。
- 作者邮箱。
- 图片内容。
- API token。
- refresh token。

日志允许保存：

- profile id。
- backend。
- package 名。
- error code。
- compatibility score。
- raw fallback count。

### 13.4 Token 安全

要求：

- refresh token 只进 OS keychain。
- access token 只在内存和短期缓存。
- 日志永不输出 token。
- logout 时清除 keychain。

---

## 十四、客户端与服务端 API 契约

### 14.1 登录状态

```json
{
  "user": {
    "id": "usr_xxx",
    "email": "user@example.com"
  },
  "plan": {
    "id": "pro",
    "name": "Pro",
    "status": "active"
  },
  "usage": {
    "conversions_used": 42,
    "conversions_limit": 200,
    "period_end": "2026-07-21T00:00:00Z"
  }
}
```

### 14.2 创建转换任务

```json
{
  "main_tex": "main.tex",
  "profile": "auto",
  "mode": "cloud-standard",
  "quality_level": "standard",
  "input_upload_id": "upl_xxx"
}
```

### 14.3 转换状态

```json
{
  "id": "conv_xxx",
  "status": "rendering",
  "progress": 72,
  "profile": {
    "selected": "tacl",
    "confidence": 0.95
  },
  "backend": {
    "selected": "luatex-node",
    "fallback": false
  }
}
```

### 14.4 转换结果

```json
{
  "id": "conv_xxx",
  "status": "succeeded",
  "docx_url": "...",
  "report_url": "...",
  "quality": {
    "status": "passed_with_warnings",
    "score": 86
  }
}
```

---

## 十五、商业客户端 UI 状态机

### 15.1 Auth 状态

```text
Anonymous
  ↓ login
Authenticating
  ↓ success
Authenticated
  ↓ token expired
Refreshing
  ↓ fail
Anonymous
```

### 15.2 Conversion 状态

```text
Idle
  ↓ select project
ProjectSelected
  ↓ detect
Detected
  ↓ analyze
Analyzed
  ↓ convert
Converting
  ↓ success
Succeeded
  ↓ fail
Failed
```

### 15.3 Subscription 状态

```text
Free
  ↓ checkout
Pending
  ↓ webhook sync
Active
  ↓ payment failed
PastDue
  ↓ canceled
Canceled
```

---

## 十六、客户端错误码

```text
E_PROJECT_INVALID
E_MAIN_TEX_NOT_FOUND
E_PROFILE_LOW_CONFIDENCE
E_COMPAT_UNSUPPORTED
E_LOCAL_RUNTIME_MISSING
E_LOCAL_CONVERT_FAILED
E_CLOUD_UPLOAD_FAILED
E_CLOUD_QUOTA_EXCEEDED
E_CLOUD_CONVERT_FAILED
E_TOKEN_EXPIRED
E_SUBSCRIPTION_REQUIRED
E_UPDATE_CHECK_FAILED
E_UPDATE_SIGNATURE_INVALID
```

错误展示原则：

- 用户看到简洁可执行描述。
- 技术详情进入展开面板和报告。
- 提供复制诊断信息按钮。

---

## 十七、开发里程碑

### D0：可行性验证

周期：3 到 5 天

任务：

1. 新增 `crates/desktop-slint`。
2. 创建 Slint 主窗口。
3. 实现文件选择。
4. 调用 `doc-compiler-engine` 执行本地转换。
5. 输出 DOCX 到指定目录。

验收：

```text
Windows/Linux/macOS 至少一平台可运行
选择 examples/paper3 或 examples/journals/generic 可生成 DOCX
UI 不阻塞
```

### D1：转换工作台

周期：1 到 2 周

任务：

1. Convert 页面。
2. Jobs 页面。
3. Reports 页面。
4. Profile 检测展示。
5. Compatibility score 展示。
6. 本地转换报告展示。

验收：

```text
7 个 minimal fixture 可通过客户端操作
失败可展示错误报告
```

### D2：账号和商业 API

周期：2 周

任务：

1. 新增 `commercial-api-client`。
2. 登录/注册。
3. token 存储。
4. `/me`、`/usage`、`/plans`。
5. 客户端 Usage 页面。

验收：

```text
用户可登录
可看到套餐和用量
退出后 token 被清除
```

### D3：订阅和用量管控

周期：2 周

任务：

1. Checkout URL。
2. Billing Portal。
3. 本地用量缓存。
4. 转换前额度检查。
5. 云端转换用量同步。

验收：

```text
Free/Pro 权限差异可展示
额度不足时阻断云端转换
```

### D4：云端转换

周期：2 到 3 周

任务：

1. 上传项目。
2. 创建任务。
3. 轮询任务。
4. 下载 DOCX/report。
5. 云端失败诊断。

验收：

```text
客户端可完成云端转换闭环
中断后可恢复任务状态
```

### D5：自动升级

周期：2 周

任务：

1. release manifest。
2. 检查更新。
3. 下载更新包。
4. sha256 和签名校验。
5. 引导安装。
6. 更新日志展示。

验收：

```text
stable/beta 通道可切换
签名不匹配时拒绝更新
```

### D6：三平台发布

周期：3 到 4 周

任务：

1. Windows installer。
2. macOS DMG + notarization。
3. Linux AppImage/deb。
4. CI build matrix。
5. Smoke test。

验收：

```text
Windows/macOS/Linux 均有可安装产物
安装后可登录、转换、检查更新
```

---

## 十八、测试策略

### 18.1 单元测试

```text
auth token refresh
usage quota decision
conversion job state machine
update manifest verification
local config load/save
```

### 18.2 集成测试

```text
desktop-slint local convert generic fixture
desktop-slint local convert chinese fixture
commercial-api-client mock server
updater fake manifest
```

### 18.3 UI smoke

Slint UI 可做轻量 smoke：

```text
启动窗口
点击选择文件
触发转换
检查状态文本
检查生成文件
```

### 18.4 发布测试

| 平台 | 测试 |
|---|---|
| Windows | 安装、启动、登录、转换、卸载 |
| macOS | Gatekeeper、启动、转换、自动更新 |
| Linux | AppImage 执行、deb 安装、配置目录 |

---

## 十九、商业化验收标准

### Preview

```text
Slint 客户端可启动
可本地转换
可显示 Profile 检测
可显示兼容性报告
Windows/Linux 至少一平台产物
```

### Beta

```text
三平台可安装
账号登录可用
用量展示可用
云端转换可用
订阅入口可用
自动更新可用
```

### GA

```text
三平台签名发布
用量与套餐准确
云端转换 SLA 可控
崩溃率可监控
升级回滚可控
企业数据隐私策略明确
```

---

## 二十、风险与应对

| 风险 | 影响 | 应对 |
|---|---|---|
| Slint 授权不满足商业需求 | 发布风险 | 商业发布前购买商业许可或满足官方授权要求 |
| Slint 生态较小 | 开发部分控件成本高 | UI 保持工具化，避免复杂富交互 |
| 自动升级跨平台复杂 | 发布延迟 | 首期手动更新，Beta 前补 updater |
| macOS 签名公证复杂 | 无法安装 | 尽早建立 Apple Developer 流程 |
| Linux 发行格式碎片化 | 支持成本高 | 首发 AppImage + deb |
| 本地 TeX runtime 缺失 | 高质量转换不可用 | 云端转换兜底 |
| 用户隐私顾虑 | 上传转化率低 | 本地模式 + 明确上传提示 |
| 支付合规 | 客户端审核风险 | 支付全部放浏览器和服务端 |

---

## 二十一、与商业化方案的衔接

该 Slint 客户端与商业化总方案的关系：

```text
Semantic Engine = 产品内核
SaaS API = 商业后端
Slint Desktop = PC 端付费入口
Web = 获客入口
CLI = 专业用户入口
Extension = 轻量触达入口
```

推荐商业路径：

1. Web 免费试用获客。
2. Slint Desktop 提供更强本地能力。
3. Pro/Team 通过云端高质量转换变现。
4. Enterprise 通过离线授权和私有部署变现。

---

## 二十二、建议首期开发清单

立即开发项：

1. 新增 `crates/desktop-slint`。
2. 添加 Slint 依赖和最小窗口。
3. 实现文件选择与输出目录选择。
4. 调用 `doc-compiler-engine` 本地转换。
5. 展示 profile、backend、compatibility、quality。
6. 新增 `commercial-api-client` 骨架。
7. 实现登录 token 存储。
8. 实现 Usage 页面 mock 数据。
9. 实现自动更新 manifest 检查 mock。
10. 编写 Windows/Linux 本地构建脚本。

---

## 二十三、参考资料

- Slint 官方网站：https://slint.dev/
- Slint Rust 文档：https://docs.slint.dev/latest/docs/rust/slint/
- Slint 支持平台说明：https://docs.slint.dev/latest/docs/slint/guide/platforms/
- Slint Pricing / Licensing：https://slint.dev/pricing

---

## 二十四、结论

基于当前项目目标，Rust + Slint 是新增 PC 客户端的可行路线。它能最大化复用现有 Rust 转换引擎，并降低 Flutter/Dart FFI 带来的复杂度。Slint 本身不提供账号、订阅、用量和自动升级能力，因此商业化客户端必须配套建设：

```text
commercial-api-client
license-client
usage/quota 模块
updater 模块
release manifest
服务端 billing/conversion API
```

建议把 Slint Desktop 作为新的商业 PC 客户端主线，先完成本地转换 MVP，再接入账号、订阅、用量、云端转换和自动升级，最终形成 Windows/macOS/Linux 三平台可发布的商业化客户端。
