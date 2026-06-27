param(
    [string]$Database = "docdb",
    [string]$PgUser = "postgres",
    [string]$PgPassword = "postgres",
    [string]$PgHost = "localhost",
    [int]$PgPort = 5432,
    [int]$Retain = 2,
    [string]$OutputRoot = "database"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. "$PSScriptRoot\common.ps1"

if ($Retain -lt 1) {
    throw "Retain must be at least 1."
}

$outputRootPath = $OutputRoot
New-Item -ItemType Directory -Force -Path $outputRootPath | Out-Null

$pgVersion = Invoke-PgVersionLabel -PgHost $PgHost -PgPort $PgPort -PgUser $PgUser -PgPassword $PgPassword
$versionDirName = $pgVersion.Label
$versionDir = Join-Path $outputRootPath $versionDirName
New-Item -ItemType Directory -Force -Path $versionDir | Out-Null

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backupFile = Join-Path $versionDir "$Database-$timestamp.dump"
$wslBackupFile = ConvertTo-WslPath $backupFile

Write-Host "Backing up PostgreSQL database '$Database' (server_version_num $($pgVersion.VersionNum), label $($pgVersion.Label)) ..."
Write-Host "Output: $backupFile"

Invoke-Wsl @(
    "env", "PGPASSWORD=$PgPassword",
    "pg_dump", "-h", $PgHost, "-p", "$PgPort", "-U", $PgUser,
    "--format=custom", "--blobs", "--verbose",
    "--file", $wslBackupFile,
    $Database
) | Write-Host

$backups = Get-ChildItem -Path $versionDir -Filter "$Database-*.dump" -File |
    Sort-Object LastWriteTime -Descending

$backups | Select-Object -Skip $Retain | ForEach-Object {
    Write-Host "Removing old backup: $($_.FullName)"
    Remove-Item -LiteralPath $_.FullName -Force
}

Write-Host "Backup complete. Kept latest $Retain backup file(s) in $versionDir"
