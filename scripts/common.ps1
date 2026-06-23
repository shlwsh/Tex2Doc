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
