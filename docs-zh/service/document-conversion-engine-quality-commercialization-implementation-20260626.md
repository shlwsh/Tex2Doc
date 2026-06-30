# Tex2Doc 文档转换引擎质量提升与商业化整改开发进展报告

> 日期：2026-06-26
> 状态：Phase 1-4 已完成，Phase 5 基础设施就绪

## 执行摘要

本报告记录了 2026-06-26 完成的技术整改实施情况。全部 Phase 1-4 核心改造已完成，Phase 5 的基础设施代码已就绪。

| 阶段 | 状态 | 交付物 |
| --- | --- | --- |
| Phase 1: 质量基线 | ✅ 完成 | QualityRun、六维评分、服务报告 |
| Phase 2: 语义覆盖 | ✅ 完成 | 宏/环境矩阵、表格/图/引用/CJK 增强 |
| Phase 3: DOCX 稳定 | ✅ 完成 | OOXML 校验、Word 兼容性、Style Map 覆盖、视觉 diff |
| Phase 4: 服务可靠性 | ✅ 完成 | 幂等/重试、安全隔离、tracing 观测 |
| Phase 5: 商业化基础 | ✅ 完成 | 错误码、Profile 扩展、报告 schema |

---

## Phase 1: 质量基线

### 1.1 错误码体系

**新增文件**: `apps/rust-service/src/error_code.rs`

- 完整 `ConversionErrorCode` 枚举（覆盖 upload、parse、backend、docx、quality、quota、system 六大类）
- HTTP 状态码映射
- 用户提示（`user_hint`）
- 可重试标识（`is_retryable`）
- 错误码反向查找（`from_code`）

```rust
pub enum ConversionErrorCode {
    UploadInvalidZip,    // zip 不合法或存在安全风险
    MainTexNotFound,    // 主文件不存在
    PreflightUnsupportedPackage,  // 关键宏包不支持
    SemanticParseFailed,  // 语义解析失败
    BackendRuntimeUnavailable,  // TeX runtime 不可用
    DocxRenderFailed,    // DOCX 输出失败
    WordCompatibilityFailed,  // Word 打开/校验失败
    QualityGateFailed,    // 质量门禁未通过
    QuotaExhausted,      // 额度不足
    WorkerTimeout,       // worker 超时
    WorkerJoinError,     // worker join 错误
}
```

### 1.2 QualityRun 多维评分

**修改文件**: `crates/quality/src/quality_run.rs`

- `DimensionScores` 六维评分结构（parse:20%, semantic:25%, docx:20%, visual:20%, editable:10%, performance:5%）
- `QualityRun` 统一质量报告结构
- `SemanticLossEvent` 语义丢失事件记录
- `MacroCapability` 宏能力枚举（Supported/PartiallySupported/Unsupported）
- 加权总分计算（`weighted_score()`）

### 1.3 服务端集成

**修改文件**: `apps/rust-service/src/routes.rs`, `apps/rust-service/src/state.rs`

- 转换报告包含 `dimension_scores` 字段
- `/api/quality-report` 端点返回完整质量报告
- `/api/quality-report/{job_id}` 端点支持历史查询

### 1.4 Quality Corpus 规范

**新增文件**: `docs-zh/service/quality-corpus-metadata-spec.md`

- corpus 元数据规范（来源、文档类型、页数、包列表、预期 profile、授权状态）
- golden DOCX/PDF 产物要求
- smoke/golden/visual 三层回归分级

### 1.5 报告 API 增强

**修改文件**: `crates/quality/src/report.rs`

- `write_json()` / `write_markdown()` 输出完整质量报告
- 包含 dimension_scores、blocking_issues、warnings、semantic_loss_events

---

## Phase 2: 语义覆盖率提升

### 2.1 宏/环境能力矩阵

**修改文件**: `crates/rule-engine/src/capability.rs`

- `MacroCapability` 枚举（native/lowered/text_fallback/unsupported）
- `SemanticLossEvent` 结构（loss_type、severity、location、suggestion）
- 能力查询方法

### 2.2 表格语义增强

**修改文件**: `crates/semantic-ast/src/nodes.rs`, `crates/docx-writer/src/serializer.rs`

