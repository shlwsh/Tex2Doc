# Common cross-platform utilities for WSL-based PostgreSQL operations
# Dot-source this file from sibling scripts: & "$PSScriptRoot\common.ps1"

function Invoke-Wsl {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & wsl -- @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($exitCode -ne 0) {
        throw ($output | Out-String)
    }
    $output
}

function Get-CleanLine {
    param([string[]]$Lines)
    $Lines |
        Where-Object { $_ -and $_.Trim() -ne "" -and $_ -notmatch "^discover_other_daemon:" } |
        Select-Object -First 1
}

function ConvertTo-WslPath {
    param([Parameter(Mandatory = $true)][string]$Path)
    $full = [System.IO.Path]::GetFullPath($Path)
    if ($full -match "^([A-Za-z]):\\(.*)$") {
        $drive = $Matches[1].ToLowerInvariant()
        $rest = $Matches[2] -replace "\\", "/"
        return "/mnt/$drive/$rest"
    }
    Get-CleanLine (Invoke-Wsl @("wslpath", "-a", $full))
}

function Escape-SqlLiteral {
    param([Parameter(Mandatory = $true)][string]$Value)
    $Value.Replace("'", "''")
}

function Escape-SqlIdentifier {
    param([Parameter(Mandatory = $true)][string]$Value)
    '"' + $Value.Replace('"', '""') + '"'
}

function ConvertTo-PgVersionLabel {
    param([Parameter(Mandatory = $true)][string]$VersionNum)

    if ($VersionNum -notmatch '^\d{4,6}$') {
        throw "Unexpected PostgreSQL server_version_num '$VersionNum' (expected 4-6 digits)."
    }
    $padded = $VersionNum.PadLeft(6, '0')
    $major = [int]$padded.Substring(0, 2)
    $minor = [int]$padded.Substring(2, 2)
    $patch = [int]$padded.Substring(4, 2)
    return "pg${major}.${minor}.${patch}"
}

function Invoke-PgVersionLabel {
    param(
        [Parameter(Mandatory = $true)][string]$PgHost,
        [Parameter(Mandatory = $true)][int]$PgPort,
        [Parameter(Mandatory = $true)][string]$PgUser,
        [Parameter(Mandatory = $true)][string]$PgPassword,
        [string]$AdminDb = "postgres"
    )

    $raw = Get-CleanLine (Invoke-Wsl @(
        "env", "PGPASSWORD=$PgPassword",
        "psql", "-h", $PgHost, "-p", "$PgPort", "-U", $PgUser, "-d", $AdminDb,
        "-Atc", "show server_version_num;"
    ))
    if (-not $raw) {
        throw "Unable to read PostgreSQL server_version_num from WSL."
    }
    $label = ConvertTo-PgVersionLabel $raw
    return [pscustomobject]@{
        VersionNum = $raw
        Label      = $label
    }
}
