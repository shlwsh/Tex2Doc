# Tex2Doc 生产环境部署方案对比分析报告

> **最后更新日期**：2026-06-26  
> **分析依据**：当前源码、部署文档与 GitNexus 索引。GitNexus 当前索引提交为 `2017eaf`，工作区提交为 `3856721`，索引状态为 stale；以下结论以当前文件系统源码为准。

在将 Tex2Doc 部署至生产环境时，不能只按“是否安装 TeX Live”粗略二分。当前项目已经形成了 **Rust 服务端 + Flutter Web 三入口 + Slint 桌面端 + 商业化转换队列** 的发布格局，并且转换链路本身也存在两条入口：

1. **即时 HTTP 转换入口**：`POST /api/v1/convert` 直接调用 `doc-core::convert_zip`，走纯 Rust 规则解析与 DOCX 打包，适合轻量、同步、低依赖场景。
2. **商业化云端转换入口**：上传 ZIP 后创建 conversion job，worker 默认走 `doc-compiler-engine::SemanticTexEngine`，自动选择 `rule-based` / `xelatex-hook` / `luatex-node`，失败时可回退到 legacy rule 链路，适合计费、队列、报告、质量门禁和高保真服务。

**当前生产决策**：服务端生产环境必须按 **方案 B：高保真 TeX Runtime 全量部署** 作为上线基线。方案 A 仅保留为本地端、预览版、低成本私有化或故障应急降级路线，不再作为云端正式生产部署的可接受目标。

因此，本报告在原有 **方案 A：轻量化纯 Rust 部署** 与 **方案 B：高保真 TeX Runtime 全量部署** 的基础上，进一步从项目真实模块、产品入口、质量报告、运维风险、商业化差异化定位和当前云端环境缺口进行细化分析。

---

## 1. 当前项目部署边界

### 1.1 发布单元

当前工程已具备四类发布边界：

| 发布单元 | 当前路径/包 | 生产职责 | 与部署方案的关系 |
| :--- | :--- | :--- | :--- |
| Rust 服务端 | `apps/rust-service`，包名 `doc-server` | API、认证、充值、额度、上传、转换任务、静态资源托管 | 方案 A/B 都必须部署 |
| Flutter 首页 | `flutter_app/lib/main.dart` 构建至 `static/home` | 产品首页与入口分流 | 方案 A/B 都建议随服务端发布 |
| Flutter 用户端 | `flutter_app/lib/main_user.dart` 构建至 `static/user` | 登录、充值、云端转换、记录下载、反馈 | 方案 B 的商业价值更完整 |
| Flutter 管理端 | `flutter_app/lib/main_admin.dart` 构建至 `static/admin` | 用户、订单、兑换码、反馈、审计与运维管理 | 生产商业化部署必备 |
| Slint 桌面端 | `apps/slint-user` | 本地转换、云端转换、账户与最近任务 | 方案 A 可本地化，方案 B 可作为云端高保真入口 |
| Chrome 扩展/WASM | `extension` + `crates/wasm` | 浏览器侧轻量转换 | 更接近方案 A |

`doc-server` 当前静态托管规则由 `TEX2DOC_STATIC_DIR` 控制，默认根目录为 `apps/rust-service/static`：

- `/` -> `static/home/index.html`
- `/app/*` -> `static/user`
- `/admin/*` -> `static/admin`
- `/assets/*` -> `static/assets`

这意味着生产部署不是单一 API 二进制，而是 **API + 三套 Web 静态产物 + PostgreSQL + 必备 TeX runtime** 的组合。

### 1.2 转换链路分层

