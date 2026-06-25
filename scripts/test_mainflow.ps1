param(
    [string]$PostgresAdminUrl = $(if ($env:TEST_MAINFLOW_POSTGRES_URL) { $env:TEST_MAINFLOW_POSTGRES_URL } else { "postgres://postgres:postgres@127.0.0.1:5432/postgres" }),
    [int]$ServerPort = 0,
    [switch]$SkipSlint,
    [switch]$KeepDatabase
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Root = (Resolve-Path (Join-Path $ScriptDir "..")).Path
$RunId = (Get-Date -Format "yyyyMMddHHmmss") + "_" + (Get-Random -Minimum 1000 -Maximum 9999)
$DbName = "tex2doc_mainflow_$RunId".ToLowerInvariant()
$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "tex2doc-mainflow-$RunId"
$TempRustBin = Join-Path $Root "apps\slint-user\src\bin\test_mainflow_slint_tmp.rs"
$Paper3Zip = Join-Path $Root "examples\paper3\upload.zip"
$Paper3MainTex = "main-jos.tex"
$ServerProcess = $null
$CreatedDatabase = $false
$FinalStatus = "FAILED"
$CleanupNotes = [System.Collections.Generic.List[string]]::new()

function Write-Step([string]$Message) {
    Write-Host "[test_mainflow] $Message"
}

function Find-CommandPath([string]$Name, [string[]]$Fallbacks = @()) {
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }
    foreach ($path in $Fallbacks) {
        if (Test-Path $path) {
            return $path
        }
    }
    throw "Required command not found: $Name"
}

function Get-FreePort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse("127.0.0.1"), 0)
    $listener.Start()
    try {
        return [int]$listener.LocalEndpoint.Port
    }
    finally {
        $listener.Stop()
    }
}

function Set-DatabaseNameInUrl([string]$Url, [string]$DatabaseName) {
    $query = ""
    $base = $Url
    $queryIndex = $Url.IndexOf("?")
    if ($queryIndex -ge 0) {
        $query = $Url.Substring($queryIndex)
        $base = $Url.Substring(0, $queryIndex)
    }
    $slash = $base.LastIndexOf("/")
    if ($slash -lt 0) {
        throw "Invalid PostgreSQL URL: $Url"
    }
    return $base.Substring(0, $slash + 1) + $DatabaseName + $query
}

function Invoke-External([string]$Label, [scriptblock]$Command) {
    Write-Step $Label
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

function Wait-ServerReady([string]$HealthUrl, [System.Diagnostics.Process]$Process) {
    $deadline = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $deadline) {
        if ($Process.HasExited) {
            throw "doc-server exited before becoming ready, exit code $($Process.ExitCode)"
        }
        try {
            $resp = Invoke-RestMethod -Uri $HealthUrl -TimeoutSec 2
            if ($resp.status -eq "ok") {
                return
            }
        }
        catch {
            Start-Sleep -Milliseconds 500
        }
    }
    throw "doc-server did not become ready within 60 seconds: $HealthUrl"
}

function Get-JsonLine([string[]]$Lines, [string]$Prefix) {
    $line = $Lines | Where-Object { $_.StartsWith($Prefix) } | Select-Object -Last 1
    if (-not $line) {
        throw "Missing result line with prefix $Prefix"
    }
    return ($line.Substring($Prefix.Length) | ConvertFrom-Json)
}

function Invoke-NativeCapture([scriptblock]$Command) {
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        return @(& $Command 2>&1 | ForEach-Object { "$_" })
    }
    finally {
        $ErrorActionPreference = $oldPreference
    }
}

function Write-CapturedOutput([string]$Source, [string[]]$Lines) {
    foreach ($line in $Lines) {
        if ($line.StartsWith("WEB_FLOW_JSON:") -or $line.StartsWith("SLINT_FLOW_JSON:") -or $line.StartsWith("REDEEM_CODE:")) {
            continue
        }
        if (-not [string]::IsNullOrWhiteSpace($line)) {
            Write-Host "[$Source] $line"
        }
    }
}

