# Semantic TeX Engine P5 Desktop Cloud Convert 开发进展报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**时间戳**：20260622-011837  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P5 Slint Desktop MVP、P6 商业 API、P7 云端转换 Worker  
**本轮目标**：在桌面端账号/用量入口基础上，继续打通上传、创建云端转换、轮询、下载 DOCX/report 的 Cloud Convert MVP。

---

## 一、当前结论

本轮完成了桌面端第一条云端转换闭环的代码接入：

```text
Slint Desktop
  -> access token
  -> package project as zip
  -> upload_project_zip
  -> create_conversion
  -> poll get_conversion
  -> download_conversion_docx
  -> get_conversion_report
  -> write DOCX/report to local output path
```

当前能力定位：

| 能力 | 状态 |
---|---|
| 本地转换 | 已有 |
| 登录/用量 | 已有 |
| 云端转换按钮 | 新增 |
| 目录自动打包 zip | 新增 |
| 直接上传 zip | 新增 |
| 轮询云端任务 | 新增 |
| 下载 DOCX | 新增 |
| 下载并保存 report JSON | 新增 |
| GUI 真实操作验收 | 未完成 |
| token 安全存储 | 未完成 |
| server 生产持久化/sandbox | 未完成 |

因此 P5 已从“本地桌面 MVP + 账号入口”推进到“桌面商业闭环 MVP 雏形”，但仍不能标记 completed。

---

## 二、实现内容

### 2.1 新增 Cloud Convert 适配层

新增文件：

```text
crates/desktop-slint/src/cloud_convert.rs
```

核心类型：

```rust
CloudConvertResult
CloudConvertError
```

核心入口：

```rust
convert_project_blocking(
    base_url,
    access_token,
    project_path,
    main_tex,
    output_docx,
    profile,
    quality,
)
```

该入口完成：

1. 检查 access token。
2. 读取或打包项目。
3. 调用 `upload_project_zip`。
4. 调用 `create_conversion`。
5. 轮询 `get_conversion`，等待 `completed + docx_ready + report_ready`。
6. 下载 DOCX。
7. 下载 report。
8. 将 DOCX 写入用户指定路径。
9. 将 report 写入同名 `.report.json`。

### 2.2 目录打包策略

`cloud_convert.rs` 支持两种输入：

| 输入 | 行为 |
---|---|
| `.zip` 文件 | 直接读取并上传 |
| 目录 | 递归打包为 zip 后上传 |

目录打包会跳过：

```text
.git
target
output
.DS_Store
__pycache__
```

Main TeX 解析策略：

1. UI 显式传入 `main_tex` 时优先使用。
2. 目录输入且未显式传入时，按候选文件查找：
   - `main.tex`
   - `main-jos.tex`
   - `minimal.tex`
   - `paper.tex`
   - `article.tex`
3. zip 文件未显式传入时默认 `main.tex`。

说明：paper3 这类项目通常应在 UI 中填写 `main-jos.tex`。

### 2.3 Slint UI 扩展

修改文件：

```text
crates/desktop-slint/src/ui/main.slint
```

新增属性：

```text
main-tex
```

新增 callback：

```text
cloud-convert-clicked(string, string, string, string, string, string)
```

新增控件：

- `Main TeX for cloud conversion`
- `Cloud Convert` 按钮

按钮输入：

```text
api-base-url
project-path
main-tex
detected-profile
quality-level
output-path
```

### 2.4 main.rs 接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增：

- `mod cloud_convert`
- `ui.on_cloud_convert_clicked(...)`

行为：

- Cloud Convert 与本地 Convert 分离。
- 点击 Cloud Convert 时保存项目路径、输出路径、profile、quality、API base URL。
- 从 `AppState` 读取 access token。
- 后台线程执行云端转换，避免阻塞 UI。
- 成功后显示：
  - cloud report line
  - job id
  - DOCX 路径和字节数
  - report JSON 路径
- 失败后显示错误并把质量状态设置为 `Cloud failed`。

### 2.5 新增依赖

修改文件：

```text
crates/desktop-slint/Cargo.toml
```

新增：

```toml
zip = { version = "2.2", default-features = false, features = ["deflate"] }
```

用途：

- 桌面端对 TeX 项目目录进行 zip 打包后上传到 `/v1/uploads`。

---

## 三、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint cloud_convert -- --nocapture
cargo check -p doc-desktop-slint
```

结果：

```text
PASS
```

测试项：

| 测试 | 结果 |
---|---|
| `cloud_convert::tests::report_path_uses_docx_stem` | PASS |
| `cloud_convert::tests::explicit_main_tex_is_normalized` | PASS |

说明：

- 当前验证覆盖编译、Slint 回调生成、cloud_convert 辅助逻辑。
- 尚未在 GUI 中做真实点击验收。
- 尚未启动本地 server 通过 UI 进行端到端云转换手工验证。

---

## 四、当前边界与风险

### 4.1 P5 仍未完成

还缺：

- 文件/目录选择器。
- zip/project 拖拽。
- GUI 真实操作验收。
- 任务进度更细粒度展示。
- 云端转换 recent jobs 集成。
- token 安全存储。
- 注册、退出、刷新登录。
- billing checkout/portal UI。

### 4.2 P6/P7 生产化仍未完成

桌面端现在已经能调用 API，但后端仍是 preview：

- demo token。
- in-memory uploads/jobs/usage。
- mpsc 内存队列。
- 无数据库。
- 无对象存储。
- 无 sandbox。
- 无支付 provider。

所以 Cloud Convert MVP 只能证明产品路径，不能作为正式商业发布依据。

---

## 五、下一步建议

建议下一步优先补三项：

1. **本地 server + desktop cloud convert 验收脚本**
   - 启动 `doc-server`
   - 使用 preview auth
   - 上传 `examples/paper3/upload.zip`
   - 创建 conversion
   - 下载 DOCX/report

2. **桌面端 recent jobs 接入云端任务**
   - Cloud Convert 成功/失败写入 `AppState.jobs`
   - Recent Jobs 显示 executor 为 cloud

3. **P7 worker sandbox 方案落地**
   - 当前桌面端已经能触发云端 worker
   - 下一步需要降低公网开放风险
   - 优先实现 per-job 临时目录、timeout、zip slip 防护、禁止 shell escape

完成这些后，P5 可接近“桌面商业闭环 MVP”，P7 可继续向 Beta 级云端 worker 推进。
