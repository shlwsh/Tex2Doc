# Tex2Doc 前端质量展示优化技术方案

> 日期：2026-06-26
> 输出目录：`docs-zh/service`
> 目标：将后端引擎增强（Phase 1-4）的质量能力同步到用户界面

---

## 1. 背景与目标

### 1.1 问题陈述

Tex2Doc 后端已完成 Phase 1-4 质量增强：

| 后端能力 | 当前前端展示 | 差距 |
|---|---|---|
| 六维质量评分 (DimensionScores) | 仅有 compatibility-score | 用户无法感知完整质量 |
| 语义丢失事件 (SemanticLossEvent) | 无展示 | 用户不知问题所在 |
| Word 兼容性状态 | 无展示 | 用户不知文档能否直接投稿 |
| 视觉 diff 摘要 | 无展示 | 用户无法对比效果 |
| OOXML 校验结果 | 无展示 | 用户不知结构是否合规 |
| Style Map 覆盖率 | 无展示 | 用户不知样式映射情况 |
| 详细质量报告下载 | 仅有 Open Report | 报告内容未结构化展示 |

### 1.2 目标

1. **转换结果页**：展示六维评分、语义丢失详情、Word 兼容性
2. **历史记录页**：展示每次转换的质量等级、成功/警告/失败原因
3. **质量报告页**：结构化展示报告内容，支持 PDF 对比图查看
4. **管理后台**：展示质量趋势看板（成功率、失败原因 Top N 等）

---

## 2. 前端改造设计

### 2.1 新增 Slint 类型定义

**修改文件**: `apps/slint-user/src/ui/types.slint`

```slint
// 新增类型
export struct QualityDimensions {
    parse: int,        // 解析完整性 0-100
    semantic: int,     // 语义保真 0-100
    docx: int,         // DOCX 结构 0-100
    visual: int,       // 版面一致 0-100
    editable: int,     // 可编辑性 0-100
    performance: int, // 性能与稳定 0-100
}

export struct SemanticLossItem {
    loss-type: string,     // macro/environment/table/figure/citation/formula
    severity: string,      // blocking/degrading/ignorable
    location: string,      // 文件:行号
    description: string,   // 具体描述
    suggestion: string,    // 修复建议
}

export struct WordCompatibilityInfo {
    status: string,        // passed/warnings/failed/unchecked
    errors: [string],      // 错误列表
    check-method: string,  // word/libreoffice/schema_only
}

export struct QualityReportSummary {
    job-id: string,
    engine-version: string,
    profile: string,
    quality-score: int,           // 0-100 加权总分
    dimension-scores: QualityDimensions,
    word-compatibility: WordCompatibilityInfo,
    blocking-issues: [SemanticLossItem],
    warnings: [SemanticLossItem],
    semantic-loss-events: [SemanticLossItem],
    style-coverage-rate: float,  // 0.0-100.0
    visual-diff-percentage: float,
    created-at: string,
}
```

### 2.2 转换结果弹窗设计

**新增组件**: `apps/slint-user/src/ui/components/quality-result-dialog.slint`