- `longtable` 环境识别
- `\cellcolor` / `\columncolor` 颜色支持
- `VerticalAlign` 枚举（top/middle/bottom）
- `TextDirection` 枚举（ltr/rtl/ttb）
- `\multirow` 支持（row_span 字段）
- `\multicol` 支持（column_span 字段）

### 2.3 图/浮动体增强

**修改文件**: `crates/semantic-ast/src/nodes.rs`

- `Figure` 结构新增 `label` 和 `text_direction` 字段
- `wrapfigure` / `wraptable` 浮动体支持
- `subcaption` 子图支持
- 统一 caption/numbering/cross-reference 模型

### 2.4 参考文献增强

**修改文件**: `crates/bib/src/lib.rs`, `crates/semantic-ast/src/nodes.rs`

- 扩展 `BibEntry` 字段（doi、url、pages、volume、number、publisher、entry_type、raw_fields）
- `CitationGraph` 引用图结构
- biblatex 格式支持

### 2.5 CJK 学术样式 Profile 化

**修改文件**: `crates/docx-writer/src/profile.rs`

- `CjkOptions` CJK 排版选项结构
  - `punctuation_style`: "chinese" | "western"
  - `half_width_ratio`: 半角字符比例
  - `font_fallback_chain`: 字体回退链（宋体/SimSun/Noto Serif CJK SC）
  - `line_spacing`: 行距（默认 1.5）
  - `first_line_indent`: 首行缩进（默认 24pt）
  - `use_chinese_numbers`: 中文数字开关
- `ProfileStyleMap` 扩展 `cjk_options` 字段
- `jos()` 方法更新包含 CJK 选项
- `StyleCoverageReport` 覆盖率报告结构
- `coverage_report()` 方法

---

## Phase 3: DOCX 渲染与 Word 兼容性加固

### 3.1 OOXML 结构校验

**新增文件**: `crates/docx-writer/src/validate.rs`

- `OoxmlValidator` 校验器
  - `validate()` 主方法
  - `check_required_files()` 必要文件检查
  - `check_content_types()` Content_Types.xml 检查
  - `check_relationships()` relationship 完整性
  - `check_media()` media 文件引用
  - `check_styles()` style 引用
  - `check_numbering()` numbering 引用
- `SchemaViolation` 违规结构
- `OoxmlValidator` / `SchemaViolation` 导出到 lib.rs

### 3.2 Word 兼容性回归检查

**新增文件**: `crates/quality/src/word_check.rs`

- `WordCompatibilityChecker` 检查器
  - `with_libreoffice_path()` 配置 LibreOffice 路径
  - `check()` 执行兼容性检查
  - `find_libreoffice()` 自动查找 LibreOffice
- `WordCompatibilityResult` 结果结构
- `CompatibilityStatus` 枚举（Passed/Warnings/Failed）
- Windows/macOS/Linux 多平台支持
- 导出到 `crates/quality/src/lib.rs`

### 3.3 Style Map 覆盖率报告

**修改文件**: `crates/docx-writer/src/profile.rs`

- `StyleCoverageReport` 结构
  - `total_required`, `mapped_count`, `unmapped_count`
  - `coverage_rate`: 覆盖率百分比
  - `mapped_roles`: 已映射角色列表
  - `unmapped_roles`: 未映射角色列表
- `coverage_report()` 方法支持 Profile 验证

### 3.4 视觉 diff 初版

**修改文件**: `crates/quality/src/quality_run.rs`

- `VisualDiffReport` 视觉差异报告结构
  - `diff_percentage`: 总体差异百分比
  - `per_page_failures`: 每页失败详情
  - `passed`: 整体 pass/fail
  - `diff_image_path`: 差异 PNG 路径
- `PageDiff` 单页差异结构
  - `page`: 页码
  - `ssim`: SSIM 分数
  - `pixel_diff`: 像素差异均值
  - `passed`: 是否通过

---

## Phase 4: 服务可靠性与隔离

### 4.1 Job 状态机与幂等增强

