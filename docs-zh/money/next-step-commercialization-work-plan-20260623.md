# Tex2Doc 下一步商业化工作计划

**日期**：2026-06-23  
**输出目录**：`docs-zh/money`  
**参考文档**：

- `docs-zh/money/commercialization-promotion-plan-20260622.md`
- `docs-zh/money/p6-p9-cloud-client-implementation-plan-20260623.md`
- `docs-zh/semantic-tex-engine-commercialization-release-gap-design-20260622-021418.md`
- `docs-zh/semantic-tex-engine-commercialization-roadmap-20260622-014918.md`

## 一、当前进展判断

Tex2Doc 目前已经具备商业化 Preview 原型，但还不适合直接公开自助收费。下一步的核心不是继续扩大功能面，而是把已经形成的转换能力、桌面端、云端 API、回归验证和商业推广承接入口收口成可交付、可验证、可支持的受控 PoC。

当前可利用的基础：

| 方向 | 当前进展 | 下一步意义 |
|---|---|---|
| 转换核心 | Semantic TeX Engine、legacy rule path、XeLaTeX Hook、LuaTeX Node、7 类 profile、paper3 三路径验证已形成 | 可作为 PoC 演示和质量基准核心 |
| 桌面端 | Slint 已具备本地转换、云端转换、账号、用量、套餐、checkout/portal、recent jobs、诊断包、更新检查 preview | 可作为首个商业交付载体 |
| 商业 API | preview API 覆盖 auth、usage、plans、billing、upload、conversion、release manifest | 需要从内存/demo 升级到生产 auth/store/ledger |
| 云端 Worker | 已有 in-memory queue + worker + DOCX/report 下载链路 | 需要持久化、队列化、sandbox 和对象存储 |
| 质量验证 | 已有 profile fixture、nightly regression、DOCX ZIP/XML 检查、可选 LibreOffice 打开验证 | 需要变成 PoC/Beta 发布门禁和销售证明 |
| 商业推广 | 已有商业化推广缺口分析、P6-P9 数据库和客户端对接方案 | 需要落地 waitlist、demo 包、触达清单和支持流程 |

当前阶段建议定义为：

```text
内部 Preview -> 受控 PoC 准入
```

短期目标不是“公开发布 Pro”，而是：

```text
用 2 周让 5-10 个合作用户可以在人工支持下完成真实论文试用。
用 4-6 周验证质量、付费意愿、价格和生产化架构。
用 8-12 周再判断是否进入付费 Beta。
```

## 二、下一步总目标

### 2.1 14 天目标

完成受控 PoC 基础包：

1. 有可下载或可交付的 Preview 客户端。
2. 有 3 个 before/after demo 包。
3. 有 waitlist 和线索管理入口。
4. 有真实用户试用操作手册。
5. 有失败样本回收、诊断包和支持流程。
6. 有最小质量门禁：paper3、7 profile fixtures、DOCX openability、conversion stats。

### 2.2 30 天目标

进入邀请制 Beta 准备：

1. PostgreSQL schema 从草案进入可运行 migration。
2. demo auth 替换为 JWT + refresh token hash + Argon2id。
3. usage ledger 和 quota reservation 可用。
4. upload、conversion、artifact 元数据落库。
5. worker 有 zip guard、大小限制、timeout 和基础 sandbox。
6. 桌面端完成 GUI 验收矩阵和 Windows 优先打包预研。

### 2.3 60-90 天目标

判断是否推进付费 Beta：

1. 真实支付或支付沙箱闭环完成。
2. 对象存储、持久化队列、sandbox worker 进入 Beta 可用状态。
3. 每个首期 profile 有足够真实样本和质量趋势。
4. 三平台安装包、签名、更新和回滚路径明确。
5. 支持、监控、隐私、条款和数据删除流程可运行。

## 三、立即执行计划：第 1-2 周

### Week 1：PoC 收口与推广承接

| 编号 | 工作项 | 产出 | 验收标准 |
|---|---|---|---|
| W1-1 | 固定 PoC 支持范围 | `PoC 支持范围说明` | 明确支持 7 profile、文件大小、云端次数、已知不支持项 |
| W1-2 | 制作 3 个 demo 包 | 原始 TeX、输出 DOCX、report、截图、对比说明 | 英文期刊、中文论文、复杂公式/图片各 1 个 |
| W1-3 | 搭建 waitlist 承接 | landing page 或 GitHub Pages 表单、线索表 | 可收集邮箱、身份、模板类型、是否愿意提供样本 |
| W1-4 | 输出 PoC 试用手册 | 10 分钟操作手册、FAQ、失败样本提交说明 | 非研发用户可按手册完成本地或云端试用 |
| W1-5 | 完成桌面端手工验收矩阵 | Windows 优先，Linux/macOS 可先记录风险 | 登录、用量、本地转换、云端转换、下载、report、诊断包 |
| W1-6 | 建立失败分类第一版 | error taxonomy、样本回收模板 | 失败可归类到缺资源、unsupported package、runtime、quota、network 等 |

