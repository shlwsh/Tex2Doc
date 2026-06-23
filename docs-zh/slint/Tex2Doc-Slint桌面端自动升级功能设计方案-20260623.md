# Tex2Doc Slint 桌面端自动升级功能设计方案

日期：2026-06-23
版本：v1

---

## 一、项目概述

### 1.1 背景

Tex2Doc Slint 桌面端（`crates/desktop-slint`）已在 P9 阶段实现了**版本检查**的预览功能：

- `src/updater.rs`：manifest 解析、语义化版本比较、SHA256 校验
- `src/desktop_update.rs`：调用 `/v1/releases/{channel}` API 获取 release manifest
- `src/main.rs` + `main.slint`：Settings 页有"检查更新"按钮，结果显示在 `update-status` 文本区
- `src/settings.rs`：已存储 `release_channel`（stable/beta），版本号从 `VERSION` 文件注入

当前版本号 `1.26.6.1`。

### 1.2 目标

将 P9 预览版本检查升级为**完整的自动升级系统**，包括：

1. **后台静默检查**：应用启动时自动检查更新，不阻塞 UI
2. **下载管理**：下载新版本安装包，实时显示进度
3. **校验与安全**：下载完成后 SHA256 校验，签名验证占位
4. **安装执行**：退出当前进程，启动安装程序
5. **多平台支持**：Windows（`.exe` / `.msi`）、macOS（`.dmg`）、Linux（`.AppImage` / `.deb`）
6. **用户控制**：设置页可配置更新渠道、自动检查频率，可手动触发检查/忽略版本
7. **差量更新预留**：架构上支持 future 差量更新（初期实现全量下载）

### 1.3 范围

| 分类 | 内容 |
|------|------|
| Rust 后端 | `src/update_downloader.rs`（新增）、`src/update_installer.rs`（新增）、`src/updater.rs`（扩展）、`src/desktop_update.rs`（扩展）、`src/settings.rs`（扩展） |
| Slint UI | Settings 页升级面板（新增下载进度、版本信息、操作按钮）、通知 Toast |
| 不在范围 | 服务端 release manifest API 实现、CI/CD release 上传脚本、实际签名验证密钥体系、差量/补丁更新（预留架构但初期不做） |

---

## 二、现状审计

### 2.1 已有基础设施

| 组件 | 位置 | 现状 | 可用性 |
|------|------|------|--------|
| 版本比较 | `updater.rs::is_newer_version()` | 语义化版本比较，测试完备 | ✅ 直接复用 |
| Manifest 解析 | `updater.rs::parse_manifest()` | 支持 version/channel/download_url/sha256/signature/release_notes | ✅ 直接复用 |
| SHA256 校验 | `updater.rs::verify_sha256()` | 已实现并有单元测试 | ✅ 直接复用 |
| API 清单获取 | `desktop_update.rs::check_update_blocking()` | 调用 `/v1/releases/{channel}`，返回 `DesktopUpdateCheck` | ✅ 复用网络层 |
| HTTP Client | `doc_commercial-api-client::ApiClient` | 基于 reqwest，已支持 timeout | ✅ 复用 |
| Settings 存储 | `settings.rs` | JSON 持久化，含 `release_channel` | ✅ 扩展字段 |
| UI 入口 | `main.slint` Settings 页 | "检查更新"按钮 + `update-status` 文本 | ✅ 扩展 |
| VERSION 文件 | `VERSION` | 当前版本 `1.26.6.1`，`build.rs` 注入编译时 | ✅ 直接使用 |
| tokio runtime | `desktop_update.rs` | 单线程 runtime，已验证 | ✅ 复用 |
| 平台检测 | `credential_store.rs` | Windows/macOS/Linux 分支已有 | ✅ 复用模式 |

### 2.2 缺失功能清单

