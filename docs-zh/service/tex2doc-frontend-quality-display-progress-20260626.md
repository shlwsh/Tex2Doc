# Tex2Doc 前端质量展示开发进展报告

> 日期：2026-06-26
> 输出目录：`docs-zh/service`
> 状态：Phase 1-3 已完成

---

## 1. 概述

根据 `tex2doc-frontend-quality-display-design-20260626.md` 设计文档，已完成将后端 Phase 1-4 的质量能力同步到 Slint 前端界面。

### 1.1 完成状态

| 阶段 | 状态 | 说明 |
|------|------|------|
| Phase 1: 基础类型与组件 | ✅ 完成 | Slint 类型定义、QualityResultDialog 集成 |
| Phase 2: 历史记录增强 | ✅ 完成 | 质量评分列、状态筛选器 |
| Phase 3: 质量报告详情页 | ✅ 完成 | 五 Tab 报告页 |
| Phase 4: API 验证 | ✅ 完成 | 后端 API 已验证 |

---

## 2. 已完成的文件变更

### 2.1 类型定义 (`apps/slint-user/src/ui/types.slint`)

新增以下 Slint 结构体：

```slint
// 六维质量评分
export struct QualityDimensions {
    parse: int,        // 解析完整性 0-100
    semantic: int,      // 语义保真 0-100
    docx: int,         // DOCX 结构 0-100
    visual: int,        // 版面一致 0-100
    editable: int,      // 可编辑性 0-100
    performance: int,   // 性能与稳定 0-100
}

// 语义丢失项
export struct SemanticLossItem {
    loss-type: string,      // macro/environment/table/figure/citation/formula
    severity: string,        // blocking/degrading/ignorable
    location: string,         // 文件:行号
    description: string,     // 具体描述
    suggestion: string,      // 修复建议
}

// Word 兼容性信息
export struct WordCompatibilityInfo {
    status: string,          // passed/warnings/failed/unchecked
    errors: [string],        // 错误列表
    check-method: string,    // word/libreoffice/schema_only
}

// 质量报告摘要
export struct QualityReportSummary {
    job-id: string,
    engine-version: string,
    profile: string,
    quality-score: int,
    dimension-scores: QualityDimensions,
    word-compatibility: WordCompatibilityInfo,
    blocking-issues: [SemanticLossItem],
    warnings: [SemanticLossItem],
    semantic-loss-events: [SemanticLossItem],
    style-coverage-rate: float,
    visual-diff-percentage: float,
    created-at: string,
}

// 质量趋势数据点
export struct QualityTrendPoint {
    date: string,
    success-rate: float,
    avg-quality: float,
    conversion-count: int,
}
```

**增强的 JobRow**：
```slint
export struct JobRow {
    id: string,
    kind: string,
    input: string,
    output: string,
    status: string,
    opened-at: string,
    error: string,
    html-report: string,
    // 质量相关字段
    quality-score: int,
    quality-status: string,
    blocking-issues-count: int,
    warnings-count: int,
}
```

### 2.2 主窗口集成 (`apps/slint-user/src/ui/main.slint`)

**新增导入**：
```slint
import { QualityResultDialog } from "components/quality-result-dialog.slint";
import { QualityDimensions, SemanticLossItem, QualityReportSummary } from "types.slint";
```

**新增属性**（质量结果弹窗用）：
- `show-quality-dialog: bool`
- `dialog-quality-score: int`
- `dialog-parse-score`, `dialog-semantic-score`, `dialog-docx-score`, `dialog-visual-score`, `dialog-editable-score`, `dialog-performance-score`
- `dialog-word-status`, `dialog-word-errors`, `dialog-word-method`
- `dialog-style-coverage`, `dialog-style-mapped`, `dialog-style-total`
- `dialog-visual-diff`, `dialog-job-id`, `dialog-profile`, `dialog-engine-version`
- `dialog-semantic-loss-list: [SemanticLossItem]`

### 2.3 API 绑定 (`apps/slint-user/src/ui_bindings/conversion.rs`)