function New-RedeemCode([string]$BaseUrl, [string]$AdminEmail, [string]$AdminPassword, [string]$Note) {
    $py = @'
import json
import os
import requests

base = os.environ["MAINFLOW_BASE"]
admin_email = os.environ["MAINFLOW_ADMIN_EMAIL"]
admin_password = os.environ["MAINFLOW_ADMIN_PASSWORD"]
note = os.environ["MAINFLOW_REDEEM_NOTE"]

def trace(message):
    print(f"REDEEM_HELPER: {message}", flush=True)

session = requests.Session()
trace("login admin for redeem batch")
resp = session.post(
    f"{base}/auth/login",
    json={"email": admin_email, "password": admin_password},
    timeout=30,
)
resp.raise_for_status()
token = resp.json()["access_token"]
trace("create count_10 redeem batch")
resp = session.post(
    base.replace("/v1", "/admin/v1") + "/redeem-code-batches",
    headers={"Authorization": f"Bearer {token}"},
    json={
        "package_id": "count_10",
        "quantity": 1,
        "channel": "test-mainflow",
        "note": note,
    },
    timeout=30,
)
resp.raise_for_status()
batch = resp.json()
codes = batch.get("codes") or []
if not codes:
    raise SystemExit("admin batch response did not contain codes")
trace("redeem code generated")
print("REDEEM_CODE:" + codes[0])
'@
    $helperPath = Join-Path $TempRoot "new_redeem_code.py"
    Set-Content -Path $helperPath -Value $py -Encoding UTF8
    $env:MAINFLOW_BASE = $BaseUrl
    $env:MAINFLOW_ADMIN_EMAIL = $AdminEmail
    $env:MAINFLOW_ADMIN_PASSWORD = $AdminPassword
    $env:MAINFLOW_REDEEM_NOTE = $Note
    $output = Invoke-NativeCapture { & $Python $helperPath }
    if ($LASTEXITCODE -ne 0) {
        throw "creating redeem code failed:`n$($output -join [Environment]::NewLine)"
    }
    Write-CapturedOutput "redeem-helper" $output
    $line = $output | Where-Object { $_.StartsWith("REDEEM_CODE:") } | Select-Object -Last 1
    if (-not $line) {
        throw "creating redeem code returned no code:`n$($output -join [Environment]::NewLine)"
    }
    return $line.Substring("REDEEM_CODE:".Length)
}

$Cargo = $null
$Python = $null
$Psql = $null