```
┌─────────────────────────────────────────────────────────────┐
│  转换结果                                      [×]        │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 质量评分: 88/100  ● 通过                            │   │
│  │ ████████████████████████░░░░░░░  88%               │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  六维评分                                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                  │
│  │ 解析     │ │ 语义     │ │ 结构     │                  │
│  │  ████ 95 │ │  ████ 86 │ │  ████ 92 │                  │
│  │  解析完整 │ │  13项降级 │ │  合规    │                  │
│  └──────────┘ └──────────┘ └──────────┘                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                  │
│  │ 版面     │ │ 可编辑   │ │ 性能     │                  │
│  │  ████ 80 │ │  ████ 90 │ │  ████ 85 │                  │
│  │  差异2.3% │ │  可直接用 │ │  稳定    │                  │
│  └──────────┘ └──────────┘ └──────────┘                  │
│                                                             │
│  Word 兼容性  ● 通过                                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ ✓ LibreOffice 转换成功                              │   │
│  │ ✓ DOCX 结构校验通过                                 │   │
│  │ ✓ 未检测到自动修复提示                              │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ⚠ 语义丢失 (3 项)                                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ [!] 宏: \smartqed - 部分支持 (降级)                 │   │
│  │     位置: main.tex:45                               │   │
│  │     建议: 移除或替换为标准 LaTeX 命令               │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ [!] 表格: multirow - 部分支持 (降级)                │   │
│  │     位置: tables.tex:23                            │   │
│  │     建议: 手动调整单元格垂直对齐                    │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ [!] 引用: \textcite - 回退为普通文本 (降级)         │   │
│  │     位置: intro.tex:67                             │   │
│  │     建议: 使用 \cite 替代                           │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  样式覆盖率: 95% (19/20 角色已映射)                         │
│  ████████████████████████░░░░░░░░░░░░░░  95%            │
│                                                             │
│  [打开 DOCX]  [查看详细报告]  [导出诊断]                     │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 质量报告详情页

**新增组件**: `apps/slint-user/src/ui/pages/quality-report.slint`

```
┌─────────────────────────────────────────────────────────────┐
│  质量报告: job_abc123                         [← 返回]    │
├─────────────────────────────────────────────────────────────┤
│  基本信息                                                   │
│  ┌────────────┬────────────┬────────────┬────────────┐    │
│  │ Job ID     │ Profile    │ Engine     │ Quality    │    │
│  │ job_abc123 │ jos-paper  │ v1.0.0     │ 88/100    │    │
│  └────────────┴────────────┴────────────┴────────────┘    │
│                                                             │
│  [报告摘要] [语义丢失] [结构校验] [视觉对比] [原始数据]    │
│                                                             │
│  ── 语义丢失 ──                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 类型       │ 严重性 │ 位置        │ 说明            │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ 宏         │ 降级   │ main.tex:45 │ \smartqed     │   │
│  │ 表格       │ 降级   │ tables.tex  │ multirow      │   │
│  │ 引用       │ 降级   │ intro.tex   │ \textcite     │   │
│  │ 公式       │ 可忽略 │ math.tex    │ \cancel       │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ── 结构校验 ──                                            │
│  ✓ relationships 完整                                      │
│  ✓ media 文件引用正确                                       │
│  ✓ styles 引用有效                                          │
│  ✓ numbering 格式正确                                        │
│                                                             │
│  ── 视觉对比 (可选) ──                                      │
│  ┌────────────────────┬────────────────────┐               │
│  │     参考 PDF       │      转换结果     │               │
│  │                    │                    │               │
│  │   [渲染图片]      │   [渲染图片]       │               │
│  │                    │                    │               │
│  └────────────────────┴────────────────────┘               │
│  差异百分比: 2.3%   SSIM: 0.977                           │
│                                                             │
│  [下载报告 PDF]  [下载原始 DOCX]  [提交反馈]                │
└─────────────────────────────────────────────────────────────┘
```

### 2.4 历史记录页增强

**修改文件**: `apps/slint-user/src/ui/pages/history.slint`

```
┌─────────────────────────────────────────────────────────────┐
│  转换记录                                                  │
├─────────────────────────────────────────────────────────────┤
│  [全部] [成功] [警告] [失败]     [导出 CSV] [导出诊断]    │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 状态  │ 质量   │ 输入    │ Profile │ 时间    │ 操作 │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ ● 通过 │ 88    │ main.tex│ jos     │ 10:23   │ 查看 │   │
│  │ ● 通过 │ 92    │ paper2  │ generic │ 09:45   │ 查看 │   │
│  │ ⚠ 警告 │ 76    │ thesis  │ auto    │ 09:12   │ 查看 │   │
│  │ ● 通过 │ 85    │ cvpr    │ cvpr    │ 08:30   │ 查看 │   │
│  │ ✗ 失败 │ --    │ broken  │ auto    │ 08:15   │ 查看 │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  质量趋势 (近 30 天)                                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  █                                                 │   │
│  │  █ █   █                                         │   │
│  │  █ █ █ █ █ █   █                               │   │
│  │  █ █ █ █ █ █ █ █ █ █                           │   │
│  │  ────────────────────────                         │   │
│  │  6/26  6/25  6/24  6/23  6/22  6/21            │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  成功率: 85%  平均质量: 85.3  转换次数: 20               │
└─────────────────────────────────────────────────────────────┘
```

### 2.5 后端 API 集成

**新增 API 端点需求** (`apps/rust-service/src/routes.rs`):

| 端点 | 方法 | 返回 | 用途 |
|---|---|---|---|
| `/api/quality-report/{job_id}` | GET | QualityReportSummary | 获取质量报告摘要 |
| `/api/quality-report/{job_id}/detail` | GET | 完整 JSON | 获取详细报告 |
| `/api/quality-report/{job_id}/visual-diff` | GET | PNG/ZIP | 下载视觉对比图 |
| `/api/quality-trends` | GET | 趋势数据 | 管理后台趋势 |

---

## 3. 技术实现方案

### 3.1 Rust → Slint 数据绑定

**修改文件**: `apps/slint-user/src/ui_bindings/conversion.rs`

```rust
// 新增质量报告类型
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct QualityDimensions {
    pub parse: u8,
    pub semantic: u8,
    pub docx: u8,
    pub visual: u8,
    pub editable: u8,
    pub performance: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SemanticLossItem {
    pub loss_type: String,
    pub severity: String,
    pub location: String,
    pub description: String,
    pub suggestion: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct WordCompatibilityInfo {
    pub status: String,
    pub errors: Vec<String>,
    pub check_method: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct QualityReportSummary {
    pub job_id: String,
    pub engine_version: String,
    pub profile: String,
    pub quality_score: u8,
    pub dimension_scores: QualityDimensions,
    pub word_compatibility: WordCompatibilityInfo,
    pub blocking_issues: Vec<SemanticLossItem>,
    pub warnings: Vec<SemanticLossItem>,
    pub semantic_loss_events: Vec<SemanticLossItem>,
    pub style_coverage_rate: f64,
    pub visual_diff_percentage: f64,
    pub created_at: String,
}

// 暴露给 Slint 的 API
pub fn fetch_quality_report(job_id: &str) -> Result<QualityReportSummary, String> {
    // 调用 /api/quality-report/{job_id}
}

pub fn fetch_quality_trends() -> Result<QualityTrends, String> {
    // 调用 /api/quality-trends
}
```

### 3.2 Slint 组件开发清单

| 组件 | 文件 | 优先级 |
|---|---|---|
| `QualityScoreGauge` | `components/quality-score-gauge.slint` | P0 |
| `DimensionScoreBar` | `components/dimension-score-bar.slint` | P0 |
| `SemanticLossList` | `components/semantic-loss-list.slint` | P0 |
| `WordCompatibilityCard` | `components/word-compatibility-card.slint` | P0 |
| `StyleCoverageIndicator` | `components/style-coverage-indicator.slint` | P1 |
| `VisualDiffViewer` | `components/visual-diff-viewer.slint` | P1 |
| `QualityResultDialog` | `components/quality-result-dialog.slint` | P0 |
| `QualityReportPage` | `pages/quality-report.slint` | P1 |
| `QualityTrendChart` | `components/quality-trend-chart.slint` | P2 |

### 3.3 类型定义更新

**修改文件**: `apps/slint-user/src/ui/types.slint`

```slint
// 新增类型定义
export struct QualityDimensions {
    parse: int,
    semantic: int,
    docx: int,
    visual: int,
    editable: int,
    performance: int,
}

export struct SemanticLossItem {
    loss-type: string,
    severity: string,
    location: string,
    description: string,
    suggestion: string,
}

export struct WordCompatibilityInfo {
    status: string,
    errors: [string],
    check-method: string,
}

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

export struct QualityTrendPoint {
    date: string,
    success-rate: float,
    avg-quality: float,
    conversion-count: int,
}
```

---

## 4. 用户体验设计

### 4.1 转换成功流程

```
1. 用户点击「转换」
2. 显示进度条（上传 → 检测 → 编译 → 质检）
3. 转换完成 → 弹出 QualityResultDialog
4. 展示质量评分（六维雷达图/条形图）
5. 展示 Word 兼容性状态
6. 展示语义丢失列表（如果有）
7. 提供「打开 DOCX」「查看详细报告」「导出诊断」按钮
```

### 4.2 转换失败流程

```
1. 转换失败
2. 弹出对话框，显示失败原因
3. 根据错误码展示修复建议
4. 提供「查看详细报告」按钮
5. 提供「提交反馈」按钮（自动关联 job_id）
```

### 4.3 质量等级定义

| 质量分 | 等级 | 颜色 | 含义 |
|---|---|---|---|
| 90-100 | 优秀 | 绿色 | 可直接投稿 |
| 75-89 | 良好 | 蓝色 | 可使用，有少量降级 |
| 60-74 | 一般 | 黄色 | 需检查后使用 |
| < 60 | 较差 | 红色 | 不建议直接使用 |

---

## 5. 实施计划

### 5.1 Phase 1: 基础展示 (1-2 周)

- [ ] 新增 Slint 类型定义
- [ ] 开发 `QualityScoreGauge` 组件
- [ ] 开发 `DimensionScoreBar` 组件
- [ ] 修改转换结果展示逻辑
- [ ] 后端 API 端点实现

### 5.2 Phase 2: 语义丢失展示 (1 周)

- [ ] 开发 `SemanticLossList` 组件
- [ ] 开发 `WordCompatibilityCard` 组件
- [ ] 集成到转换结果弹窗
- [ ] API 数据绑定

### 5.3 Phase 3: 报告详情页 (1 周)

- [ ] 开发 `QualityReportPage`
- [ ] 实现 Tab 切换（摘要/语义/结构/视觉）
- [ ] 实现报告下载功能

### 5.4 Phase 4: 历史与趋势 (1 周)

- [ ] 增强历史记录列表（质量分列）
- [ ] 开发 `QualityTrendChart`
- [ ] 集成管理后台趋势看板

---

## 6. 技术风险与注意事项

### 6.1 性能考虑

- 质量报告可能较大，考虑懒加载
- 视觉对比图较大，使用缩略图 + 点击放大
- 历史记录列表使用虚拟滚动

### 6.2 国际化

- 所有文本使用 i18n 字符串
- 错误码和严重级别需多语言支持

### 6.3 离线支持

- 本地转换也需生成质量报告
- 报告存储在本地，用户可查看历史

---

## 7. 成功指标

| 指标 | 目标 |
|---|---|
| 用户报告「不知道转换质量如何」投诉 | 下降 80% |
| 用户使用「查看详细报告」功能比例 | > 30% |
| 用户提交反馈关联 job_id 比例 | > 50% |
| 质量报告页跳出率 | < 40% |