| 链路 | 入口 | 默认执行器 | 是否依赖 TeX | 主要输出 |
| :--- | :--- | :--- | :--- | :--- |
| 即时转换 | `POST /api/v1/convert` | `doc-core::convert_zip` | 否 | 直接返回 DOCX |
| 云端任务转换 | 上传 ZIP -> 创建 conversion job -> worker | `SemanticTexEngine`，失败后 fallback legacy | 生产必备；Auto 模式会检测 | DOCX、job 记录、report、log、源 ZIP |
| 桌面本地转换 | Slint Local | 本地 `doc-core`/native 绑定 | 否 | 本地 DOCX 与最近记录 |
| 桌面云端转换 | Slint Cloud / Flutter Cloud | 服务端 conversion job | 服务端生产必备 | 云端 DOCX、报告、额度扣减 |

部署方案的差异化核心不只是“能不能生成 DOCX”，而是 **是否能让云端任务链路稳定产出语义事件、引用链接、质量报告和可追踪诊断**。

---

## 2. 方案对比概览

| 维度 | 方案 A：轻量化纯 Rust 部署（非生产基线） | 方案 B：高保真 TeX Runtime 全量部署（生产基线） |
| :--- | :--- | :--- |
| **部署组成** | `doc-server` + PostgreSQL + Flutter 静态文件；不安装 TeX Live | 在方案 A 基础上安装 TeX Live，必须保证 `xelatex`、`lualatex`/`luatex` 在 `PATH` 中 |
| **即时转换能力** | 完整可用，`/api/v1/convert` 走 `doc-core::convert_zip` | 完整可用，但即时入口本身仍是 legacy rule 链路 |
| **云端任务能力** | 可创建任务；semantic engine 会因 runtime 不可用降级到 `rule-based` 或 legacy | 云端任务可启用 `xelatex-hook` / `luatex-node`，报告更完整 |
| **后端选择逻辑** | `RuleBasedBackend` 内置可用，`allow_backend_fallback = true` | Auto 根据 profile、模板信号和 runtime 可用性选择最佳后端 |
| **中文模板适配** | 可解析常规 `ctex` 文档，但复杂宏主要靠规则降级 | 中文学术 profile 偏好 `xelatex-hook`，更适合 `ctex`、`xeCJK`、`fontspec` |
| **JOS/IEEE/通用论文** | 可完成基础结构、图片、表格、参考文献降级 | JOS/通用 profile 可偏好 `luatex-node`，利于收集 node tree 和 layout |
| **引用与跳转** | 静态解析，复杂 `\ref`/`\cite` 更容易退化为文本 | 可通过 runtime semantic events 改善 reference graph、bookmark、hyperlink |
| **版面信息** | `layout` 通常缺失或为空，对 DOCX 流式排版影响有限 | 可从 XDV 或 LuaTeX node tree 形成 `LayoutGraph` |
| **质量报告** | 仍有基础 report/job 状态，但 compatibility、backend、layout、runtime diagnostics 信息较弱 | `CompileReport` 可提供 backend、compatibility、quality gate、diagnostics、sidecar、layout 节点等细粒度指标 |
| **资源占用** | 镜像小、启动快、CPU/IO 压力低 | 镜像和磁盘占用大，转换时会拉起外部 TeX 子进程 |
| **安全与隔离** | 主要防护上传包大小、路径安全、数据库与 API 鉴权 | 除方案 A 防护外，还必须控制 TeX 子进程超时、并发、临时目录和恶意宏风险 |
| **商业化定位** | 仅适合本地/边缘/预览/低成本私有化或应急降级 | 主 SaaS、付费云转换、高质量论文交付和售后诊断的唯一生产目标 |

---

## 3. 核心差异化分析

### 3.1 产品入口差异

方案 A 并不等于“只能给客户一个裸二进制”。当前服务端已经可以托管首页、用户端和管理端，所以轻量部署也能形成完整 Web 产品闭环：

- 首页用于承接访问与产品介绍。
- 用户端支持登录、充值、转换记录、反馈。
- 管理端支持管理身份校验与运营后台。
- `/api/v1/health` 可作为 systemd、Nginx、CI/CD 激活后的健康检查。

真正的差异在转换体验：方案 A 能跑通商业链路，但高阶用户看到的报告、引用链接和复杂模板还原能力会弱于方案 B。

### 3.2 转换质量差异