Week 1 的核心验收：

```text
可以向 10 个目标用户发送试用邀请，并能承接报名、发包、收样本、定位失败。
```

### Week 2：PoC 稳定化与商业技术底座启动

| 编号 | 工作项 | 产出 | 验收标准 |
|---|---|---|---|
| W2-1 | 将 `001_docdb_business_schema.sql` 转为可执行初始化流程 | README 或 migration 脚本 | 本地 PostgreSQL 可创建 users/plans/subscriptions/usage/uploads/jobs/releases |
| W2-2 | 完成 P6 auth/store 技术拆分设计 | crate 拆分与 API 迁移清单 | 明确 `doc-commercial-auth/store/domain` 最小边界 |
| W2-3 | 为 server 加入 zip guard 和上传限制 | 限大小、限文件数、防 zip-slip 方案 | 恶意 zip、过大 zip 被稳定拒绝 |
| W2-4 | 强化 nightly regression 输出 | `conversion_stats.md`、profile 通过率、openability | PoC 发布前可一眼判断样本通过情况 |
| W2-5 | 整理首批触达名单 | 50 个高校/实验室/论文作者/技术编辑目标 | 至少 10 个进入首轮邀请 |
| W2-6 | 输出第一版销售材料 | 一页纸 PDF/Markdown、FAQ、竞品对比 | 能解释为什么不用 pandoc 或手工改 Word |

Week 2 的核心验收：

```text
至少 5 个真实项目进入试用或样本收集，形成 Top 10 质量问题和 Top 5 购买理由。
```

## 四、工程主线计划

### 4.1 P10：Preview 收口

优先级：P0  
周期：1-2 周  
目标：把当前 Preview 变成受控 PoC。

任务：

- 完成桌面端 GUI 手工验收矩阵。
- 补齐 project/zip 拖拽或文件选择的用户路径。
- recent jobs 显示 job id、状态、输出路径、report 路径和错误。
- 诊断包加入 compile report、quality gate、错误码、版本信息。
- `semantic-verify` 最小实现：DOCX ZIP、XML、styles、rels、media、LibreOffice openability。
- nightly regression 输出 profile 维度统计。
- paper3 三路径脚本纳入 PoC 验收。

验收：

```text
桌面端可完成：
登录 -> 用量 -> 选择 paper3 -> 云端转换 -> 下载 DOCX/report -> recent jobs 可见 -> 导出诊断包。
```

### 4.2 P11：账号、订阅、用量生产化

优先级：P0  
周期：2-3 周  
目标：替换 demo token 和内存用量。

任务：

- 将 PostgreSQL schema 接入 server 初始化或 migration。
- 实现 Argon2id 密码哈希。
- 实现 JWT access token。
- 实现 refresh token hash 存储、轮换、撤销。
- 实现 usage event ledger。
- 实现 quota reservation：创建 conversion 预占、成功确认、失败返还。
- 接入 billing provider trait。
- 先用 Stripe test mode 或等价沙箱跑通 checkout/webhook。

验收：

```text
服务重启后用户、session、套餐、用量不丢失。
额度不足稳定返回 402。
失败任务返还额度。
重复 webhook 不重复发放权益。
logout 后 refresh token 不可继续刷新。
```

### 4.3 P12：云端 Worker 生产化与 Sandbox

优先级：P0  
周期：3-4 周  
目标：把内存态 worker 替换为可恢复、可隔离的转换平台。

任务：

- uploads、conversions、artifacts 落库。
- 接入本地对象存储目录，后续平滑切换 S3/MinIO。
- 先采用 Postgres queue，降低 Beta 部署复杂度。
- worker 支持 retry、cancel、timeout、dead letter、过期清理。
- 实现 zip slip 防护、文件数限制、总大小限制、单文件大小限制。
- sandbox runner 第一版：独立 workspace、禁网络、wall-clock timeout、进程/磁盘限制。
- 编译日志脱敏并 artifact 化。

