# scripts/install_commit_push_hook.ps1
# ------------------------------------------------------------
# 安装一个 git post-commit hook，使得之后任何 commit（手工 `git commit`、
# Cursor / VSCode 提交、其它脚本）都自动 push 到 origin/<current-branch>。
#
# 方案 A：单仓库 config（推荐，与团队成员共享）
#   1. 本脚本会把 .githooks/post-commit 链接为 .git/hooks/post-commit
#   2. 团队成员在克隆后跑一次：
#        .\scripts\install_commit_push_hook.ps1
#      或等价命令：
#        git config core.hooksPath .githooks
#   3. 之后所有 commit 自动 push。
#
# 方案 B：纯本地（不想 commit 钩子到仓库）
#   .\scripts\install_commit_push_hook.ps1 -LocalOnly
#   会在 .git/hooks/post-commit 写一份内嵌副本（不进版本库）。
#
# 卸载：
#   .\scripts\install_commit_push_hook.ps1 -Uninstall
# ------------------------------------------------------------

[CmdletBinding()]
param(
    [switch]$Uninstall,
    # 仅在 .git/hooks/ 写一份内嵌副本，不链接仓库中的 .githooks/
    [switch]$LocalOnly
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$HookDir = Join-Path $RepoRoot '.git/hooks'
$HookFile = Join-Path $HookDir 'post-commit'
$SharedHook = Join-Path $RepoRoot '.githooks/post-commit'

if ($Uninstall) {
    if (Test-Path $HookFile) { Remove-Item $HookFile -Force }
    # 同时把 core.hooksPath 还原到默认
    & git config --unset core.hooksPath 2>$null
    Write-Host "✅ 已卸载 post-commit hook" -ForegroundColor Green
    exit 0
}

if (-not (Test-Path $HookDir)) {
    throw "未找到 .git/hooks 目录：$HookDir（请确认在 git 仓库根目录运行）"
}

# 优先用仓库中版本化的 .githooks/post-commit；找不到时回退到内嵌副本。
$useLocal = $LocalOnly -or -not (Test-Path $SharedHook)

if (-not $useLocal) {
    # 共享模式：把 .githooks/post-commit 复制到 .git/hooks/post-commit
    #   注：符号链接在 Windows + Git-Bash 上可能受限，复制更稳。
    Copy-Item $SharedHook $HookFile -Force
    Write-Host "✅ 已链接共享 hook：.githooks/post-commit -> .git/hooks/post-commit" -ForegroundColor Green
    Write-Host "   团队成员首次 clone 后执行：git config core.hooksPath .githooks" -ForegroundColor DarkGray
} else {
    # 内嵌副本
    $hook = @'
#!/bin/sh
# 自动推送：post-commit 时把当前分支推送到 origin。
# 失败仅警告，不影响 commit 本身。
set -e

BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
if [ -z "$BRANCH" ] || [ "$BRANCH" = "HEAD" ]; then
    exit 0
fi

if ! git rev-parse --abbrev-ref "$BRANCH@{u}" >/dev/null 2>&1; then
    echo "[post-commit] 首次推送，设置 upstream origin/$BRANCH"
    git push --set-upstream origin "$BRANCH" || echo "[post-commit] 推送失败（commit 已保留）"
else
    echo "[post-commit] 自动推送 origin/$BRANCH"
    git push origin "$BRANCH" || echo "[post-commit] 推送失败（commit 已保留）"
fi
'@
    Set-Content -Path $HookFile -Value $hook -Encoding UTF8 -NoNewline
    Write-Host "✅ 已写入内嵌 hook：$HookFile（-LocalOnly）" -ForegroundColor Green
}

# Git hooks 在 Unix 上需要可执行权限
if ($IsLinux -or $IsMacOS) {
    & chmod +x $HookFile
}

Write-Host ""
Write-Host "用法：之后任何 git commit 都会自动 push 到 origin/<branch>。" -ForegroundColor Cyan
Write-Host "      若推送失败，commit 仍保留；脚本会打 warning。" -ForegroundColor DarkGray
Write-Host "      卸载：.\scripts\install_commit_push_hook.ps1 -Uninstall" -ForegroundColor DarkGray