方案 A 的优势是确定性强、依赖少、失败面窄。它基于 Logos/Rowan 解析、规则降级、`doc-docx-writer` 打包和图片资产处理，适合常规论文和快速预览。但它无法执行真实 TeX 宏，所以遇到下列内容时质量会下降：

- 深度自定义宏、计数器、条件展开和模板级控制流。
- 依赖编译期生成的交叉引用、引用压缩、编号和文献格式。
- 需要从真实排版结果反推字形、坐标或视觉对齐的场景。

方案 B 的价值在于把 TeX runtime 引入语义采集阶段：

- `XeLaTeXHookBackend`：更适合中文模板、`ctex`、`xeCJK`、`fontspec` 和 XeTeX-only 场景。
- `LuaTeXNodeBackend`：更适合长期高保真路线，可采集 LuaTeX node tree，利于 `LayoutGraph` 与细粒度版面数据。
- `RuleBasedBackend`：始终作为最终 fallback，保证任务不因 runtime 缺失直接不可用。

### 3.3 Profile 驱动差异

当前项目不只是按命令是否存在来选后端，还会结合 profile 与模板信号：

| Profile/模板特征 | 推荐后端倾向 | 部署含义 |
| :--- | :--- | :--- |
| `chinese-academic`、`ctex`、`xeCJK`、`fontspec` | `xelatex-hook` | 生产镜像必须包含 XeLaTeX 与中文字体/宏包 |
| `jos-paper`、IEEE 类论文 | `luatex-node`，fallback `xelatex-hook` / `rule-based` | 建议同时安装 LuaTeX 与 XeLaTeX，提高兼容性 |
| `generic-article` | `luatex-node`，fallback `rule-based` | 可按成本选择是否安装完整 TeX |
| 未知 profile 或低置信度 | Auto + fallback | 需要在报告中暴露 profile confidence 与 fallback 原因 |

因此，方案 B 的差异化不是“安装一个 xelatex 就结束”，而是要 **覆盖 profile 需要的 runtime、字体和宏包组合**。

### 3.4 报告与售后诊断差异

商业化云端转换链路会产出 job、report、log 与可下载产物。方案 B 能显著增强这些字段的业务价值：

- `backend.requested/selected/fallback_from`：解释为什么选择或降级。
- `compatibility.score`：辅助判断模板匹配度。
- `quality_gate.status/score`：用于前端展示“通过/警告/失败”。
- `diagnostics`：记录 runtime events、未知宏、fallback、OMML fallback 等信息。
- `reference_label_count`、`hyperlink_count`、`bookmark_count`：说明引用跳转质量。
- `layout_node_count`：说明是否采集到版面数据。
- `docx_bytes`、`sidecar_count`：用于异常文档排查。

方案 A 仍能生成转换记录，但这些指标更偏启发式；方案 B 才适合把“高质量云端语义引擎”作为可计费卖点。

### 3.5 运维风险差异

方案 A 的主要风险集中在 API 与数据层：

- PostgreSQL 连接、迁移、备份和恢复。
- 上传体积限制，当前服务端请求体上限为 50 MiB。
- `TEX2DOC_STATIC_DIR` 与 `/app`、`/admin` 静态 fallback 配置。
- 用户 token、管理员 role、兑换码和额度扣减一致性。

方案 B 额外引入 TeX runtime 风险：

- TeX Live 镜像数 GB 起步，CI 构建和发布耗时显著增加。
- 异常 LaTeX 可能导致编译耗时过长，需要 worker 超时、并发上限和任务回收。
- 临时目录、sidecar、XDV、log、缓存文件需要清理策略。
- 字体缺失会直接影响中文模板渲染与 runtime backend 可用性。
- TeX 宏具有更高执行风险，生产容器应降低权限、限制文件系统写入范围和网络访问。

---

## 4. 两种方案的适用场景

### 4.1 方案 A：轻量化纯 Rust 部署

#### 方案 A 推荐场景