验收：

```text
恶意 zip 不能写出 workspace。
超时任务会失败并清理临时目录。
worker 崩溃后任务可恢复或明确失败。
并发任务不会互相污染输出。
```

### 4.4 P13：质量基准与 Profile 商业化

优先级：P0/P1  
周期：3-5 周  
目标：建立可对外说明的质量边界。

任务：

- 建立真实样本库：minimal、realistic、failure、golden 四类。
- 每个首期 profile 先达到 PoC 样本门槛。
- 完整实现 `semantic-verify`。
- 建立 profile quality dashboard。
- 输出失败分类和用户可读修复建议。
- 对 pandoc / Tex2Doc / 手工改稿做可复现对比。

PoC 样本门槛：

| Profile | PoC 样本数 | Beta 样本数 |
|---|---:|---:|
| jos-paper | 10 | 20 |
| chinese-academic | 15 | 30 |
| tacl | 8 | 15 |
| cvpr | 8 | 15 |
| nature | 5 | 10 |
| springer | 10 | 20 |
| generic/arXiv | 25 | 50 |

验收：

```text
DOCX 实际打开率达到 PoC >= 95%。
所有失败有稳定 error_code 和用户可读 message。
质量指标可按 profile/backend/date 追踪。
```

### 4.5 P14：桌面客户端 Beta 产品化

优先级：P1  
周期：2-4 周  
目标：让 Slint 客户端达到邀请制 Beta 可用水平。

任务：

- Windows MSI/MSIX 优先，其次 macOS DMG/pkg、Linux AppImage/deb。
- token 安全存储在 Windows/macOS/Linux 分别完成验证。
- 登录 session 自动恢复。
- 云端任务进度条和阶段状态。
- 失败诊断可复制、可导出。
- billing checkout/portal 打开系统浏览器。
- updater 下载、校验、安装执行和失败 fallback。

验收：

```text
Windows/macOS/Linux 至少各完成一次：
安装 -> 登录 -> 本地转换 -> 云端转换 -> 下载 -> 查看报告 -> 退出登录 -> 重启恢复 session。
```

## 五、商业推广主线计划

### 5.1 产品定位与材料

| 工作项 | 输出 | 截止 |
|---|---|---|
| ICP 固定 | 研究生/论文作者、课题组、期刊/出版社技术编辑三类画像 | 2 天 |
| 一句话定位 | 把真实 LaTeX 论文项目转换为可编辑 Word DOCX 的桌面与云端工具 | 2 天 |
| 套餐草案 | Free Preview、Pro Desktop、Cloud Credits、Team、Enterprise | 3 天 |
| 竞品对比 | pandoc / Tex2Doc / 手工改稿对比表 | 3 天 |
| 销售一页纸 | 产品价值、适用场景、限制、试用方式、联系方式 | 5 天 |

### 5.2 获客与试用

| 渠道 | 动作 | 指标 |
|---|---|---|
| Waitlist | 表单收集邮箱、身份、模板、样本意愿 | 2 周 50 个线索 |
| GitHub/README | 增加商业 Preview 区、demo GIF、waitlist 链接 | README 可承接外部流量 |
| SEO 内容 | 先写 6 篇长尾大纲 | 2 周完成大纲，4 周开始发布 |
| 学术社区 | 知乎/B站/小红书/Reddit 选 2 个 | 每周 2 条内容 |
| 高校/实验室 | 建 50 个目标名单 | 2 周 10 个有效回复 |
| 期刊/出版社 | 建 20 个技术编辑名单 | 4 周 5 次沟通 |

### 5.3 支持与反馈闭环

| 工作项 | 输出 | 验收 |
|---|---|---|
| Support 通道 | 邮箱、微信群或 GitHub issue 分流规则 | Beta 用户 48 小时内首次响应 |
| 失败样本回收 | 用户授权模板、脱敏流程、样本编号 | 每个失败样本进入质量 backlog |
| 用户访谈 | 问卷和访谈记录模板 | 2 周至少 10 份有效反馈 |
| 质量 Top 10 | 每周质量问题清单 | 研发任务能追踪到 profile/sample/error_code |

## 六、里程碑与准入标准

### 6.1 受控 PoC 准入

满足以下条件即可启动 5-10 个合作用户试用：

- paper3 三路径稳定输出。
- 7 profile fixture 全部通过。
- 桌面端完成登录、用量、云端转换、下载、report、诊断包。
- server 支持基础用量限制和转换任务。
- 每次失败有错误码和 report。
- 有 waitlist、试用手册、支持通道和样本回收流程。

