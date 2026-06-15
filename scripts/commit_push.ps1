# scripts/commit_push.ps1
# ------------------------------------------------------------
# Tex2Doc 仓库专用：自动 stage → commit → push。
#
# 与 .agent/skills/mygit 配套脚本（p3-microservice 风格）不同，
# 本脚本是 Tex2Doc 独立定制，不依赖 DashScope AI / 任何项目子目录。
#
# 用法（PowerShell 7+，在仓库根 E:\work\Tex2Doc 下）：
#   .\scripts\commit_push.ps1 -Message "fix: 修复 xxx"
#   .\scripts\commit_push.ps1 -Message "feat: 新增 xxx" -Scope latex-reader
#
# 行为契约：
# 1. 若 working tree 干净（无 staged / unstaged / untracked），拒绝执行并退出 0。
# 2. 调 `git add -A`（自动尊重 .gitignore）；
# 3. 用传入的 -Message 调 `git commit --no-verify` 提交（与项目其它提交风格一致）；
# 4. 自动 `git push origin <current-branch>`：
#    - 首次推送自动 --set-upstream；
#    - 推送失败保留本地 commit，由用户决定重试。
# 5. Conventional Commits 风格前缀可选：feat / fix / docs / style / refactor /
#    test / chore / build / ci / perf。
# ------------------------------------------------------------

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Message,

    # 可选：commit 标题中括号里的 scope，如 -Scope latex-reader
    [string]$Scope,

    # 可选：commit 正文（多行用 ; 分隔）
    [string]$Body,

    # 只本地提交，不推送
    [switch]$NoPush,

    # 强制推送到远端（谨慎：可能覆盖远端历史）
    [switch]$ForcePush
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

# 0. 检查 git 仓库
& git rev-parse --git-dir 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "当前目录不是 git 仓库：$RepoRoot"
}

# 1. 检查工作区是否真的有变更
$status = & git status --porcelain
if ([string]::IsNullOrWhiteSpace($status)) {
    Write-Host "✅ 没有变更需要提交。" -ForegroundColor DarkGray
    exit 0
}

# 2. 解析 -Message：第一行作标题
# 兼容 \n / \r\n / 字面 \\n —— 用 Out-String 转 String[] 即可避免 char 切片。
$msgString = [string]$Message
$rawLines = $msgString -split "`r?`n" | ForEach-Object { $_.TrimEnd() }
$nonEmpty = @($rawLines | Where-Object { $_.Length -gt 0 })
if ($nonEmpty.Count -eq 0) {
    throw "-Message 不能为空"
}
$title = $nonEmpty[0]
$inlineBody = ""
if ($nonEmpty.Count -gt 1) {
    $inlineBody = ($nonEmpty[1..($nonEmpty.Count - 1)] -join "`n").Trim()
}

# 给标题加 scope（如果用户给了 -Scope 且标题里没有括号）
if (-not [string]::IsNullOrEmpty($Scope)) {
    if ($title -notmatch '\([^)]+\):') {
        $title = $title -replace '^([a-zA-Z]+)(:)', "`$1($Scope)`$2"
    }
}

# 3. 拼装 commit message
$commitMsg = $title
if (-not [string]::IsNullOrEmpty($Body)) {
    $commitMsg += "`n`n$Body"
}
if (-not [string]::IsNullOrEmpty($inlineBody)) {
    $commitMsg += "`n`n$inlineBody"
}

# 4. git add -A（自动尊重 .gitignore）
Write-Host "== Tex2Doc · 自动 commit & push ==" -ForegroundColor Cyan
Write-Host "工作区变更："
$status -split "`n" | ForEach-Object {
    if ($_ -match '^(.{2})\s+(.+)$') {
        Write-Host ("  {0,-3} {1}" -f $Matches[1], $Matches[2])
    }
}
Write-Host ""
Write-Host "[1/3] git add -A ..."
& git add -A
if ($LASTEXITCODE -ne 0) {
    throw "git add 失败，退出码 = $LASTEXITCODE"
}

# 5. git commit
Write-Host "[2/3] git commit ..."
Write-Host ("  标题: {0}" -f $title)
& git commit --no-verify -m $commitMsg
if ($LASTEXITCODE -ne 0) {
    throw "git commit 失败，退出码 = $LASTEXITCODE"
}

if ($NoPush) {
    Write-Host ""
    Write-Host "✅ 提交完成（-NoPush，跳过推送）" -ForegroundColor Green
    exit 0
}

# 6. git push
Write-Host "[3/3] git push ..."
$branch = (& git rev-parse --abbrev-ref HEAD).Trim()
Write-Host ("  分支: {0}" -f $branch)

# 检测上游是否已配置
$upstream = & git rev-parse --abbrev-ref "$branch@{u}" 2>$null
$pushArgs = @('push', 'origin', $branch)
if ($LASTEXITCODE -ne 0) {
    # 没有上游，首次推送要 --set-upstream
    $pushArgs = @('push', '--set-upstream', 'origin', $branch)
}
if ($ForcePush) {
    $pushArgs = @('push', '--force-with-lease', 'origin', $branch)
}

& git @pushArgs
$pushExit = $LASTEXITCODE
if ($pushExit -ne 0) {
    Write-Warning "git push 失败（exit=$pushExit），本地 commit 已保留。"
    Write-Host ("  重试: git push {0} {1}" -f 'origin', $branch)
    exit $pushExit
}

Write-Host ""
Write-Host "✨ 提交并推送成功！" -ForegroundColor Green
& git log --oneline -1