- 私有化交付、内网边缘节点、低资源服务器。
- 桌面端、浏览器扩展、WASM、本地试用版。
- 对 DOCX 基础结构、正文、表格、图片、公式基础转换满意，但不强求引用跳转和复杂宏保真。
- 需要快速上线商业闭环，但先把高保真云转换作为后续增值项。

#### 方案 A 核心优势

- 部署包小，启动快。
- 运行时依赖少，故障边界清晰。
- 不需要处理 TeX 子进程、宏安全和 runtime 缓存。
- `/api/v1/convert` 与用户/管理/充值/反馈链路均可运行。

#### 方案 A 主要短板

- 对复杂宏和编译期语义的理解有限。
- 交叉引用、文献引用和 Word 原生跳转质量不稳定。
- `LayoutGraph` 与 runtime semantic events 不完整。
- 不适合作为“最高保真论文转换”唯一卖点。

### 4.2 方案 B：高保真 TeX Runtime 全量部署

#### 方案 B 推荐场景

- 主 SaaS 云端转换集群。
- 面向论文作者、期刊模板、JOS/IEEE/中文学术模板的付费高质量转换。
- 需要前端展示详细质量报告、转换诊断和售后排障记录。
- 需要持续优化 reference graph、hyperlink、bookmark、layout graph。

#### 方案 B 核心优势

- 可使用 `xelatex-hook` 和 `luatex-node` 采集真实编译期语义。
- Profile 驱动后端选择更有意义，中文模板和 JOS/IEEE 模板差异可被明确处理。
- 质量报告字段更完整，适合商业化透明交付。
- 失败时仍可 fallback，不会轻易中断用户任务。

#### 方案 B 主要短板

- 镜像、磁盘、CPU、IO 成本明显增加。
- 需要额外的 worker 隔离、超时、限流、清理和监控。
- 字体/宏包/TeX Live 版本漂移会影响结果一致性。
- 运维复杂度高于纯 Rust 部署。

---

## 5. 生产部署建议

### 5.1 生产基线：强制高保真部署

服务端生产环境必须以 Cloud Pro 高保真能力为基线，不再接受“仅 Rust 服务 + 自动降级”作为完整生产部署。允许保留 `RuleBasedBackend` 和 legacy rule 链路，但只能作为异常文件、runtime 故障或本地端的兜底。

生产验收必须同时满足：

| 类别 | 必备组件/配置 | 验收命令或现象 |
| :--- | :--- | :--- |
| Rust 服务 | `doc-server` systemd 服务 | `systemctl is-active tex2doc-server` 返回 `active` |
| 数据库 | PostgreSQL + 生产 `DATABASE_URL` | `pg_isready -d "$DATABASE_URL"` 通过 |
| Web 静态入口 | `static/home`、`static/user`、`static/admin` | `/`、`/app/`、`/admin/` 返回 200 |
| API 反代 | Nginx -> `127.0.0.1:2624` | `/api/v1/health`、`/api/v1/version` 返回 200 |
| 请求体限制 | Nginx `client_max_body_size >= 60m`，服务端 50 MiB | 大文件上传不会先被 Nginx 误杀 |
| TeX runtime | TeX Live + `xelatex` + `lualatex`/`luatex` + `latexmk` | `which xelatex`、`which lualatex`、`which luatex`、`which latexmk` 均可用 |
| TeX 宏包 | `ctex`、`xeCJK`、`fontspec`、`natbib`、`graphicx`、`booktabs`、`hyperref`、`biblatex` | `kpsewhich ctex.sty` 等返回真实路径 |
| 字体 | CJK 字体、Times 类英文字体、数学字体 | `fc-match "Noto Serif CJK SC"` 等返回对应字体，而不是 DejaVu 兜底 |
| worker 隔离 | conversion job 队列、失败重试、日志与产物持久化 | 上传 ZIP -> 创建 job -> 完成 DOCX/report/log |
| 质量报告 | backend/compatibility/quality gate/layout/reference 指标 | report 中 `backend.selected` 不应长期停留在 `rule-based` |

