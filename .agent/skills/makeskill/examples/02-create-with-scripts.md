# 示例 2：创建带脚本的技能

## 场景描述

用户需要创建一个包含自动化脚本的全局级技能，用于数据库备份。

## 输入

```text
用户: 创建一个 db-backup 技能，需要包含备份脚本，放在全局级别。
```

## 执行过程

1. **解析参数**：
   - `skill-name`: `db-backup`
   - `scope`: `global`
   - `has-scripts`: `true`
   - `has-examples`: `true`（默认）
   - `has-resources`: `false`（默认）

2. **创建目录结构**：
   ```text
   ~/.gemini/antigravity/skills/db-backup/
   ├── SKILL.md
   ├── examples/
   │   └── 01-full-backup.md
   └── scripts/
       └── backup.sh
   ```

3. **生成文件**：
   - `SKILL.md`：包含脚本调用说明和失败处理逻辑
   - `scripts/backup.sh`：包含 `--help` 选项的备份脚本模板
   - `examples/01-full-backup.md`：完整备份示例

## 预期输出

```text
✅ 技能 db-backup 创建成功！（全局级）

📁 目录结构：
  ~/.gemini/antigravity/skills/db-backup/
  ├── SKILL.md
  ├── examples/
  │   └── 01-full-backup.md
  └── scripts/
      └── backup.sh

⚠️  注意事项：
  - scripts/backup.sh 需要执行权限：chmod +x scripts/backup.sh
  - 请根据实际数据库配置修改脚本参数
```
