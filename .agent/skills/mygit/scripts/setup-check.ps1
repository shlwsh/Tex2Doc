# .agent/skills/mygit/scripts/setup-check.ps1
# ------------------------------------------------------------
# mygit 环境检查与初始化脚本（Windows / PowerShell 版）
# 用法（在仓库根 E:\work\Tex2Doc 下，PowerShell 5.1 / 7+ 均可）：
#   powershell -ExecutionPolicy Bypass -File .agent\skills\mygit\scripts\setup-check.ps1
#   或：
#   .\.agent\skills\mygit\scripts\setup-check.ps1
#
# 与 setup-check.sh（WSL / Linux）一一对应，输出含义相同。
# ------------------------------------------------------------

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

# 仓库根 = 本脚本的祖父目录
$ScriptDir = $PSScriptRoot
if ([string]::IsNullOrEmpty($ScriptDir)) {
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
}
$SkillDir  = Split-Path -Parent $ScriptDir           # .agent/skills/mygit
$RepoRoot  = (Resolve-Path -LiteralPath (Join-Path $SkillDir '..\..\..')).ProviderPath
Set-Location -LiteralPath $RepoRoot

Write-Host "🔍 mygit 环境检查 (Windows PowerShell)" -ForegroundColor Cyan
Write-Host "================================"

# 1. Python
$pyCmd = $null
$pyExe = $null
foreach ($c in @('python', 'py', 'python3')) {
    $exe = (Get-Command $c -ErrorAction SilentlyContinue)
    if ($exe) { $pyCmd = $c; $pyExe = $exe.Source; break }
}
if ($pyExe) {
    $ver = (& $pyExe --version 2>&1) -join ''
    Write-Host ("✅ Python: {0}  ({1})" -f $ver, $pyExe) -ForegroundColor Green
} else {
    Write-Host "❌ Python 未安装（请安装 Python 3.x 并加入 PATH）" -ForegroundColor Red
    exit 1
}

# 2. requests
$reqVer = (& $pyExe -c "import requests; print(requests.__version__)" 2>&1) -join ''
if ($reqVer -match '^\d+\.\d+\.\d+') {
    Write-Host ("✅ Python requests: 已安装 (v$reqVer)") -ForegroundColor Green
} else {
    Write-Host "⚠️  Python requests: 未安装，请执行: $pyExe -m pip install requests" -ForegroundColor Yellow
}

# 3. Git
$gitExe = (Get-Command git -ErrorAction SilentlyContinue)
if ($gitExe) {
    Write-Host ("✅ Git: {0}" -f (& $gitExe.Source --version)) -ForegroundColor Green
} else {
    Write-Host "❌ Git 未安装" -ForegroundColor Red
    exit 1
}

# 4. Git 仓库
& git rev-parse --git-dir 2>$null | Out-Null
if ($LASTEXITCODE -eq 0) {
    $branch = (& git rev-parse --abbrev-ref HEAD).Trim()
    $remote = (& git remote 2>$null | Select-Object -First 1)
    if ([string]::IsNullOrWhiteSpace($remote)) { $remote = '(未配置)' }
    Write-Host ("✅ Git 仓库: 分支=$branch, 远程=$remote") -ForegroundColor Green
} else {
    Write-Host "❌ 当前目录不是 Git 仓库: $RepoRoot" -ForegroundColor Red
    exit 1
}