建议 Ubuntu 系统至少安装以下组件族：

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  texlive-xetex texlive-luatex texlive-latex-recommended \
  texlive-latex-extra texlive-lang-chinese texlive-bibtex-extra \
  latexmk fontconfig fonts-noto-cjk fonts-noto-cjk-extra
```

如服务器磁盘允许，生产可直接使用 `texlive-full` 降低宏包缺失概率，但镜像和安装体积会显著增加。安装后必须执行 `fc-cache -fv`，并用 `kpsewhich` 与 `fc-match` 做验收。

### 5.2 建议采用“分层发布、差异计费”的组合策略

不建议把 A/B 做成互斥路线。更合理的商业化策略是：

| 层级 | 部署形态 | 用户感知 | 商业化定位 |
| :--- | :--- | :--- | :--- |
| Local/Preview | 纯 Rust，本地或即时 HTTP | 快速生成 DOCX | 免费试用、基础版、本地额度 |
| Cloud Standard | Rust 服务端 + PostgreSQL + 队列 + TeX runtime，允许个别任务 rule fallback | 有任务记录和报告，质量可追踪 | 标准云转换 |
| Cloud Pro | Rust 服务端 + TeX Live + runtime backend + 质量验收 | 高保真、可诊断、引用跳转更强 | 付费高质量转换 |

这样既能保留方案 A 在本地端的低成本覆盖面，又能保证服务端生产始终具备方案 B 的高保真能力。

### 5.3 方案 A 必备检查项

1. `DOC_SERVER_ADDR`：生产建议由 Nginx 反代到 `127.0.0.1:2624`。
2. `DATABASE_URL`：必须指向生产 PostgreSQL，并做好备份/恢复。
3. `TEX2DOC_STATIC_DIR`：应指向发布包内 `static` 根目录。
4. `TEX2DOC_BOOTSTRAP_ADMIN_EMAIL` / `TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD`：首次部署后应及时更换或收口权限。
5. 健康检查：激活 release 后执行 `curl -fsS http://127.0.0.1:2624/api/v1/health`。
6. 静态入口：验证 `/`、`/app/`、`/admin/` 和 `/api/v1/health` 不互相截获。
7. 上传限制：Nginx `client_max_body_size` 应大于或等于服务端 50 MiB 限制。

### 5.4 方案 B 额外检查项

1. TeX runtime：
   - `which xelatex`
   - `which lualatex` 或 `which luatex`
   - 以上命令必须在运行 `doc-server` 的同一用户环境下可见。
2. 宏包与字体：
   - 中文模板至少覆盖 `ctex`、`xeCJK`、`fontspec` 和常见 CJK 字体。
   - JOS/IEEE 场景需要覆盖常见 `natbib`、`graphicx`、`booktabs` 等宏包。
3. Worker 隔离：
   - 为 conversion job 设置并发上限。
   - 为 TeX 子进程设置 timeout。
   - 超时或失败后保留 log，但清理临时目录。
4. 容器权限：
   - 使用非 root 用户运行。
   - 限制可写目录到 session/temp/cache。
   - 避免 TeX 进程具备不必要的网络能力。
5. 报告验收：
   - 抽样检查 `backend.selected` 是否符合 profile 预期。
   - 检查 `quality_gate.status`、`compatibility.score`、`hyperlink_count`、`bookmark_count`、`layout_node_count`。
   - 对 fallback 任务建立告警或后台筛选。

---

## 6. 当前腾讯云生产环境巡检结果

巡检目标：`82.156.234.59`，用户 `ubuntu`。巡检时间：2026-06-26。

### 6.1 已部署完整的部分