try {
    New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
    $Cargo = Find-CommandPath "cargo" @("$env:USERPROFILE\.cargo\bin\cargo.exe")
    $Python = Find-CommandPath "python" @("D:\Python\Python311\python.exe")
    $Psql = Find-CommandPath "psql" @("D:\Program Files\PostgreSQL\18\bin\psql.exe", "C:\Program Files\PostgreSQL\18\bin\psql.exe")
    if (-not (Test-Path $Paper3Zip)) {
        throw "Required test upload file not found: $Paper3Zip"
    }

    if ($ServerPort -eq 0) {
        $ServerPort = Get-FreePort
    }

    $DatabaseUrl = Set-DatabaseNameInUrl $PostgresAdminUrl $DbName
    $BaseUrl = "http://127.0.0.1:$ServerPort/v1"
    $HealthUrl = "http://127.0.0.1:$ServerPort/api/v1/health"
    $AdminEmail = "mainflow-admin-$RunId@example.com"
    $AdminPassword = "mainflow-admin-pass-123456"
    $WebEmail = "mainflow-web-$RunId@example.com"
    $SlintEmail = "mainflow-slint-$RunId@example.com"
    $UserPassword = "123456"

    Write-Step "root: $Root"
    Write-Step "test upload: $Paper3Zip"
    Write-Step "temp database: $DbName"
    Write-Step "temp server: $BaseUrl"

    Invoke-External "creating temp PostgreSQL database" {
        & $Psql $PostgresAdminUrl "-v" "ON_ERROR_STOP=1" "-c" "CREATE DATABASE $DbName;"
    }
    $CreatedDatabase = $true
    Write-Step "temp PostgreSQL database created"

    Push-Location $Root
    try {
        Invoke-External "building doc-server" {
            & $Cargo build -p doc-server
        }
    }
    finally {
        Pop-Location
    }

    $ServerExe = Join-Path $Root "target\debug\doc-server.exe"
    if (-not (Test-Path $ServerExe)) {
        $ServerExe = Join-Path $Root "target\debug\doc-server"
    }
    if (-not (Test-Path $ServerExe)) {
        throw "doc-server binary not found under target/debug"
    }

    Write-Step "starting isolated doc-server"
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $ServerExe
    $psi.WorkingDirectory = $TempRoot
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.Environment["DATABASE_URL"] = $DatabaseUrl
    $psi.Environment["DOC_SERVER_ADDR"] = "127.0.0.1:$ServerPort"
    $psi.Environment["TEX2DOC_BOOTSTRAP_ADMIN_EMAIL"] = $AdminEmail
    $psi.Environment["TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD"] = $AdminPassword
    $psi.Environment["TEX2DOC_STATIC_DIR"] = Join-Path $Root "apps\rust-service\static"
    $ServerProcess = [System.Diagnostics.Process]::Start($psi)
    Wait-ServerReady $HealthUrl $ServerProcess
    Write-Step "doc-server ready: $HealthUrl"

    $pythonFlow = @'
import json
import os
import time

import requests

base = os.environ["MAINFLOW_BASE"]
admin_email = os.environ["MAINFLOW_ADMIN_EMAIL"]
admin_password = os.environ["MAINFLOW_ADMIN_PASSWORD"]
user_email = os.environ["MAINFLOW_WEB_EMAIL"]
user_password = os.environ["MAINFLOW_USER_PASSWORD"]
paper3_zip = os.environ["MAINFLOW_PAPER3_ZIP"]
paper3_main_tex = os.environ["MAINFLOW_PAPER3_MAIN_TEX"]

def trace(message):
    print(f"WEB_FLOW: {message}", flush=True)

def require(condition, message):
    if not condition:
        raise AssertionError(message)

def auth_headers(token):
    return {"Authorization": f"Bearer {token}"}

s = requests.Session()

trace(f"register user {user_email}")
register = s.post(
    f"{base}/auth/register",
    json={"email": user_email, "password": user_password, "display_name": "Mainflow Web"},
    timeout=30,
)
register.raise_for_status()

trace("login user and fetch access token")
login = s.post(f"{base}/auth/login", json={"email": user_email, "password": user_password}, timeout=30)
login.raise_for_status()
token = login.json()["access_token"]

trace("login admin for redeem batch creation")
admin_login = s.post(f"{base}/auth/login", json={"email": admin_email, "password": admin_password}, timeout=30)
admin_login.raise_for_status()
admin_token = admin_login.json()["access_token"]

trace("verify initial balance is 0")
usage0 = s.get(f"{base}/usage", headers=auth_headers(token), timeout=30).json()
require(int(usage0.get("count_balance", 0)) == 0, f"initial count_balance should be 0: {usage0}")

trace("create count_10 redeem code batch")
batch = s.post(
    base.replace("/v1", "/admin/v1") + "/redeem-code-batches",
    headers=auth_headers(admin_token),
    json={
        "package_id": "count_10",
        "quantity": 1,
        "channel": "test-mainflow",
        "note": "web flow",
    },
    timeout=30,
)
batch.raise_for_status()
batch_json = batch.json()
codes = batch_json.get("codes") or []
require(codes, "redeem batch returned no codes")
code = codes[0]
trace(f"redeem code acquired: {code[:8]}...")

trace("redeem count_10 code")
redeem = s.post(f"{base}/redeem-codes/redeem", headers=auth_headers(token), json={"code": code}, timeout=30)
redeem.raise_for_status()
redeem_json = redeem.json()
require(int(redeem_json.get("quantity", 0)) == 10, f"redeem quantity should be 10: {redeem_json}")

trace("verify balance after redeem is 10")
usage1 = s.get(f"{base}/usage", headers=auth_headers(token), timeout=30).json()
require(int(usage1.get("count_balance", 0)) == 10, f"balance after redeem should be 10: {usage1}")

with open(paper3_zip, "rb") as f:
    zip_bytes = f.read()
trace(f"upload paper3 package {paper3_zip} ({len(zip_bytes)} bytes)")
upload = s.post(
    f"{base}/uploads",
    headers=auth_headers(token),
    files={"file": ("upload.zip", zip_bytes, "application/zip")},
    timeout=30,
)
upload.raise_for_status()
upload_id = upload.json()["upload_id"]
trace(f"upload stored: {upload_id}")

trace(f"create conversion with main_tex={paper3_main_tex}")
created = s.post(
    f"{base}/conversions",
    headers=auth_headers(token),
    json={"upload_id": upload_id, "main_tex": paper3_main_tex, "profile": "auto", "quality": "standard"},
    timeout=30,
)
created.raise_for_status()
job_id = created.json()["job_id"]
trace(f"conversion job created: {job_id}")

job = None
last_status = None
for attempt in range(120):
    job_resp = s.get(f"{base}/conversions/{job_id}", headers=auth_headers(token), timeout=30)
    job_resp.raise_for_status()
    job = job_resp.json()
    status = job.get("status")
    if status != last_status or attempt % 10 == 0:
        trace(f"poll conversion job={job_id} status={status} docx_ready={job.get('docx_ready')} report_ready={job.get('report_ready')}")
        last_status = status
    if job.get("status") == "completed" and job.get("docx_ready"):
        break
    if job.get("status") in ("failed", "expired"):
        raise AssertionError(f"conversion failed: {job}")
    time.sleep(0.5)
else:
    raise AssertionError(f"conversion timed out: {job}")

trace("download DOCX and verify file signature")
docx = s.get(f"{base}/conversions/{job_id}/download/docx", headers=auth_headers(token), timeout=30)
docx.raise_for_status()
docx_bytes = docx.content
require(docx_bytes.startswith(b"PK"), "downloaded DOCX is not a zip/docx file")
require(len(docx_bytes) > 1000, f"downloaded DOCX is too small: {len(docx_bytes)}")
trace(f"DOCX verified: {len(docx_bytes)} bytes")

trace("verify conversion record")
records = s.get(f"{base}/conversions", headers=auth_headers(token), timeout=30).json()
require(any(item.get("job_id") == job_id and item.get("status") == "completed" for item in records), "conversion record missing")

trace("verify recharge and redeem records")
recharges = s.get(f"{base}/recharges", headers=auth_headers(token), timeout=30).json()
redeems = s.get(f"{base}/redeem-codes/records", headers=auth_headers(token), timeout=30).json()
require(any(item.get("package_id") == "count_10" and int(item.get("quantity", 0)) == 10 for item in recharges), "recharge record missing")
require(any(item.get("package_id") == "count_10" and item.get("status") == "redeemed" for item in redeems), "redeem record missing")

trace("verify final balance is 9")
usage2 = s.get(f"{base}/usage", headers=auth_headers(token), timeout=30).json()
require(int(usage2.get("count_balance", -1)) == 9, f"balance after conversion should be 9: {usage2}")
trace("Web/API flow checks completed")

result = {
    "user": user_email,
    "redeem_code_preview": code[:8] + "...",
    "job_id": job_id,
    "docx_bytes": len(docx_bytes),
    "balance_before": int(usage0.get("count_balance", 0)),
    "balance_after_redeem": int(usage1.get("count_balance", 0)),
    "balance_after_conversion": int(usage2.get("count_balance", 0)),
    "cloud_conversions_used": int(usage2.get("cloud_conversions_used", 0)),
    "conversion_records": len(records),
    "recharge_records": len(recharges),
    "redeem_records": len(redeems),
}
print("WEB_FLOW_JSON:" + json.dumps(result, ensure_ascii=True, sort_keys=True))
'@
    $pythonFlowPath = Join-Path $TempRoot "web_flow.py"
    Set-Content -Path $pythonFlowPath -Value $pythonFlow -Encoding UTF8

    Write-Step "running Web/API main flow"
    $env:MAINFLOW_BASE = $BaseUrl
    $env:MAINFLOW_ADMIN_EMAIL = $AdminEmail
    $env:MAINFLOW_ADMIN_PASSWORD = $AdminPassword
    $env:MAINFLOW_WEB_EMAIL = $WebEmail
    $env:MAINFLOW_USER_PASSWORD = $UserPassword
    $env:MAINFLOW_PAPER3_ZIP = $Paper3Zip
    $env:MAINFLOW_PAPER3_MAIN_TEX = $Paper3MainTex
    $webOutput = Invoke-NativeCapture { & $Python $pythonFlowPath }
    if ($LASTEXITCODE -ne 0) {
        throw "Web/API main flow failed:`n$($webOutput -join [Environment]::NewLine)"
    }
    Write-CapturedOutput "web" $webOutput
    $WebResult = Get-JsonLine $webOutput "WEB_FLOW_JSON:"

    $SlintResult = $null
    if (-not $SkipSlint) {
        Write-Step "creating Slint redeem code"
        $SlintRedeemCode = New-RedeemCode $BaseUrl $AdminEmail $AdminPassword "slint flow"
        $SlintRedeemPreview = if ($SlintRedeemCode.Length -gt 8) { $SlintRedeemCode.Substring(0, 8) + "..." } else { $SlintRedeemCode }
        Write-Step "Slint redeem code ready: $SlintRedeemPreview"
        $binDir = Split-Path -Parent $TempRustBin
        New-Item -ItemType Directory -Force -Path $binDir | Out-Null
        $rustSource = @'
#![allow(dead_code)]

#[path = "../cloud_account.rs"]
mod cloud_account;
#[path = "../cloud_convert.rs"]
mod cloud_convert;

use std::env;
use std::fs;
use std::path::PathBuf;

fn require(condition: bool, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string().into())
    }
}