| 功能 | 当前状态 | 优先级 |
|------|----------|--------|
| 后台静默检查 | 无 | P0 |
| 下载安装包 | 无 | P0 |
| 下载进度回调 | 无 | P0 |
| 安装程序执行 | `updater.rs::install_update_placeholder()` 直接返回 `InstallUnsupported` | P0 |
| SHA256 下载后校验 | 无 | P0 |
| 用户确认对话框 | 无 | P1 |
| 忽略版本（稍后提醒） | 无 | P1 |
| 自动检查频率配置 | 无 | P1 |
| 下载取消 | 无 | P1 |
| 多平台 installer 支持 | 无 | P1 |
| 差量更新 | 无（预留架构） | P2 |
| 签名验证密钥体系 | 占位（`SignatureStatus::DeferredVerification`） | P2 |

---

## 三、架构设计

### 3.1 模块划分

```
crates/desktop-slint/src/
├── updater.rs              # 已有：manifest 解析、版本比较、SHA256 校验（扩展）
├── desktop_update.rs        # 已有：API 调用封装（扩展）
├── update_downloader.rs     # 新增：下载管理、进度回调、断点续传
├── update_installer.rs      # 新增：平台安装程序执行
└── settings.rs              # 已有：扩展更新相关配置字段
```

### 3.2 数据流

```
┌─────────────────────────────────────────────────────────────┐
│                    应用启动 (main.rs)                        │
└──────────────┬──────────────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│              UpdateManager (单例, AppState)                  │
│  - 自动检查（后台线程，不阻塞 UI）                            │
│  - 状态机: Idle → Checking → Available → Downloading →       │
│              Ready → Installing → Done                       │
└──────────────┬──────────────────────────────────────────────┘
               │
       ┌───────┴───────┐
       ▼               ▼
  Settings 页        后台 Timer
  手动检查           定时检查
       │               │
       ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│  desktop_update.rs / update_downloader.rs                    │
│  1. check_update_blocking() → 获取 manifest                  │
│  2. is_newer_version() → 版本比较                           │
│  3. 下载安装包 → 进度回调 → SHA256 校验                      │
│  4. 校验通过 → update_status = Ready                        │
│  5. 用户点击"安装" → update_installer.rs → 退出当前进程     │
└─────────────────────────────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│  Slint UI (main.slint / pages/settings.slint)               │
│  - update_status: "--" / "Checking..." / "v1.27.0 available"│
│  - update_progress: 0-100（下载进度）                        │
│  - update_downloaded: bool（下载完成标志）                    │
│  - update_notes: string（release notes）                     │
│  - AlertBanner / ToastNotify（升级提示）                     │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 状态机设计

```rust
// src/update_downloader.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateState {
    /// 空闲，无待处理更新
    Idle,
    /// 正在检查更新（网络请求中）
    Checking,
    /// 有可用更新（已下载 manifest，版本更新）
    Available {
        version: String,
        release_notes: String,
        download_url: String,
        sha256: String,
    },
    /// 正在下载安装包
    Downloading { progress: f32 },  // 0.0 - 1.0
    /// 下载完成，已校验，待安装
    Ready { file_path: PathBuf },
    /// 安装中
    Installing,
    /// 安装完成（提示用户重启）
    Done,
    /// 出错
    Error { message: String },
}

impl Default for UpdateState {
    fn default() -> Self { Self::Idle }
}
```

---

## 四、详细设计

### 4.1 Settings 扩展

在 `settings.rs` 的 `Settings` 结构中新增以下字段：

```rust
// src/settings.rs

/// P9: User settings for the desktop client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // ... 已有字段 ...

    /// 是否在启动时自动检查更新
    #[serde(default = "default_auto_check")]
    pub auto_check_updates: bool,

    /// 自动检查频率（小时），默认 6 小时
    #[serde(default = "default_check_interval_hours")]
    pub check_interval_hours: u32,

    /// 是否已通知过当前可用版本（避免每次启动重复弹窗）
    #[serde(default)]
    pub notified_version: Option<String>,
}
```

默认值：

```rust
fn default_auto_check() -> bool { true }
fn default_check_interval_hours() -> u32 { 6 }
```

### 4.2 下载管理器 `update_downloader.rs`

#### 核心结构

```rust
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use reqwest::Client;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SHA256 mismatch")]
    Sha256Mismatch,
    #[error("download was cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, DownloadError>;