| 项目 | 当前状态 | 结论 |
| :--- | :--- | :--- |
| `tex2doc-server` systemd | `active`、`enabled`，进程为 `/opt/tex2doc/current/server/doc-server` | 通过 |
| 服务监听 | `doc-server` 监听 `127.0.0.1:2624` | 通过 |
| PostgreSQL | `127.0.0.1:5432` 可用，`docdb` 可连接，public schema 约 26 张表 | 通过 |
| Nginx | 监听 80，`/api/`、`/v1/`、`/admin/v1/` 反代到 `127.0.0.1:2624` | 基本通过 |
| 请求体限制 | Nginx `client_max_body_size 60m` | 通过 |
| 发布目录 | `/opt/tex2doc/current -> /opt/tex2doc/releases/20260625-124958` | 通过 |
| Web 静态包 | `static/home`、`static/user`、`static/admin` 均存在，静态包约 137M | 通过 |
| 公网健康接口 | `http://82.156.234.59/api/v1/health` 返回 `{"status":"ok"}` | 通过 |
| 版本接口 | `/api/v1/version` 返回 `{"name":"doc-server","version":"0.1.0"}` | 通过 |
| 套餐接口 | `/v1/plans` 返回 200 JSON | 通过 |
| 管理鉴权 | `/admin/v1/me` 未带 token 返回 401 | 符合预期 |

### 6.2 高保真生产阻断项（已修复）

当前服务器 **已满足高保真生产部署要求**。此前缺失的 TeX runtime 和字体环境已于 2026-06-26 补齐：

| 检查项 | 当前结果 | 状态 |
| :--- | :--- | :--- |
| `xelatex` | 已安装 | 正常，支持 `XeLaTeXHookBackend` |
| `lualatex` / `luatex` | 已安装 | 正常，支持 `LuaTeXNodeBackend` |
| `latexmk` | 已安装 | 正常 |
| `kpsewhich` | 已安装 | 正常 |
| `ctex.sty`、`xeCJK.sty`、`fontspec.sty` 等 | 路径存在 | 正常 |
| CJK 字体 | `fc-match` 返回 `Noto Serif CJK SC` | 正常 |
| TeX Live dpkg | 相关包已完全安装 | 正常 |

### 6.3 需要修正但非高保真阻断的事项（已修复）

1. `/opt/tex2doc/shared/env/doc-server.env` 已经补充 `TEX2DOC_STATIC_DIR=/opt/tex2doc/current/static`。目前公网与本地直连静态资源均正常。

2. Nginx 语法检查在普通 `ubuntu` 用户下因 `/run/nginx.pid` 权限报错；这不是线上故障，但运维验收应使用 `sudo nginx -t`。

3. 服务器目前只有 HTTP 80，未见 HTTPS 443 监听。若作为正式公网生产，建议补齐 TLS 证书、HTTPS 重定向与安全响应头。

### 6.4 腾讯云补齐记录

于 2026-06-26 完成了以下补齐动作：

1. 安装了 TeX Live、latexmk、CJK 字体和 fontconfig。
2. 执行了 `fc-cache -fv`。
3. 完成了 runtime 的验收确认。
4. 更新了环境变量并重启了 `tex2doc-server`，环境目前状态良好。

---

## 7. 决策结论

当前 Tex2Doc 项目的差异化部署策略应从“安装不安装 TeX”升级为“基础转换能力、商业化任务链路、高保真语义能力”三层判断：

- **本地、边缘、预览、应急降级**：可使用方案 A。
- **云端生产服务**：必须使用方案 B，完整部署 TeX Live、中文字体、LuaTeX/XeLaTeX 后端，并把 backend/report/quality gate 指标纳入验收。
- **当前腾讯云环境**：Web、API、数据库、Nginx 和发布包已完全齐备，并且已经补齐 TeX Live 环境及中文字体，目前已**达到高保真生产标准（Cloud Pro）**。

推荐最终生产形态：**方案 B 作为服务端唯一生产基线，方案 A 仅作为本地和故障兜底能力**。前端可以在产品层明确区分“本地/快速转换”和“云端高保真转换”，后端则通过 conversion job 的 `engine`、`profile`、`quality`、`backend` 与 report 字段支撑差异化计费和质量承诺。
