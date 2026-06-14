#!/usr/bin/env pwsh
# scripts/install_commit_push_hook.ps1
# ------------------------------------------------------------
# 安装一个 git post-commit hook，使得之后任何 commit（手工 `git commit`、
# Cursor / VSCode 提交、其它脚本）都自动 push 到 origin/<current-branch>。
#
# 用法（PowerShell 7+，在仓库根 E:\work\Tex2Doc 下）：
#   .\scripts\install_commit_push_hook.ps1
#
# 卸载：
#   .\scripts\install_commit_push_hook.ps1 -Uninstall
# ------------------------------------------------------------

[CmdletBinding()]
param(
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$HookDir = Join-Path $RepoRoot '.git/hooks'
$HookFile = Join-Path $HookDir 'post-commit'

if ($Uninstall) {
    if (Test-Path $HookFile) {
        Remove-Item $HookFile -Force
        Write-Host "✅ 已卸载 post-commit hook" -ForegroundColor Green
    } else {
        Write-Host "未找到 post-commit hook，无需卸载" -ForegroundColor DarkGray
    }
    exit 0
}

if (-not (Test-Path $HookDir)) {
    throw "未找到 .git/hooks 目录：$HookDir"
}

# 写 hook 脚本（自包含 shell 脚本，跨 Unix / Git-Bash / WSL 工作）
$hook = @'
#!/bin/sh
# 自动推送：post-commit 时把当前分支推送到 origin。
# 失败仅警告，不影响 commit 本身。
set -e

BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
if [ -z "$BRANCH" ] || [ "$BRANCH" = "HEAD" ]; then
    exit 0
fi

# 若没有上游则首次推送
if ! git rev-parse --abbrev-ref "$BRANCH@{u}" >/dev/null 2>&1; then
    echo "[post-commit] 首次推送，设置 upstream origin/$BRANCH"
    git push --set-upstream origin "$BRANCH" || echo "[post-commit] 推送失败（commit 已保留）"
else
    echo "[post-commit] 自动推送 origin/$BRANCH"
    git push origin "$BRANCH" || echo "[post-commit] 推送失败（commit 已保留）"
fi
'@

Set-Content -Path $HookFile -Value $hook -Encoding UTF8 -NoNewline

# Git hooks 需要可执行权限（Windows 上 git 通常忽略；Unix/Git-Bash 仍需要）
if ($IsLinux -or $IsMacOS) {
    & chmod +x $HookFile
}

Write-Host "✅ 已安装 post-commit hook：$HookFile" -ForegroundColor Green
Write-Host "   之后所有 commit 都会自动 push 到 origin/<branch>。" -ForegroundColor DarkGray
Write-Host "   卸载：.\scripts\install_commit_push_hook.ps1 -Uninstall" -ForegroundColor DarkGray