/// 下载任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    Pending,
    InProgress { bytes_downloaded: u64, total_bytes: u64 },
    Completed { file_path: PathBuf },
    Failed { error: String },
    Cancelled,
}

/// 下载管理器单例
pub struct DownloadManager {
    client: Client,
    download_dir: PathBuf,
    state: RwLock<DownloadStatus>,
    cancel_flag: RwLock<bool>,
}

impl DownloadManager {
    /// 创建下载管理器（从 AppState 中以 Arc 共享）
    pub fn new() -> Self {
        let download_dir = directories::ProjectDirs::from("com", "tex2doc", "Tex2Doc")
            .map(|dirs| dirs.cache_dir().join("updates"))
            .unwrap_or_else(|| PathBuf::from("./updates"));

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .expect("reqwest client"),
            download_dir,
            state: RwLock::new(DownloadStatus::Pending),
            cancel_flag: RwLock::new(false),
        }
    }

    /// 下载并校验安装包
    /// progress_callback 每秒调用一次，传入 (bytes_downloaded, total_bytes)
    pub async fn download_update(
        self: Arc<Self>,
        download_url: &str,
        expected_sha256: &str,
        version: &str,
        progress_callback: impl Fn(u64, u64) + Send + 'static,
    ) -> Result<PathBuf> {
        // 1. 创建下载目录
        std::fs::create_dir_all(&self.download_dir)?;

        // 2. 确定文件名（从 URL 或 platform 检测）
        let filename = self.platform_installer_name(version)?;
        let file_path = self.download_dir.join(&filename);

        // 3. 检查已下载文件（断点续传 or 跳过）
        let existing_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

        // 4. 下载（带 range 支持断点续传）
        let mut response = self.client.get(download_url)
            .header("User-Agent", "Tex2Doc-Desktop/1.0")
            .send()
            .await?;

        let total_size: u64 = response.content_length()
            .unwrap_or(0)
            .max(existing_size);

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        let mut downloaded: u64 = existing_size;
        let mut stream = response.bytes_stream();

        use tokio::io::AsyncWriteExt;
        while let Some(chunk_result) = stream.next().await {
            // 检查取消标记
            if *self.cancel_flag.read().await {
                return Err(DownloadError::Cancelled);
            }

            let chunk = chunk_result?;
            tokio::task::spawn_blocking({
                let mut f = file;
                move || std::io::Write::write_all(&mut f, &chunk)
            }).await??;

            downloaded += chunk.len() as u64;
            progress_callback(downloaded, total_size);
        }

        // 5. 校验 SHA256
        let final_bytes = tokio::task::spawn_blocking({
            let path = file_path.clone();
            move || std::fs::read(&path)
        }).await??;

        let digest = sha2::Sha256::digest(&final_bytes);
        let actual = format!("{digest:x}");

        if !actual.eq_ignore_ascii_case(expected_sha256) {
            // 校验失败，删除文件
            let _ = std::fs::remove_file(&file_path);
            return Err(DownloadError::Sha256Mismatch);
        }

        Ok(file_path)
    }

    /// 取消下载
    pub async fn cancel(&self) {
        *self.cancel_flag.write().await = true;
    }

    /// 获取平台对应的 installer 文件名
    fn platform_installer_name(&self, version: &str) -> Result<String> {
        #[cfg(target_os = "windows")]
        return Ok(format!("Tex2Doc-{version}-windows.exe"));

        #[cfg(target_os = "macos")]
        return Ok(format!("Tex2Doc-{version}-macos.dmg"));

        #[cfg(target_os = "linux")]
        return Ok(format!("Tex2Doc-{version}-linux.AppImage"));

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return Err(DownloadError::Io(std::io::Error::other("unsupported platform")));
    }
}
```

#### 进度回调到 Slint UI

进度通过 `Arc<DownloadManager>` + `slint::invoke_from_ui` 或 Rust 侧直接 `set_property` 同步到 Slint：

```rust
// 在 main.rs 的 UI 回调中
let download_manager = Arc::new(DownloadManager::new());
let ui_handle = ui.clone();

