# 示例 1：创建简单技能

## 场景描述

用户需要创建一个用于代码审查的项目级技能，不需要脚本和资源文件。

## 输入

```text
用户: 帮我创建一个 skill，名称是 code-review，用于自动审查代码质量。
```

## 执行过程

1. **解析参数**：
   - `skill-name`: `code-review`
   - `scope`: `project`（默认）
   - `has-scripts`: `false`（默认）
   - `has-examples`: `true`（默认）
   - `has-resources`: `false`（默认）

2. **创建目录结构**：
   ```text
   .agent/skills/code-review/
   ├── SKILL.md
   └── examples/
       ├── 01-normal-review.md
       └── 02-no-issues.md
   ```

3. **生成 SKILL.md**：包含完整的 YAML Frontmatter、Goal、Instructions、Examples、Constraints

## 预期输出

```text
✅ 技能 code-review 创建成功！

📁 目录结构：
  .agent/skills/code-review/
  ├── SKILL.md
  └── examples/
      ├── 01-normal-review.md
      └── 02-no-issues.md

📝 后续步骤：
  1. 检查 SKILL.md 内容是否符合预期
  2. 根据实际需求补充 examples/ 中的示例
  3. 在对话中使用 @code-review 触发技能
```
