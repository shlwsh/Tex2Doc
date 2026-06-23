# scripts/mygit.ps1
# ------------------------------------------------------------
# Tex2Doc - mygit Windows (PowerShell) entry point.
# Same args contract as mygit.sh: argv[1]=workspace, argv[2]=script_dir
# ------------------------------------------------------------

[CmdletBinding()]
param(
    [string]$TargetDir,
    [string]$Python
)

$ErrorActionPreference = "Stop"

# 0. Resolve target directory
if ([string]::IsNullOrWhiteSpace($TargetDir)) {
    $TargetDir = (Get-Location).ProviderPath
}
$TargetDir = (Resolve-Path -LiteralPath $TargetDir).ProviderPath
Set-Location -LiteralPath $TargetDir

$ScriptDir = $PSScriptRoot
if ([string]::IsNullOrEmpty($ScriptDir)) {
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
}

$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $ScriptDir "..")).ProviderPath

# 1. Verify git repo
& git rev-parse --git-dir 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Not a valid Git repository: " -ForegroundColor Red -NoNewline
    Write-Host $TargetDir
    exit 1
}

# 2. Resolve Python interpreter
#
# On Windows, "Get-Command python" often returns a WindowsApps stub launcher that
# can't execute scripts directly.  We work around this by scanning common
# installation directories for real python.exe binaries and verifying each one
# with "python -c pass" (most reliable check that avoids --version edge cases).
#
# Priority order: user-local Python installations (most likely to have project
# dependencies installed), then system-wide, then PATH-based candidates.
function Resolve-Python {
    param([string]$Override)

    if (-not [string]::IsNullOrWhiteSpace($Override)) {
        if (-not (Test-Path -LiteralPath $Override)) {
            throw ("Python not found: " + $Override)
        }
        return $Override
    }

    # Search paths: user installs first, then system
    $searchRoots = @(
        "$env:LOCALAPPDATA\Programs\Python"
        "$env:ProgramFiles\Python"
        "$env:ProgramFiles(x86)\Python"
        "$env:USERPROFILE\AppData\Local\Programs\Python"
    )
    $found = @()

    foreach ($root in $searchRoots) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue | ForEach-Object {
            $exe = Join-Path $_.FullName "python.exe"
            if (Test-Path -LiteralPath $exe) {
                $found += $exe
            }
        }
    }

    # Verify each found python.exe and check it has requests
    foreach ($exe in $found) {
        # Verify python runs at all
        & $exe -c "pass" 2>$null | Out-Null
        if ($LASTEXITCODE -ne 0) { continue }

        # Check requests library (our main dependency)
        & $exe -c "import requests" 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            return @{ Exe = $exe; Args = @() }
        }
    }

    # Fallback: try py launcher (may pick a Python without requests, but gives
    # a better error message than "Python not found")
    $pyLauncher = (Get-Command "py" -ErrorAction SilentlyContinue)
    if ($pyLauncher) {
        & $pyLauncher.Source -3 -c "pass" 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            return @{ Exe = $pyLauncher.Source; Args = @("-3") }
        }
    }

    # Last resort: try Get-Command python but skip WindowsApps stubs
    $pythonCmd = (Get-Command "python" -ErrorAction SilentlyContinue)
    if ($pythonCmd) {
        $src = $pythonCmd.Source
        $parentDir = Split-Path -Parent $src
        if (-not ($parentDir -like "*WindowsApps*")) {
            & $src -c "pass" 2>$null | Out-Null
            if ($LASTEXITCODE -eq 0) {
                return @{ Exe = $src; Args = @() }
            }
        }
    }

    throw "Python not found. Install Python 3 or specify via -Python <path>"
}

$py = Resolve-Python -Override $Python
if ($py -is [string]) {
    $pyExe = $py
    $pyArgs = @()
} else {
    $pyExe = $py.Exe
    $pyArgs = $py.Args
}

# 3. Check requests library (double-check; Resolve-Python already verified this)
$reqCheck = & $pyExe @pyArgs -c "import requests; print(requests.__version__)" 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: requests library not installed." -ForegroundColor Red
    Write-Host ("Run: " + $pyExe + " -m pip install requests")
    exit 1
}

$pyVer = & $pyExe @pyArgs -c "import sys; print('%d.%d.%d' % sys.version_info[:3])" 2>&1
Write-Host ("Python " + $pyVer + " | requests " + $reqCheck)

# 4. Check .env.mygit
$localEnv  = Join-Path $TargetDir ".env.mygit"
$globalEnv = Join-Path $RepoRoot ".env.mygit"
$envTemplate = Join-Path $RepoRoot ".agent\skills\mygit\resources\env.mygit.template"

if (-not (Test-Path -LiteralPath $localEnv)) {
    if (-not (Test-Path -LiteralPath $globalEnv)) {
        Write-Host "ERROR: .env.mygit not found (checked project root and repo root)." -ForegroundColor Red
        Write-Host "Run:"
        if (Test-Path -LiteralPath $envTemplate) {
            Write-Host ("  copy " + $envTemplate + " " + $localEnv)
        } else {
            Write-Host "  Create .env.mygit in project root"
        }
        Write-Host "  Then fill in DASHSCOPE_API_KEY / DASHSCOPE_BASE_URL / DASHSCOPE_MODEL"
        exit 1
    }
}

# 5. Call mygit.py
$pyFile = Join-Path $ScriptDir "mygit.py"
if (-not (Test-Path -LiteralPath $pyFile)) {
    throw ("mygit.py not found: " + $pyFile)
}

Write-Host "== Tex2Doc mygit (PowerShell) ==" -ForegroundColor Cyan
Write-Host ("  workspace : {0}" -f $TargetDir)
Write-Host ("  scriptDir : {0}" -f $ScriptDir)
Write-Host ("  python    : {0}" -f $pyExe)
Write-Host ""

& $pyExe @pyArgs $pyFile $TargetDir $ScriptDir
exit $LASTEXITCODE