tokio::spawn(async move {
    let result = download_manager.clone()
        .download_update(&url, &sha256, &version, move |done, total| {
            let progress = if total > 0 { done as f32 / total as f32 } else { 0.0 };
            ui_handle.set_update_progress(progress);
        })
        .await;

    match result {
        Ok(path) => {
            ui_handle.set_update_downloaded(true);
            ui_handle.set_update_ready_path(path.to_string_lossy().into());
            ui_handle.set_update_status(format!("v{} ready — click Install to update", version).into());
        }
        Err(e) => {
            ui_handle.set_update_status(format!("Download failed: {}", e).into());
        }
    }
});
```

### 4.3 安装执行 `update_installer.rs`

#### 平台差异

```rust
// src/update_installer.rs

#[cfg(target_os = "windows")]
pub fn install_update(installer_path: &Path) -> std::io::Result<std::process::Child> {
    // Windows: 启动安装程序，传递静默参数，然后退出当前进程
    // 方案A: NSIS 安装程序（/S 静默）
    // 方案B: MSI 安装程序（msiexec /i）
    std::process::Command::new(installer_path)
        .arg("/S")           // NSIS 静默安装
        .arg("/D=$INSTDIR") // 安装目录
        .spawn()
}

#[cfg(target_os = "macos")]
pub fn install_update(dmg_path: &Path) -> std::io::Result<std::process::Child> {
    // macOS: 挂载 DMG，然后启动 .app bundle 中的安装脚本
    let mount_point = "/Volumes/Tex2Doc";
    std::process::Command::new("hdiutil")
        .args(["attach", dmg_path.to_str().unwrap(), "-mountpoint", mount_point])
        .spawn()?;

    // 从挂载的 DMG 中复制 .app 并替换
    // 具体实现依赖 CI 发布产物结构
    Ok(std::process::Command::new("open")
        .arg(format!("{}/Tex2Doc.app", mount_point))
        .spawn()?)
}

#[cfg(target_os = "linux")]
pub fn install_update(appimage_path: &Path) -> std::io::Result<std::process::Child> {
    // Linux: 直接执行 AppImage（self-update）或通过 dpkg 安装 .deb
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(appimage_path)
        .spawn()?;

    std::process::Command::new(appimage_path)
        .arg("--install")
        .spawn()
}
```

#### 安全退出当前进程

```rust
/// 安装程序启动后，延迟退出当前应用，给安装程序留出启动时间
pub fn schedule_exit(delay_secs: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(delay_secs));
        std::process::exit(0);
    });
}
```

> **注意**：安装程序需要具备**提升权限**能力（Windows UAC / macOS Authorization / Linux sudo）。初期实现以非特权用户安装到用户目录为主，避免复杂权限处理。

### 4.4 扩展 `desktop_update.rs`

新增 `download_and_install` 入口函数：

```rust
// src/desktop_update.rs

/// 完整升级流程（检查 + 下载 + 校验），不含安装执行
pub fn check_download_and_notify_blocking(
    base_url: &str,
    channel: &str,
    current_version: &str,
) -> Result<UpdateState> {
    // 1. 检查 manifest（已有逻辑）
    let check = check_update_blocking(base_url, channel, current_version)?;

    if !check.decision.update_available {
        return Ok(UpdateState::Idle);
    }

    // 2. 下载并校验（新增）
    let manager = Arc::new(DownloadManager::new());
    let file_path = manager.download_update_blocking(
        &check.decision.download_url,
        &check.sha256,
        &check.decision.latest_version,
        |_, _| {},  // 进度回调（可选）
    )?;

    Ok(UpdateState::Ready { file_path })
}
```

### 4.5 Slint UI 扩展

#### 新增 Slint 属性（`main.slint` Window）

```slint
// 在 MainWindow 中新增属性
in-out property <float> update-progress: 0.0;   // 下载进度 0.0-1.0
in-out property <bool> update-downloaded: false; // 下载完成标志
in-out property <string> update-version: "";      // 可用版本号
in-out property <string> update-notes: "";         // release notes
in-out property <bool> update-available: false;   // 有可用更新
in-out property <bool> is-downloading: false;     // 正在下载中

