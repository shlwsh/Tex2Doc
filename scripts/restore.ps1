param(
    [string]$Database = "docdb",
    [string]$PgUser = "postgres",
    [string]$PgPassword = "postgres",
    [string]$PgHost = "localhost",
    [int]$PgPort = 5432,
    [string]$InputRoot = "database",
    [string]$BackupFile,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. "$PSScriptRoot\common.ps1"

$pgVersion = Invoke-PgVersionLabel -PgHost $PgHost -PgPort $PgPort -PgUser $PgUser -PgPassword $PgPassword
$versionDirName = $pgVersion.Label
$versionDir = Join-Path $InputRoot $versionDirName

if ($BackupFile) {
    $selectedBackup = [System.IO.Path]::GetFullPath($BackupFile)
} else {
    if (-not (Test-Path -LiteralPath $versionDir)) {
        throw "No backup directory for PostgreSQL label '$($pgVersion.Label)' (server_version_num $($pgVersion.VersionNum)): $versionDir"
    }
    $latest = Get-ChildItem -Path $versionDir -Filter "$Database-*.dump" -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $latest) {
        throw "No backup files found for database '$Database' in $versionDir"
    }
    $selectedBackup = $latest.FullName
}

if (-not (Test-Path -LiteralPath $selectedBackup)) {
    throw "Backup file does not exist: $selectedBackup"
}

Write-Host "Restoring PostgreSQL database '$Database' (server_version_num $($pgVersion.VersionNum), label $($pgVersion.Label)) ..."
Write-Host "Input: $selectedBackup"
Write-Host "WARNING: this will terminate active connections, drop '$Database', recreate it, and restore the backup."

if (-not $Force) {
    $confirm = Read-Host "Type RESTORE to continue (or y to force overwrite)"
    if ($confirm -ne "RESTORE" -and $confirm -ne "y") {
        Write-Host "Restore cancelled."
        exit 0
    }
}

$dbLiteral = Escape-SqlLiteral $Database
$dbIdentifier = Escape-SqlIdentifier $Database
$ownerIdentifier = Escape-SqlIdentifier $PgUser

Invoke-Wsl @(
    "env", "PGPASSWORD=$PgPassword",
    "psql", "-h", $PgHost, "-p", "$PgPort", "-U", $PgUser, "-d", "postgres",
    "-c", "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$dbLiteral' AND pid <> pg_backend_pid();"
) | Write-Host

Invoke-Wsl @(
    "env", "PGPASSWORD=$PgPassword",
    "psql", "-h", $PgHost, "-p", "$PgPort", "-U", $PgUser, "-d", "postgres",
    "-c", "DROP DATABASE IF EXISTS $dbIdentifier;"
) | Write-Host

Invoke-Wsl @(
    "env", "PGPASSWORD=$PgPassword",
    "psql", "-h", $PgHost, "-p", "$PgPort", "-U", $PgUser, "-d", "postgres",
    "-c", "CREATE DATABASE $dbIdentifier OWNER $ownerIdentifier;"
) | Write-Host

$wslBackupFile = ConvertTo-WslPath $selectedBackup
Invoke-Wsl @(
    "env", "PGPASSWORD=$PgPassword",
    "pg_restore", "-h", $PgHost, "-p", "$PgPort", "-U", $PgUser,
    "--dbname", $Database,
    "--no-owner", "--verbose",
    $wslBackupFile
) | Write-Host

Write-Host "Restore complete."