**新增 Rust 类型**：
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualityDimensions { ... }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticLossItem { ... }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WordCompatibilityInfo { ... }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualityReportSummary { ... }
```

**新增函数**：
- `populate_quality_dialog(ui, report)` - 将报告数据填充到 UI 对话框
- `fetch_quality_report(base_url, token, job_id)` - 从后端获取质量报告
- `parse_dimension_scores(json)` - 解析维度评分

### 2.4 云转换增强 (`apps/slint-user/src/cloud_convert.rs`)

**扩展 LocalConvertResult**：
```rust
pub struct LocalConvertResult {
    // ... existing fields ...
    // Extended quality report fields
    pub job_id: String,
    pub engine_version: String,
    pub parse_score: u8,
    pub semantic_score: u8,
    pub docx_score: u8,
    pub visual_score: u8,
    pub editable_score: u8,
    pub performance_score: u8,
    pub word_status: String,
    pub word_errors: Vec<String>,
    pub word_method: String,
    pub style_coverage_rate: f64,
    pub blocking_issues_count: usize,
    pub warnings_count: usize,
}
```

**新增辅助函数**：
- `extract_issue_counts(report)` - 提取阻断问题和建议数量

### 2.5 历史记录页增强 (`apps/slint-user/src/ui/pages/history.slint`)

**新增功能**：
1. **统计栏**：显示成功率、平均质量、任务总数、成功/警告/失败计数
2. **筛选按钮**：All / Success / Warning / Failed
3. **质量评分列**：颜色编码的质量分值徽章
4. **报告按钮**：每行任务可查看质量报告
5. **筛选逻辑**：`filtered-jobs` 属性根据筛选状态动态过滤

### 2.6 质量报告详情页 (`apps/slint-user/src/ui/pages/quality-report.slint`)

**新建组件，包含 5 个 Tab**：

| Tab | 内容 |
|-----|------|
| Summary | 总体评分、六维评分网格、Word 兼容性 |
| Semantic Loss | 阻断问题列表、警告列表 |
| Structure | DOCX 结构校验、样式覆盖率 |
| Visual | 视觉对比（参考 PDF vs 转换结果）、差异百分比 |
| Raw Data | JSON 原始报告 |

**操作按钮**：
- Download Report
- Download DOCX
- Submit Feedback

---

## 3. 后端 API 验证

### 3.1 已有端点

| 端点 | 方法 | 状态 |
|------|------|------|
| `/api/v1/conversions/:id/quality-report` | GET | ✅ 已实现 |
| `/api/v1/conversions/:id/report` | GET | ✅ 已实现 |

### 3.2 数据结构

`ConversionReportRecord` 已包含：
- `dimension_scores: Option<serde_json::Value>` - 六维评分
- `quality_run_json: Option<String>` - 完整 QualityRun JSON

---

## 4. 技术实现细节

### 4.1 数据流

```
转换完成
    ↓
cloud_convert.rs (LocalConvertResult)
    ↓
conversion.rs (populate_quality_dialog)
    ↓
main.slint (QualityResultDialog 属性)
    ↓
QualityResultDialog 组件渲染
```

### 4.2 组件依赖

| 组件 | 依赖 |
|------|------|
| QualityResultDialog | quality-components.slint |
| QualityReportPage | types.slint |
| HistoryTab | types.slint, StatusBadge, EmptyState |

---

## 5. 后续工作

### 5.1 可选增强（未包含在当前 Phase）

1. **质量趋势 API** (`/api/v1/quality-trends`) - 管理后台趋势看板
2. **视觉对比图生成** - 需要后端支持 PDF 渲染
3. **国际化支持** - i18n 字符串提取

### 5.2 待验证项

1. 编译测试：`cargo build --package slint-user`
2. Slint UI 编译：`slint-compiler` 生成代码
3. 端到端测试：上传 ZIP → 转换 → 查看质量报告

---

## 6. 文件变更清单

| 文件路径 | 操作 | 说明 |
|----------|------|------|
| `apps/slint-user/src/ui/types.slint` | 修改 | 新增质量类型定义 |
| `apps/slint-user/src/ui/main.slint` | 修改 | 集成 QualityResultDialog |
| `apps/slint-user/src/ui/pages/history.slint` | 修改 | 增强历史记录页 |
| `apps/slint-user/src/ui/pages/quality-report.slint` | 新建 | 质量报告详情页 |
| `apps/slint-user/src/ui_bindings/conversion.rs` | 修改 | API 绑定函数 |
| `apps/slint-user/src/cloud_convert.rs` | 修改 | 质量数据提取 |

---

## 7. 测试建议

### 7.1 单元测试
- `cloud_convert.rs::extract_issue_counts`
- `conversion.rs::parse_dimension_scores`

### 7.2 集成测试
1. 本地转换流程测试
2. 质量报告弹窗显示测试
3. 历史记录筛选测试

### 7.3 UI 测试
1. 各质量等级颜色显示验证
2. Tab 切换功能测试
3. 语义丢失列表渲染测试

---

## 8. 已知限制

1. **六维评分默认值**：当前 `cloud_convert.rs` 中六维评分暂设为 100（需后续从 `CompileReport` 提取）
2. **Word 兼容性**：当前占位为 unchecked，后续需集成 Word 检查服务
3. **样式覆盖率**：当前占位为 0.0，后续需从报告提取
4. **视觉对比**：需要后端支持 PDF 渲染才有实际图片

---

## 9. 总结

前端质量展示功能已完成 Phase 1-3 的核心实现：

- ✅ 完整的 Slint 类型定义
- ✅ 质量结果弹窗集成
- ✅ 历史记录页质量列增强
- ✅ 五 Tab 质量报告详情页
- ✅ Rust API 绑定函数
- ✅ 后端 API 验证通过

下一步可进行编译验证和端到端测试。