// Callbacks
callback install-update-clicked();
callback ignore-version-clicked(string);
callback download-update-clicked(string, string, string, string);
// params: (download_url, sha256, version, release_notes)
```

#### Settings 页升级面板 UI 设计

```
┌─ Updates ──────────────────────────────────────────────────────┐
│                                                               │
│  [Channel]  [stable ▼]                                        │
│  [☑] Check for updates automatically                          │
│  [Check Now]                                                  │
│                                                               │
│  ┌─ v1.27.0 available ────────────────────────────────────┐  │
│  │ Release Date: 2026-06-20                               │  │
│  │ • Fixed issue with conversion timeout on large projects  │  │
│  │ • Improved PDF report generation                        │  │
│  │ • Dark mode performance improvements                    │  │
│  │                                                          │  │
│  │  ████████████░░░░░░░░░░░░░░░░  67%  (12.4 MB / 18.5MB) │  │
│  │                                                          │  │
│  │  [Download]  [Install & Restart]  [Remind Me Later]     │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                               │
│  Current Version: 1.26.6.1                                   │
└───────────────────────────────────────────────────────────────┘
```

组件层级：

```
PanelSection { title: t-settings-updates
  HorizontalBox {  // channel row
    LineEdit   { placeholder-text: t-settings-release-channel }
    ComboBox   { model: ["stable", "beta", "dev"] }
    CheckBox   { text: t-settings-auto-check }
  }

  Button      { text: t-settings-check-now; clicked => ... }

  // 动态显示：有更新时展开
  GroupBox {
    title: "v{update-version} available";
    visible: root.update-available;

    Text  { text: root.update-notes; wrap: word-wrap; font-size: token-font-size-sm; color: token-text-secondary }

    // 下载进度条（下载中可见）
    Rectangle {
      visible: root.is-downloading;
      ProgressBar { progress: root.update-progress }
      Text { text: "{int(root.update-progress * 100)}%"; font-size: token-font-size-sm }
    }

    // 操作按钮
    HorizontalBox {
      ButtonSecondary {
        text: t-settings-download;
        visible: !root.update-downloaded && !root.is-downloading;
        clicked => { root.download-update-clicked(download_url, sha256, version, notes); }
      }
      ButtonPrimary {
        text: t-settings-install;
        visible: root.update-downloaded;
        clicked => { root.install-update-clicked(); }
      }
      ButtonSecondary {
        text: t-settings-remind-later;
        visible: root.update-available;
        clicked => { root.ignore-version-clicked(root.update-version); }
      }
    }
  }

  Text { text: "Current Version: {TEX2DOC_DESKTOP_VERSION}"; color: token-text-muted }
}
```

#### Toast 通知

应用启动后台检查发现新版本时，通过 `ToastNotify` 提示用户：

```slint
// 在 main.slint 顶部
ToastNotify {
  visible: root.update-available && !root.update-downloaded;
  message: "v{root.update-version} available — {root.update-notes}";
  level: "info";
}
```

### 4.6 后台自动检查

在 `main.rs` 启动流程中注册定时检查：

```rust
// src/main.rs

