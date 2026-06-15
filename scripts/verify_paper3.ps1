# scripts/verify_paper3.ps1
# ------------------------------------------------------------
# 把 examples/paper3/latex/main-jos.tex 编译为 docx 验证脚本。
# 实际工作由 crates/core/tests/paper3_e2e.rs 中的集成测试完成：
#   - 挂载 examples/paper3/latex 目录到内存 VFS
#   - 解析 include 拓扑（处理 \input{...} 的 .tex 自动补全）
#   - 拼接 → Logos+Rowan 解析 → 降级到 semantic-ast
#   - docx-writer 序列化 + zip 打包
#   - 写出到 examples/paper3/output/main-jos.docx
#
# 用法（PowerShell 7+，在仓库根 E:\work\Tex2Doc 下）：
#   pwsh -File scripts/verify_paper3.ps1
# 或：
#   .\scripts\verify_paper3.ps1
#
# 集成 Playwright 视觉验证（可选）：
#   1) 首次：node scripts/verify_install.mjs   （下载 Chromium）
#   2) 任意时：node scripts/verify_paper3.mjs  （读 docx + 关键短语断言 + 截图）
# ------------------------------------------------------------

[CmdletBinding()]
param(
    # 是否在编译后强制重新运行（默认：只要产存在就跳过 cargo test）
    [switch]$Force,
    # 是否在 cargo test 之后跑 Playwright 视觉验证（默认：跑）
    [switch]$SkipPlaywright
)

$ErrorActionPreference = 'Stop'

# 仓库根（脚本位于 <root>/scripts/）
$RepoRoot   = Resolve-Path (Join-Path $PSScriptRoot '..')
$Paper3Dir  = Join-Path $RepoRoot 'examples/paper3'
$ProjectDir = Join-Path $Paper3Dir 'latex'
$OutDir     = Join-Path $Paper3Dir 'output'
$MainTex    = Join-Path $ProjectDir 'main-jos.tex'
$OutDocx    = Join-Path $OutDir 'main-jos.docx'

Write-Host '== Tex2Doc · paper3 端到端验证 ==' -ForegroundColor Cyan
Write-Host "仓库根      : $RepoRoot"
Write-Host "主 tex      : $MainTex"
Write-Host "输出目录    : $OutDir"
Write-Host "目标 docx   : $OutDocx"
Write-Host ''

# 1. 前置检查
if (-not (Test-Path $MainTex)) {
    throw "主 tex 不存在：$MainTex"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "未检测到 cargo，请先安装 Rust 工具链（rustup.rs）"
}

# 2. 跑集成测试
$testArgs = @(
    'test',
    '-p', 'doc-core',
    '--test', 'paper3_e2e',
    '--', '--nocapture'
)
if ($Force) { $testArgs = $testArgs + '--ignored' }

Write-Host "[1/2] 运行 cargo test ..."
Write-Host ("      cargo " + ($testArgs -join ' '))
Write-Host ''

& cargo @testArgs
if ($LASTEXITCODE -ne 0) {
    throw "cargo test 失败，退出码 = $LASTEXITCODE"
}

# 3. 验收产物
Write-Host ''
Write-Host "[2/2] 验收产物 ..."
if (-not (Test-Path $OutDocx)) {
    throw "测试通过但未找到产物：$OutDocx"
}

$info = Get-Item $OutDocx
$size = $info.Length
$header = [System.IO.File]::ReadAllBytes($OutDocx)[0..3]
$isZip  = ($header[0] -eq 0x50 -and $header[1] -eq 0x4B -and $header[2] -eq 0x03 -and $header[3] -eq 0x04)

if (-not $isZip) {
    throw "产物不是合法 zip：$OutDocx"
}

# 看一下 zip 包里有哪些部件
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($OutDocx)
$parts = foreach ($e in $zip.Entries) { '{0,-32} {1,8} bytes' -f $e.FullName, $e.Length }
$zip.Dispose()

Write-Host ''
Write-Host '✅ 转换成功' -ForegroundColor Green
Write-Host ("   产物大小  : {0} bytes" -f $size)
Write-Host ("   产物路径  : {0}" -f $OutDocx)
Write-Host '   内部部件  :'
$parts | ForEach-Object { Write-Host "     $_" }
Write-Host ''
Write-Host '可以用 Word / WPS 打开该 docx 进一步人工核验排版。' -ForegroundColor DarkGray

# 4. Playwright 视觉验证（可选）
if (-not $SkipPlaywright) {
    Write-Host ''
    Write-Host "[3/3] 运行 Playwright 视觉验证 ..." -ForegroundColor Cyan
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        Write-Warning "未检测到 node，跳过 Playwright 验证（请先安装 Node.js 20+）"
    } elseif (-not (Test-Path (Join-Path $RepoRoot 'node_modules\playwright'))) {
        Write-Host "      首次运行：安装 npm 依赖 ..."
        & npm install --no-audit --no-fund
        if ($LASTEXITCODE -ne 0) {
            throw "npm install 失败，退出码 = $LASTEXITCODE"
        }
        & node scripts/verify_install.mjs
        if ($LASTEXITCODE -ne 0) {
            throw "Playwright Chromium 下载失败，退出码 = $LASTEXITCODE"
        }
    }
    & node scripts/verify_paper3.mjs --no-cargo
    if ($LASTEXITCODE -ne 0) {
        throw "Playwright 视觉验证失败，退出码 = $LASTEXITCODE"
    }
    Write-Host ("   截图    : {0}" -f (Join-Path $OutDir 'preview.png'))
    Write-Host ("   报告页  : {0}" -f (Join-Path $OutDir 'report.html'))
}
