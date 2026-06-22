# scripts/mygit.ps1
# ------------------------------------------------------------
# Tex2Doc · mygit 的 Windows (PowerShell) 入口。
#
# 与 scripts/mygit.sh（WSL / Linux）完全等价：同一份 mygit.py，
# 同样的参数契约（argv[1]=workspace, argv[2]=script_dir），
# 同样的 .env.mygit 查找规则（先项目根，再仓库根全局）。
#
# 用法（在 PowerShell 5.1 / 7+ 均可，仓库根 E:\work\Tex2Doc 下）：
#   .\scripts\mygit.ps1
#   .\scripts\mygit.ps1 -TargetDir 'E:\work\Tex2Doc'
#   .\scripts\mygit.ps1 -TargetDir 'E:\work\Tex2Doc' -Python 'D:\Python\Python311\python.exe'
#
# 与 commit_push.ps1 的区别：
#   - commit_push.ps1 是 Tex2Doc 本地、无 AI 的 "add → commit --no-verify → push"；
#   - mygit.ps1 / mygit.sh 走 DashScope 兼容接口（OpenAI chat/completions），
#     由 mygit.py 生成中文 Conventional Commits 提交信息后再 add / commit / push。
# ------------------------------------------------------------

[CmdletBinding()]
param(
    # 可选：目标 git 仓库根；缺省取当前目录
    [string]$TargetDir,

    # 可选：显式指定 Python 解释器；缺省自动探测 (python → py -3 → python3)
    [string]$Python
)

$ErrorActionPreference = 'Stop'

# ------------------------------------------------------------
# 0. 解析目标目录
# ------------------------------------------------------------
if ([string]::IsNullOrWhiteSpace($TargetDir)) {
    $TargetDir = (Get-Location).ProviderPath
}
$TargetDir = (Resolve-Path -LiteralPath $TargetDir).ProviderPath
Set-Location -LiteralPath $TargetDir

# 脚本自身目录（不依赖 PWD，所以从任意目录调用都能找到 mygit.py）
$ScriptDir = $PSScriptRoot
if ([string]::IsNullOrEmpty($ScriptDir)) {
    # PS 5.1 在某些 host 下 $PSScriptRoot 为空，用 $MyInvocation 兜底
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
}

# 仓库根（= scripts 的父目录），用作 .env.mygit 的全局回退
$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $ScriptDir '..')).ProviderPath

# ------------------------------------------------------------
# 1. 校验 git 仓库
# ------------------------------------------------------------
& git rev-parse --git-dir 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 错误: 当前目录不是一个有效的 Git 仓库: $TargetDir" -ForegroundColor Red
    exit 1
}

# ------------------------------------------------------------
# 2. 定位 Python 解释器
# ------------------------------------------------------------
function Resolve-Python {
    param([string]$Override)

    if (-not [string]::IsNullOrWhiteSpace($Override)) {
        if (-not (Test-Path -LiteralPath $Override)) {
            throw "指定的 Python 不存在: $Override"
        }
        return $Override
    }

    # 优先级：python → py -3 → python3
    $candidates = @(
        @{ Cmd = 'python';  Args = @() },
        @{ Cmd = 'py';      Args = @('-3') },
        @{ Cmd = 'python3'; Args = @() }
    )
    foreach ($c in $candidates) {
        $exe = (Get-Command $c.Cmd -ErrorAction SilentlyContinue)
        if ($exe) {
            return @{ Exe = $exe.Source; Args = $c.Args }
        }
    }
    throw "未找到 Python 解释器（请安装 Python 3，或通过 -Python 指定路径）"
}

$py = Resolve-Python -Override $Python
if ($py -is [string]) {
    $pyExe = $py
    $pyArgs = @()
} else {
    $pyExe = $py.Exe
    $pyArgs = $py.Args
}

# 打印探测到的 Python
& $pyExe @pyArgs --version
if ($LASTEXITCODE -ne 0) {
    throw "Python 解释器无法执行: $pyExe"
}

# ------------------------------------------------------------
# 3. 校验 requests（与 mygit.sh 行为一致：缺失则给出友好提示）
# ------------------------------------------------------------
$reqCheck = & $pyExe @pyArgs -c "import requests; print(requests.__version__)" 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 错误: 未安装 Python requests，请执行: $pyExe -m pip install requests" -ForegroundColor Red
    exit 1
}
Write-Host ("✅ Python " + (& $pyExe @pyArgs -c "import sys; print('%d.%d.%d' % sys.version_info[:3])") + " · requests $reqCheck")

# ------------------------------------------------------------
# 4. 校验 .env.mygit（友好提示；实际加载仍由 mygit.py 完成）
#    查找顺序：项目根 .env.mygit → 仓库根 .env.mygit（全局回退）
# ------------------------------------------------------------
$localEnv  = Join-Path $TargetDir '.env.mygit'
$globalEnv = Join-Path $RepoRoot '.env.mygit'
$envTemplate = Join-Path $RepoRoot '.agent\skills\mygit\resources\env.mygit.template'

if (-not (Test-Path -LiteralPath $localEnv)) {
    if (-not (Test-Path -LiteralPath $globalEnv)) {
        Write-Host "❌ 错误: 找不到配置文件 .env.mygit（项目根及仓库根均未找到）" -ForegroundColor Red
        Write-Host "请执行:"
        if (Test-Path -LiteralPath $envTemplate) {
            Write-Host ("  copy {0} {1}" -f $envTemplate, $localEnv)
        } else {
            Write-Host "  在项目根创建 .env.mygit（参考 .agent\skills\mygit\resources\env.mygit.template）"
        }
        Write-Host "  然后填入 DASHSCOPE_API_KEY / DASHSCOPE_BASE_URL / DASHSCOPE_MODEL"
        exit 1
    }
}

# ------------------------------------------------------------
# 5. 调用 mygit.py（与 mygit.sh 末尾的 exec 行为完全一致）
#    参数顺序：workspace, script_dir
# ------------------------------------------------------------
$pyFile = Join-Path $ScriptDir 'mygit.py'
if (-not (Test-Path -LiteralPath $pyFile)) {
    throw "找不到 mygit.py: $pyFile"
}

Write-Host ("== Tex2Doc · mygit (PowerShell) ==") -ForegroundColor Cyan
Write-Host ("  workspace : {0}" -f $TargetDir)
Write-Host ("  scriptDir : {0}" -f $ScriptDir)
Write-Host ("  python    : {0}" -f $pyExe)
Write-Host ""

# 透传所有退出码；mygit.py 内部用 sys.exit 处理成功/失败
& $pyExe @pyArgs $pyFile $TargetDir $ScriptDir
exit $LASTEXITCODE