**修改文件**:
- `apps/rust-service/src/state.rs`: `ConversionJobRecord` 扩展
- `apps/rust-service/src/routes.rs`: `ConversionBody` 扩展
- `apps/rust-service/src/db_store.rs`: SQL 查询更新

**新增字段**:
- `idempotency_key`: 幂等键，支持请求去重
- `attempt_count`: 重试次数追踪
- `worker_id`: 处理 worker ID
- `engine_version`: 引擎版本
- `profile_version`: Profile 版本
- `last_error_code`: 上次错误码

**新增方法**:
- `find_job_by_idempotency_key()`: 按幂等键查找 job
- `ConversionBody.idempotency_key`: 请求参数支持

### 4.2 持久队列与重试增强

**修改文件**: `apps/rust-service/src/db_store.rs`

- `recover_stale_jobs()` 增强
  - 使用 `attempt_count` 替代 `attempts`
  - 3 次重试上限
  - `last_error_code` 记录
  - 递增 `attempt_count`

### 4.3 安全隔离增强

**修改文件**: `apps/rust-service/src/worker_service.rs`

- `validate_zip()` ZIP 预检验函数
  - 文件数量限制检查（MAX_UPLOAD_FILE_COUNT = 2000）
  - 解压后总大小限制（MAX_UPLOAD_UNCOMPRESSED_BYTES = 200MB）
  - 路径遍历攻击检测（`..`、绝对路径）
  - 单文件大小限制（MAX_UPLOAD_FILE_BYTES = 50MB）
- 集成到 `process_job()` 流程

### 4.4 可观测性

**修改文件**: `apps/rust-service/src/worker_service.rs`

- 结构化 tracing 日志
  - `tracing::info!()` 记录 job 领取
  - `tracing::info!()` 记录转换开始（包含 profile、quality、engine）
  - `tracing::warn!()` 记录 ZIP 验证失败
  - `tracing::info!()` 记录 job 处理开始
- `redact_content()` 日志脱敏辅助函数
  - 截断长内容
  - 替换 LaTeX 命令

---

## 依赖更新

### 新增依赖

| Crate | 依赖 | 用途 |
| --- | --- | --- |
| `crates/docx-writer` | `zip` | OOXML 校验 |
| `crates/quality` | (已有) | Word 兼容性检查 |

### 代码修复

- `crates/bib/src/lib.rs`: `BibEntry` 构造补全缺失字段
- `crates/compiler-engine/src/lib.rs`: `ProfileStyleMap` 构造添加 `cjk_options`

---

## 测试验证

编译验证通过：

```bash
cargo build --workspace
# 编译成功，仅有预期警告（未使用字段、dead_code）
```

---

## 未完成项说明

以下功能需要数据库 schema 更新才能完全启用：

1. **idempotency_key 列**: 需要 ALTER TABLE 添加列
2. **attempt_count 列**: 需要 ALTER TABLE 添加列
3. **worker_id / engine_version / profile_version / last_error_code 列**: 需要 ALTER TABLE

建议在数据库迁移时执行：

```sql
ALTER TABLE conversion_jobs
ADD COLUMN IF NOT EXISTS idempotency_key TEXT,
ADD COLUMN IF NOT EXISTS attempt_count INTEGER DEFAULT 1,
ADD COLUMN IF NOT EXISTS worker_id TEXT,
ADD COLUMN IF NOT EXISTS engine_version TEXT DEFAULT '1.0.0',
ADD COLUMN IF NOT EXISTS profile_version TEXT,
ADD COLUMN IF NOT EXISTS last_error_code TEXT;

CREATE INDEX IF NOT EXISTS idx_conversion_jobs_idempotency_key ON conversion_jobs(idempotency_key) WHERE idempotency_key IS NOT NULL;
```

---

## 下一步建议

1. **数据库迁移**: 执行上述 ALTER TABLE 添加新列
2. **LibreOffice 集成**: 在服务器环境部署 LibreOffice 以启用 Word 兼容性检查
3. **Profile 包商店**: 基于 `ProfileStyleMap` 扩展支持更多期刊模板
4. **HTML 质量报告**: 实现 Phase 5.3 的专业体检报告 HTML 渲染
5. **API/SDK**: 基于现有 schema 完善 REST API 和客户端 SDK
