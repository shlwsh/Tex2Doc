# Shared helpers for Windows build scripts.
$ErrorActionPreference = "Stop"

function Get-RepoRoot {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ScriptDirectory
    )

    return (Resolve-Path -LiteralPath (Join-Path $ScriptDirectory "..")).Path
}

function Get-TimeStamp {
    return Get-Date -Format "yyyyMMdd-HHmmss"
}

function Get-VersionNumber {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Version
    )

    return ($Version.Trim() -replace '^[vV]', '')
}

function Get-VersionTag {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Version
    )

    $number = Get-VersionNumber -Version $Version
    return "v$number"
}

function Assert-Command {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [string]$Hint = ""
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        if ([string]::IsNullOrWhiteSpace($Hint)) {
            throw "missing dependency: $Name"
        }
        throw "missing dependency: $Name ($Hint)"
    }
}

function Assert-PathExists {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [string]$Kind = "path"
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "missing $Kind`: $Path"
    }
}

function Get-PythonLauncher {
    foreach ($name in @("python", "python3")) {
        $command = Get-Command $name -ErrorAction SilentlyContinue
        if ($command) {
            return @{
                Command = $command.Source
                Prefix = @()
            }
        }
    }

    $py = Get-Command "py" -ErrorAction SilentlyContinue
    if ($py) {
        return @{
            Command = $py.Source
            Prefix = @("-3")
        }
    }

    throw "missing dependency: python or py -3"
}

function Invoke-Python {
    param(
        [string[]]$Arguments = @()
    )

    $launcher = Get-PythonLauncher
    $allArgs = @()
    $allArgs += $launcher.Prefix
    $allArgs += $Arguments
    & $launcher.Command @allArgs
    if ($LASTEXITCODE -ne 0) {
        throw "python failed with exit code $LASTEXITCODE"
    }
}

function Test-PythonModule {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ImportStatement
    )

    try {
        Invoke-Python -Arguments @("-c", $ImportStatement) | Out-Null
        return $true
    } catch {
        return $false
    }
}

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory = ""
    )

    if ([string]::IsNullOrWhiteSpace($WorkingDirectory)) {
        & $FilePath @Arguments
    } else {
        Push-Location $WorkingDirectory
        try {
            & $FilePath @Arguments
        } finally {
            Pop-Location
        }
    }

    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE"
    }
}

function Get-FileSizeBytes {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    return (Get-Item -LiteralPath $Path).Length
}

function Format-ByteSize {
    param(
        [Parameter(Mandatory = $true)]
        [long]$Bytes
    )

    if ($Bytes -ge 1GB) {
        return "{0:N2} GiB" -f ($Bytes / 1GB)
    }
    if ($Bytes -ge 1MB) {
        return "{0:N2} MiB" -f ($Bytes / 1MB)
    }
    if ($Bytes -ge 1KB) {
        return "{0:N2} KiB" -f ($Bytes / 1KB)
    }
    return "$Bytes bytes"
}

function Get-RepoRelativePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $rootFull = [System.IO.Path]::GetFullPath($Root)
    if (-not $rootFull.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $rootFull += [System.IO.Path]::DirectorySeparatorChar
    }

    $pathFull = [System.IO.Path]::GetFullPath($Path)
    $rootUri = New-Object System.Uri($rootFull)
    $pathUri = New-Object System.Uri($pathFull)
    $relative = [System.Uri]::UnescapeDataString($rootUri.MakeRelativeUri($pathUri).ToString())
    return ($relative -replace '\\', '/')
}

function Write-Utf8Lines {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string[]]$Lines
    )

    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllLines($Path, $Lines, $utf8NoBom)
}

function Get-LatestFile {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Directory,
        [Parameter(Mandatory = $true)]
        [string]$Filter,
        [datetime]$After
    )

    if (-not (Test-Path -LiteralPath $Directory)) {
        return $null
    }

    $items = Get-ChildItem -LiteralPath $Directory -File -Filter $Filter -ErrorAction SilentlyContinue
    if ($After) {
        $items = $items | Where-Object { $_.LastWriteTime -ge $After }
    }

    $latest = $items | Sort-Object LastWriteTimeUtc | Select-Object -Last 1
    if ($latest) {
        return $latest.FullName
    }
    return $null
}

function Get-DocEnginePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root
    )

    return Join-Path $Root "target\release\doc-engine.exe"
}

function Get-DocxMediaCount {
    param(
        [Parameter(Mandatory = $true)]
        [string]$DocxPath
    )

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::OpenRead($DocxPath)
    try {
        return @($zip.Entries | Where-Object { $_.FullName -like "word/media/*" }).Count
    } finally {
        $zip.Dispose()
    }
}

function New-Paper3UploadZip {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Paper3Directory,
        [Parameter(Mandatory = $true)]
        [string]$LatexDirectory,
        [Parameter(Mandatory = $true)]
        [string]$MainTex,
        [Parameter(Mandatory = $true)]
        [string]$OutputZip
    )

    Assert-PathExists -Path $MainTex -Kind "main tex"
    Add-Type -AssemblyName System.IO.Compression.FileSystem

    $keepSuffixes = @(".tex", ".bib", ".cls", ".bst", ".sty")
    $figureSuffixes = @(".png", ".jpg", ".jpeg", ".gif", ".pdf", ".eps", ".svg")

    $sources = Get-ChildItem -LiteralPath $LatexDirectory -Recurse -File |
        Where-Object { $keepSuffixes -contains $_.Extension.ToLowerInvariant() } |
        Sort-Object FullName

    $figures = @()
    $figures += Get-ChildItem -LiteralPath $LatexDirectory -Recurse -File |
        Where-Object { $figureSuffixes -contains $_.Extension.ToLowerInvariant() }

    $figDir = Join-Path $Paper3Directory "figures"
    if (Test-Path -LiteralPath $figDir) {
        $figures += Get-ChildItem -LiteralPath $figDir -Recurse -File |
            Where-Object { $figureSuffixes -contains $_.Extension.ToLowerInvariant() }
    }
    $figures = $figures | Sort-Object FullName -Unique

    $parent = Split-Path -Parent $OutputZip
    if (-not (Test-Path -LiteralPath $parent)) {
        New-Item -ItemType Directory -Path $parent | Out-Null
    }
    if (Test-Path -LiteralPath $OutputZip) {
        Remove-Item -LiteralPath $OutputZip -Force
    }

    $zip = [System.IO.Compression.ZipFile]::Open($OutputZip, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($source in $sources) {
            if ([System.IO.Path]::GetFullPath($source.FullName) -eq [System.IO.Path]::GetFullPath($MainTex)) {
                $entryName = Split-Path -Leaf $source.FullName
            } else {
                $entryName = Get-RepoRelativePath -Root $LatexDirectory -Path $source.FullName
            }
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $source.FullName, $entryName) | Out-Null
        }

        foreach ($figure in $figures) {
            $entryName = Get-RepoRelativePath -Root $Paper3Directory -Path $figure.FullName
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $figure.FullName, $entryName) | Out-Null
        }
    } finally {
        $zip.Dispose()
    }

    Write-Host "[paper3-zip] wrote $OutputZip"
    Write-Host "[paper3-zip] sources=$(@($sources).Count) figures=$(@($figures).Count)"
}