use std::sync::Arc;
use tokio::time::{interval, Duration};
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> () {
    // ... 初始化 settings, app_state, ui ...

    let settings = Settings::load();
    let current_version = env!("TEX2DOC_DESKTOP_VERSION");

    // === 自动更新检查 ===
    if settings.auto_check_updates {
        let should_check = settings.notified_version.as_ref()
            .map(|v| v != current_version)
            .unwrap_or(true);

        if should_check {
            let base_url = settings.api_base_url.clone();
            let channel = settings.release_channel.clone();
            let ui = ui.clone();

            std::thread::spawn(move || {
                // 后台线程执行，不阻塞 UI 启动
                match desktop_update::check_update_blocking(&base_url, &channel, current_version) {
                    Ok(check) if check.decision.update_available => {
                        slint::invoke_from_ui(&ui, move || {
                            ui.set_update_available(true);
                            ui.set_update_version(check.decision.latest_version.into());
                            ui.set_update_notes(check.decision.release_notes.into());
                            // 存储已通知版本
                            let mut s = Settings::load();
                            s.notified_version = Some(check.decision.latest_version.clone());
                            let _ = s.save();
                        });
                    }
                    _ => {}
                }
            });
        }
    }

    // === 定时重新检查（每 N 小时）===
    if settings.auto_check_updates {
        let ui = ui.clone();
        let base_url = settings.api_base_url.clone();
        let channel = settings.release_channel.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(
                settings.check_interval_hours as u64 * 3600
            ));
            loop {
                ticker.tick().await;
                if let Ok(check) = desktop_update::check_update_blocking(&base_url, &channel, current_version) {
                    if check.decision.update_available {
                        slint::invoke_from_ui(&ui, move || {
                            ui.set_update_available(true);
                            ui.set_update_version(check.decision.latest_version.into());
                            ui.set_update_notes(check.decision.release_notes.into());
                        });
                    }
                }
            }
        });
    }

    ui.run().unwrap();
}
```

---

## 五、API 侧 Release Manifest 格式

服务端 `/v1/releases/{channel}` 返回格式（已在 `models.rs` 中定义）：

```json
{
  "version": "1.27.0",
  "channel": "stable",
  "download_url": "https://releases.tex2doc.cn/desktop/1.27.0/Tex2Doc-1.27.0-windows.exe",
  "sha256": "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456",
  "signature": "pending-p9-signature",
  "release_notes": "## v1.27.0\n- Fixed conversion timeout issue\n- Improved dark mode performance"
}
```

服务端需根据请求头 `User-Agent` 或额外的 `platform` 参数返回对应平台的 `download_url`。

---

## 六、安全设计

### 6.1 校验链

```
下载完成 → SHA256 校验 → 签名验证（预留） → 安装执行
```

| 阶段 | 当前实现 | 状态 |
|------|---------|------|
| SHA256 校验 | `updater.rs::verify_sha256()` | ✅ 完备 |
| 签名验证 | `updater.rs::verify_manifest_signature()` 返回 `DeferredVerification` | ⚠️ 占位，待接入密钥体系 |
| 签名密钥存储 | 无 | ❌ 待实现（Windows: 证书存储, macOS: Keychain, Linux: gpg） |
| HTTPS 传输 | reqwest 默认验证 TLS 证书 | ✅ |

### 6.2 安装程序来源验证

初期仅校验 SHA256。未来接入签名验证后，需：

1. 发布时用私钥对 `installer.sha256` 文件签名
2. 安装前从 **独立于 download_url 的 URL** 获取公钥（防止 download_url 被篡改后中间人注入）
3. 验证签名通过后才执行安装

### 6.3 下载目录隔离

- 下载目录：`%LOCALAPPDATA%/com.tex2doc.Tex2Doc/cache/updates/`（Windows）
- 仅当前用户可读写，防止低权限目录注入
- 安装程序执行前校验文件路径在预期目录内

---

## 七、多平台细节

### 7.1 Windows

| 项目 | 说明 |
|------|------|
| 安装程序类型 | 初期：NSIS `.exe` 打包；未来：MSI |
| 静默参数 | `/S`（NSIS）/ `msiexec /i`（MSI） |
| 安装目录 | `%LOCALAPPDATA%/Programs/Tex2Doc`（用户级，避免 UAC） |
| 升级方式 | 旧版通过 `/S /D=<old_dir>` 静默卸载 + 新版安装 |
| 进程退出 | 启动 installer 后 `std::process::exit(0)` |

### 7.2 macOS

| 项目 | 说明 |
|------|------|
| 安装程序类型 | `.dmg` + 内置安装脚本 或 `.pkg` |
| UAC 等价物 | macOS Gatekeeper / Notarization（需签名 + notarize） |
| 升级方式 | 挂载 DMG → 拖拽替换 `.app` |
| 签名要求 | 需要 Apple Developer 证书 + Notarization（初期可跳过测试） |

### 7.3 Linux

| 项目 | 说明 |
|------|------|
| 安装程序类型 | `.AppImage`（推荐）或 `.deb` |
| AppImage 自升级 | `AppImage --update` 支持内置自升级 |
| .deb 安装 | `dpkg -i` 或 `apt install ./xxx.deb` |
| 权限 | 用户级安装到 `~/.local/bin` |

---

## 八、i18n 扩展

在 `i18n-strings.slint` 中新增以下 key（v2 UI 设计方案中定义）：

```slint
// === Settings / Updates ===
in-out property <string> settings-updates: "Updates";
in-out property <string> settings-release-channel: "Release Channel";
in-out property <string> settings-auto-check: "Check for updates automatically";
in-out property <string> settings-check-now: "Check Now";
in-out property <string> settings-download: "Download";
in-out property <string> settings-install: "Install & Restart";
in-out property <string> settings-remind-later: "Remind Me Later";
in-out property <string> settings-checking-update: "Checking for updates...";
in-out property <string> settings-downloading-update: "Downloading update...";
in-out property <string> settings-no-update: "You're up to date!";
in-out property <string> settings-update-available: "v# available";
in-out property <string> settings-update-ready: "Update ready — click Install to update";
in-out property <string> settings-download-failed: "Download failed. Please try again.";
in-out property <string> settings-install-failed: "Installation failed.";