# 5. .env.mygit
$envFile = Join-Path $RepoRoot '.env.mygit'
if (Test-Path -LiteralPath $envFile) {
    $hasKey   = (Select-String -Path $envFile -Pattern '^DASHSCOPE_API_KEY=.+' -SimpleMatch:$false -Quiet) -and
                ((Select-String -Path $envFile -Pattern '^DASHSCOPE_API_KEY=(?!.{0}$)' -Quiet))
    $hasUrl   = Select-String -Path $envFile -Pattern '^DASHSCOPE_BASE_URL=.+' -Quiet
    $hasModel = Select-String -Path $envFile -Pattern '^DASHSCOPE_MODEL=.+' -Quiet

    # 重新用更稳健的方式判断（PowerShell -match 一次性）
    $envText = Get-Content -LiteralPath $envFile -Raw
    $keyOk   = $envText -match '^DASHSCOPE_API_KEY=.+$' -and
               -not ($envText -match '^DASHSCOPE_API_KEY=\s*$')
    $urlOk   = $envText -match '^DASHSCOPE_BASE_URL=.+$'
    $modelOk = $envText -match '^DASHSCOPE_MODEL=.+$'

    if ($keyOk -and $urlOk -and $modelOk) {
        $model = ($envText -split "`r?`n" |
                  Where-Object { $_ -match '^DASHSCOPE_MODEL=' } |
                  Select-Object -First 1) -replace '^DASHSCOPE_MODEL=', ''
        Write-Host ("✅ .env.mygit: 配置完整 (模型=$model)") -ForegroundColor Green
    } else {
        Write-Host "⚠️  .env.mygit: 配置不完整，请检查必填字段" -ForegroundColor Yellow
    }
} else {
    Write-Host "❌ .env.mygit 不存在" -ForegroundColor Red
    Write-Host ""
    Write-Host "   请从模板创建配置文件："
    Write-Host "   copy .agent\skills\mygit\resources\env.mygit.template .env.mygit"
    Write-Host "   然后编辑 .env.mygit 填入你的 API 密钥"
    exit 1
}

# 6. 入口脚本
$psEntry = Join-Path $RepoRoot 'scripts\mygit.ps1'
$shEntry = Join-Path $RepoRoot 'scripts\mygit.sh'
$pyFile  = Join-Path $RepoRoot 'scripts\mygit.py'
if (Test-Path -LiteralPath $psEntry) {
    Write-Host "✅ scripts\mygit.ps1: 已就绪（Windows 入口）" -ForegroundColor Green
} else {
    Write-Host "⚠️  scripts\mygit.ps1: 不存在" -ForegroundColor Yellow
}
if (Test-Path -LiteralPath $shEntry) {
    Write-Host "✅ scripts\mygit.sh: 已就绪（WSL / Git Bash 入口）" -ForegroundColor Green
}
if (Test-Path -LiteralPath $pyFile) {
    Write-Host "✅ scripts\mygit.py: 已就绪" -ForegroundColor Green
} else {
    Write-Host "❌ scripts\mygit.py: 不存在" -ForegroundColor Red
}

# 7. Windows Git（mygit.py 已内置 WIN_GIT 路径探测）
$winGit = 'C:\Program Files\Git\cmd\git.exe'
if (Test-Path -LiteralPath $winGit) {
    Write-Host "✅ Windows Git: 已安装（mygit 将复用 Windows 凭据推送）" -ForegroundColor Green
} else {
    Write-Host "⚠️  Windows Git: 未找到，建议在 .env.local 配置 GITHUB_TOKEN" -ForegroundColor Yellow
}

# 8. 代理端口探测（7897 / 7890；WSL 下也常转发这些端口）
$proxyOk = $false
foreach ($port in @(7897, 7890, 10809, 1080)) {
    try {
        $tnc = Test-NetConnection -ComputerName '127.0.0.1' -Port $port -InformationLevel Quiet -WarningAction SilentlyContinue
        if ($tnc) {
            Write-Host ("✅ 本地代理: 127.0.0.1:$port 可达") -ForegroundColor Green
            $proxyOk = $true
            break
        }
    } catch {
        # Test-NetConnection 在 PS 5.1 不可用时抛错，静默继续
    }
}
if (-not $proxyOk) {
    Write-Host "⚠️  本地代理: 127.0.0.1:7897/7890 不可达（AI 调用可能失败）" -ForegroundColor Yellow
}

# 9. Bun（可选）
$bun = (Get-Command bun -ErrorAction SilentlyContinue)
if ($bun) {
    $bunVer = (& bun --version 2>&1) -join ''
    Write-Host ("✅ bun: $bunVer") -ForegroundColor Green
} else {
    Write-Host "ℹ️  bun 未安装（可选，直接 .\scripts\mygit.ps1 即可）" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "================================" -ForegroundColor Cyan
Write-Host "✨ 检查完成！推荐: .\scripts\mygit.ps1" -ForegroundColor Green
Write-Host "   或: .\scripts\mygit.sh (WSL / Git Bash)"