fn trace(message: impl AsRef<str>) {
    println!("SLINT_FLOW: {}", message.as_ref());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = env::var("MAINFLOW_BASE")?;
    let email = env::var("MAINFLOW_SLINT_EMAIL")?;
    let password = env::var("MAINFLOW_USER_PASSWORD")?;
    let redeem_code = env::var("MAINFLOW_SLINT_REDEEM_CODE")?;
    let paper3_zip = PathBuf::from(env::var("MAINFLOW_PAPER3_ZIP")?);
    let paper3_main_tex = env::var("MAINFLOW_PAPER3_MAIN_TEX")?;
    let temp_root = PathBuf::from(env::var("MAINFLOW_TEMP_ROOT")?).join("slint-project");
    let output_docx = temp_root.join("output").join("result.docx");

    let _ = fs::remove_dir_all(&temp_root);
    require(paper3_zip.is_file(), "paper3 upload.zip is missing")?;
    let paper3_bytes = fs::metadata(&paper3_zip)?.len();
    trace(format!(
        "using paper3 package {} ({} bytes), main_tex={}",
        paper3_zip.display(),
        paper3_bytes,
        paper3_main_tex
    ));

    trace(format!("register user {}", email));
    let registered = cloud_account::register_and_fetch_usage_blocking(&base_url, &email, &password)?;
    require(registered.usage.count_balance == 0, "initial Slint count_balance should be 0")?;

    trace("login user and verify initial balance");
    let session = cloud_account::login_and_fetch_usage_blocking(&base_url, &email, &password)?;
    require(session.usage.count_balance == 0, "Slint login usage should start at 0")?;

    trace("redeem count_10 code");
    let (_redeemed, usage_after_redeem) = cloud_account::redeem_code_blocking(
        &base_url,
        Some(session.access_token.clone()),
        &redeem_code,
    )?;
    require(usage_after_redeem.count_balance == 10, "Slint balance after redeem should be 10")?;
    trace("balance after redeem verified: 10");

    trace("start cloud conversion through Slint adapter");
    let converted = cloud_convert::convert_project_blocking(
        &base_url,
        Some(session.access_token.clone()),
        &paper3_zip,
        Some(&paper3_main_tex),
        &output_docx,
        "auto",
        "standard",
    )?;
    require(converted.docx_bytes > 1000, "Slint DOCX is too small")?;
    let docx = fs::read(&output_docx)?;
    require(docx.starts_with(b"PK"), "Slint DOCX is not a zip/docx file")?;
    trace(format!(
        "conversion completed: job={}, docx_bytes={}",
        converted.job_id, converted.docx_bytes
    ));

    trace("verify final balance is 9");
    let usage_after_conversion =
        cloud_account::fetch_usage_blocking(&base_url, &session.access_token)?;
    require(
        usage_after_conversion.count_balance == 9,
        "Slint balance after conversion should be 9",
    )?;

    trace("verify recharge/redeem table rows");
    let recharge_rows =
        cloud_account::fetch_recharge_table_blocking(&base_url, Some(session.access_token.clone()))?;
    require(
        recharge_rows.iter().any(|row| row.package.contains("count_10") && row.quantity == "10"),
        "Slint recharge/redeem table is missing the count_10 record",
    )?;

    trace("verify conversion table storage metadata");
    let conversion_rows =
        cloud_account::fetch_conversion_table_blocking(&base_url, Some(session.access_token.clone()))?;
    require(
        conversion_rows
            .iter()
            .any(|row| row.id == converted.job_id && row.has_docx && row.has_zip && row.has_log),
        "Slint conversion table is missing completed storage metadata",
    )?;
    trace("Slint flow checks completed");

    println!(
        "SLINT_FLOW_JSON:{}",
        serde_json::json!({
            "user": email,
            "job_id": converted.job_id,
            "docx_bytes": converted.docx_bytes,
            "balance_before": registered.usage.count_balance,
            "balance_after_redeem": usage_after_redeem.count_balance,
            "balance_after_conversion": usage_after_conversion.count_balance,
            "cloud_conversions_used": usage_after_conversion.cloud_conversions_used,
            "recharge_rows": recharge_rows.len(),
            "conversion_rows": conversion_rows.len()
        })
    );

    let _ = fs::remove_dir_all(&temp_root);
    Ok(())
}
'@
        Set-Content -Path $TempRustBin -Value $rustSource -Encoding UTF8

        Write-Step "running Slint client main flow"
        Push-Location $Root
        try {
            $env:MAINFLOW_BASE = $BaseUrl
            $env:MAINFLOW_SLINT_EMAIL = $SlintEmail
            $env:MAINFLOW_USER_PASSWORD = $UserPassword
            $env:MAINFLOW_SLINT_REDEEM_CODE = $SlintRedeemCode
            $env:MAINFLOW_TEMP_ROOT = $TempRoot
            $env:MAINFLOW_PAPER3_ZIP = $Paper3Zip
            $env:MAINFLOW_PAPER3_MAIN_TEX = $Paper3MainTex
            $slintOutput = Invoke-NativeCapture { & $Cargo run -p doc-desktop-slint --bin test_mainflow_slint_tmp }
            if ($LASTEXITCODE -ne 0) {
                throw "Slint main flow failed:`n$($slintOutput -join [Environment]::NewLine)"
            }
            Write-CapturedOutput "slint" $slintOutput
            $SlintResult = Get-JsonLine $slintOutput "SLINT_FLOW_JSON:"
        }
        finally {
            Pop-Location
        }
    }

    $FinalStatus = "PASSED"
    Write-Host ""
    Write-Host "MAINFLOW TEST PASSED"
    Write-Host "Web/API:"
    Write-Host "  user: $($WebResult.user)"
    Write-Host "  job: $($WebResult.job_id)"
    Write-Host "  docx_bytes: $($WebResult.docx_bytes)"
    Write-Host "  balance: $($WebResult.balance_before) -> $($WebResult.balance_after_redeem) -> $($WebResult.balance_after_conversion)"
    Write-Host "  records: conversions=$($WebResult.conversion_records), recharges=$($WebResult.recharge_records), redeems=$($WebResult.redeem_records)"
    if ($SlintResult) {
        Write-Host "Slint:"
        Write-Host "  user: $($SlintResult.user)"
        Write-Host "  job: $($SlintResult.job_id)"
        Write-Host "  docx_bytes: $($SlintResult.docx_bytes)"
        Write-Host "  balance: $($SlintResult.balance_before) -> $($SlintResult.balance_after_redeem) -> $($SlintResult.balance_after_conversion)"
        Write-Host "  rows: conversions=$($SlintResult.conversion_rows), recharges=$($SlintResult.recharge_rows)"
    }
    else {
        Write-Host "Slint: skipped"
    }
}
catch {
    Write-Host ""
    Write-Host "MAINFLOW TEST FAILED"
    Write-Host $_.Exception.Message
    exit 1
}
finally {
    if ($ServerProcess -and -not $ServerProcess.HasExited) {
        try {
            Stop-Process -Id $ServerProcess.Id -Force -ErrorAction Stop
            $CleanupNotes.Add("stopped doc-server pid $($ServerProcess.Id)")
        }
        catch {
            $CleanupNotes.Add("failed to stop doc-server pid $($ServerProcess.Id): $($_.Exception.Message)")
        }
    }

    if (Test-Path $TempRustBin) {
        try {
            Remove-Item -LiteralPath $TempRustBin -Force
            $CleanupNotes.Add("removed temp Slint bin")
        }
        catch {
            $CleanupNotes.Add("failed to remove temp Slint bin: $($_.Exception.Message)")
        }
    }

    if ($CreatedDatabase -and -not $KeepDatabase) {
        try {
            & $Psql $PostgresAdminUrl "-v" "ON_ERROR_STOP=1" "-c" "DROP DATABASE IF EXISTS $DbName WITH (FORCE);" | Out-Null
            if ($LASTEXITCODE -eq 0) {
                $verifyOutput = Invoke-NativeCapture {
                    & $Psql $PostgresAdminUrl "-tA" "-c" "SELECT 1 FROM pg_database WHERE datname = '$DbName';"
                }
                if ($LASTEXITCODE -eq 0 -and [string]::IsNullOrWhiteSpace(($verifyOutput -join ""))) {
                    $CleanupNotes.Add("dropped temp database $DbName and verified it is absent")
                }
                else {
                    $CleanupNotes.Add("drop command completed, but database absence verification was inconclusive for $DbName")
                }
            }
            else {
                $CleanupNotes.Add("failed to drop temp database $DbName, psql exit code $LASTEXITCODE")
            }
        }
        catch {
            $CleanupNotes.Add("failed to drop temp database ${DbName}: $($_.Exception.Message)")
        }
    }
    elseif ($CreatedDatabase -and $KeepDatabase) {
        $CleanupNotes.Add("kept temp database $DbName by request")
    }

    if (Test-Path $TempRoot) {
        try {
            Remove-Item -LiteralPath $TempRoot -Recurse -Force
            $CleanupNotes.Add("removed temp files")
        }
        catch {
            $CleanupNotes.Add("failed to remove temp files: $($_.Exception.Message)")
        }
    }

    Write-Host ""
    Write-Host "Cleanup:"
    foreach ($note in $CleanupNotes) {
        Write-Host "  - $note"
    }
    Write-Host "Final result: $FinalStatus"
}