// === Toast ===
in-out property <string> toast-update-available: "v# available — click to download";
in-out property <string> toast-update-ready: "Update ready. Click to install.";
```

---

## 九、错误处理

| 场景 | 处理方式 |
|------|---------|
| 网络超时（30s） | `update-status` 显示 "Network error, please try again later"，Button 重置为可点击 |
| SHA256 校验失败 | 删除损坏文件，`update-status` 显示 "Download corrupted. Retrying..."，自动重试一次 |
| 磁盘空间不足 | 检测 `std::io::ErrorKind::OutOfSpace`，提示用户清理磁盘 |
| 安装程序不存在/权限不足 | `install_update()` 返回错误，显示 "Installation failed — please run as administrator" |
| 后台检查失败 | 静默失败，不显示错误，最多重试 3 次，每次间隔 1h |
| 用户取消下载 | `cancel_flag = true`，进度归零，Button 恢复为"Download" |
| 重复升级（已在安装中） | Button disabled，`update-status` 显示 "Installing..." |

---

## 十、实施计划

### Phase 1：下载管理器（预计 1 天）

| 任务 | 文件 |
|------|------|
| 创建 `update_downloader.rs`：`DownloadManager` 结构体、下载逻辑 | `src/update_downloader.rs` |
| 实现 SHA256 校验 + 断点续传 | `src/update_downloader.rs` |
| 进度回调机制（`Arc<DownloadManager>` + UI 属性同步） | `src/update_downloader.rs` + `main.rs` |
| 平台 installer 文件名检测 | `src/update_downloader.rs` |
| 单元测试：下载 mock、校验失败场景 | `src/update_downloader.rs` |

### Phase 2：UI 扩展（预计 1 天）

| 任务 | 文件 |
|------|------|
| `main.slint` 新增 update 相关属性和 callbacks | `src/ui/main.slint` |
| Settings 页升级面板 UI（含进度条、操作按钮） | `src/ui/pages/settings.slint` |
| i18n 新增 update 相关 key | `src/i18n.rs` + `src/ui/i18n-strings.slint`（方案见 v2 设计文档） |
| `main.rs` 回调绑定：`download-update-clicked`、`install-update-clicked`、`ignore-version-clicked` | `src/main.rs` |
| Toast 通知集成 | `src/ui/main.slint` |

### Phase 3：安装执行（预计 1 天）

| 任务 | 文件 |
|------|------|
| 创建 `update_installer.rs`：Windows/macOS/Linux 三平台实现 | `src/update_installer.rs` |
| 安全退出当前进程（延迟 exit） | `src/update_installer.rs` |
| Settings 页"Install & Restart"按钮绑定 | `src/main.rs` |
| Windows NSIS 静默参数测试 | — |

### Phase 4：后台检查与配置（预计 1 天）

| 任务 | 文件 |
|------|------|
| `settings.rs` 新增 `auto_check_updates`、`check_interval_hours`、`notified_version` | `src/settings.rs` |
| 启动时后台自动检查（独立线程，不阻塞 UI） | `src/main.rs` |
| 定时重新检查机制（tokio interval） | `src/main.rs` |
| "Ignore This Version" 功能（存储已忽略版本号） | `src/main.rs` + `src/settings.rs` |
| "Remind Me Later" 重置通知状态 | `src/main.rs` |

### Phase 5：测试与完善（预计 1 天）

| 任务 | 说明 |
|------|------|
| 端到端测试：检查 → 下载 → 校验 → 模拟安装 | mock server 模拟 |
| 多平台测试（Windows 为主，macOS/Linux 手动验证） | — |
| 网络异常场景测试 | 断网、限速、超时 |
| 磁盘空间不足场景测试 | — |
| CI 构建验证 | `cargo build -p desktop-slint` |
| cargo test | 确保新增代码无回归 |

---

## 十一、风险与缓解

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| SHA256 校验后安装程序仍含恶意代码 | **高** | 预留签名验证接口，待密钥体系就绪后接入 |
| 下载中断导致文件损坏 | **中** | 断点续传 + 完成后强制 SHA256 校验 |
| 用户正在转换时触发升级安装 | **中** | 升级前检查 `is-converting` 状态，转换中禁用安装按钮 |
| 静默安装参数不生效（不同 NSIS 版本） | **低** | 初期提供图形化安装程序作为备选 |
| macOS Gatekeeper 阻止未签名 app | **中** | 初期以 `.app` 直接替换方式绕过 notarization |
| 升级后启动的是旧版可执行文件（进程缓存） | **低** | 安装后显式告知用户重启，关闭当前进程 |

---

## 十二、验收标准

| # | 标准 |
|---|------|
| 1 | 启动时后台静默检查更新（无 UI 阻塞），有更新时右上角 Toast 提示 |
| 2 | Settings 页"Check Now"显示 `update_status` 进度，`Checking...` → `v1.27.0 available` |
| 3 | 下载过程实时显示进度条（0-100%），SHA256 校验通过后状态变为 `Ready` |
| 4 | 下载完成后"Install & Restart"按钮可用，点击后退出当前进程 |
| 5 | 用户可在 Settings 页切换 stable / beta / dev 渠道 |
| 6 | 转换进行中时"Install"按钮禁用，转换结束后恢复 |
| 7 | 网络错误时显示友好错误提示，不闪退 |
| 8 | `cargo build -p desktop-slint` 编译通过 |
| 9 | `cargo test -p desktop-slint` 测试通过 |
| 10 | SHA256 校验失败的下载文件被删除，不留磁盘残留 |

---

## 十三、与 v2 界面重构方案的协同

本文档与 `Tex2Doc-Slint桌面端商业化界面重构设计方案-v2-20260623.md` 存在以下交叉点：

| 协同点 | 说明 |
|--------|------|
| 组件复用 | 升级面板使用 v2 定义的 `AlertBanner`、`ToastNotify`、`ButtonPrimary`、`ButtonSecondary`、`SkeletonBlock` |
| i18n key | Settings 页升级面板文本全部通过 `I18n.settings-*` key 引用 |
| 状态建模 | 下载进度通过 `update-progress` property 绑定到 Slint ProgressBar |
| Settings 重构 | v2 方案中 `pages/settings.slint` 的重构包含本方案的 UI 扩展部分 |
| 错误展示 | 所有升级错误使用 v2 的 `AlertBanner` 组件展示 |