### 6.2 邀请制 Beta 准入

满足以下条件再扩大到 50-100 个用户：

- 生产 auth 初版可用。
- PostgreSQL 持久化用户、session、套餐、用量。
- upload、conversion、artifact 不再依赖纯内存。
- sandbox worker 有基础资源限制。
- Windows Preview 安装包可交付。
- 真实样本库达到 PoC 门槛。

### 6.3 付费 Beta 准入

满足以下条件再开始收费：

- usage ledger 可审计，失败返还策略稳定。
- 支付 provider test/live 路径跑通。
- 支付 webhook 幂等。
- 三平台安装包或至少 Windows/macOS 可稳定安装。
- Beta 样本质量门槛达标。
- 隐私政策、服务条款、退款和数据删除策略上线。

## 七、关键指标

| 阶段 | 指标 | 目标 |
|---|---|---:|
| 受控 PoC | waitlist | 50 |
| 受控 PoC | 有效试用 | 10 |
| 受控 PoC | 真实项目完成转换 | 5-10 |
| 受控 PoC | DOCX 实际打开率 | >= 95% |
| 受控 PoC | 失败可分类率 | >= 90% |
| 邀请制 Beta | 激活试用 | 30 |
| 邀请制 Beta | 明确付费意向 | 5 |
| 邀请制 Beta | DOCX 实际打开率 | >= 98% |
| 付费 Beta | 真实付款或采购流程 | 3-5 |
| 付费 Beta | 首次响应 | < 48 小时 |
| GA | DOCX 实际打开率 | >= 99% |
| GA | 无崩溃转换率 | >= 99.5% |

## 八、风险与决策

| 风险 | 等级 | 决策 |
|---|---|---|
| 云端 TeX 执行安全风险 | 高 | P12 sandbox 前不得公开自助云端上传 |
| 真实模板质量波动 | 高 | 先受控 PoC，试用条件绑定样本回收 |
| 支付/用量不一致 | 中高 | usage ledger 和 quota reservation 先于付费 Beta |
| 三平台安装复杂 | 中 | Windows 优先，macOS/Linux 允许 PoC 阶段手工交付 |
| 推广过早损害口碑 | 中高 | 只面向邀请制用户，明确支持范围和已知限制 |
| 功能横向扩张拖慢商业闭环 | 中高 | 暂缓 Enterprise、模板市场、AI 自动修复宏、在线协作 |

## 九、本周推荐任务清单

| 优先级 | 任务 | 负责人角色 | 截止 |
|---|---|---|---|
| P0 | 输出 PoC 支持范围和试用手册 | 产品/研发 | 2 天 |
| P0 | 制作 3 个 before/after demo 包 | 产品/质量 | 3 天 |
| P0 | 建 waitlist 和线索表 | 前端/运营 | 3 天 |
| P0 | 完成桌面端 GUI 验收矩阵 | 桌面/质量 | 5 天 |
| P0 | 将 nightly regression 输出质量统计 | 质量/研发 | 5 天 |
| P0 | 设计并实现 zip guard 第一版 | 后端/安全 | 5 天 |
| P1 | 把 PostgreSQL schema 转为 migration/init 流程 | 后端 | 1 周 |
| P1 | 写 6 篇 SEO 内容大纲 | 市场 | 1 周 |
| P1 | 建 50 个高校/实验室触达名单 | 销售/运营 | 1 周 |
| P1 | 梳理 Stripe test mode 接入任务 | 后端/商业 | 1 周 |

## 十、最终建议

下一步应把目标从“商业化发布”收窄为“受控 PoC 准入”。这能最大化利用当前已经完成的技术资产，同时避免在质量、云端安全、支付账本和安装分发尚未生产化时过早公开。

建议执行顺序：

```text
1. 先完成 PoC 收口：demo、waitlist、试用手册、GUI 验收、诊断包、质量统计。
2. 同步启动生产底座：PostgreSQL、auth、usage ledger、zip guard、worker timeout。
3. 用真实用户样本验证质量和付费意愿。
4. 通过 PoC 数据决定邀请制 Beta 的发布时间和收费边界。
```

首期推广话术继续聚焦一个清晰承诺：

```text
Tex2Doc 是面向真实 LaTeX 论文项目的 DOCX 转换工作台，
优先解决公式、引用、图片、期刊模板和质量诊断这些学术用户最痛的环节。
```
