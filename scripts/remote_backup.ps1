param(
    [string]$Remote = "ubuntu@82.156.234.59",
    [string]$SshKey,
    [string]$RemoteEnvFile = "/opt/tex2doc/shared/env/doc-server.env",
    [string]$RemoteTmpDir = "/tmp",
    [string]$OutputRoot = "D:\databases",
    [int]$Retain = 7
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Retain -lt 1) {
    throw "Retain must be at least 1."
}

function Resolve-SshKey {
    param([string]$ExplicitKey)

    if ($ExplicitKey) {
        $resolved = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $ExplicitKey).Path)
        return $resolved
    }

    $candidates = @(
        (Join-Path $HOME ".ssh\tex2doc_prod_deploy"),
        (Join-Path $HOME ".ssh\orcaterm_key")
    )

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate) {
            return [System.IO.Path]::GetFullPath($candidate)
        }
    }

    throw "No SSH key found. Pass -SshKey, or create ~/.ssh/tex2doc_prod_deploy / ~/.ssh/orcaterm_key."
}

function Invoke-SshChecked {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    & ssh @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "ssh failed with exit code $LASTEXITCODE."
    }
}

function Invoke-ScpChecked {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    & scp @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "scp failed with exit code $LASTEXITCODE."
    }
}

$sshKeyPath = Resolve-SshKey $SshKey
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$remoteBaseName = "tex2doc-prod-$timestamp.dump"
$remoteDumpPath = "$RemoteTmpDir/$remoteBaseName"

New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

$remoteScript = @'
set -euo pipefail

ENV_FILE="$1"
OUT_FILE="$2"

if [ ! -f "$ENV_FILE" ]; then
  echo "Environment file not found: $ENV_FILE" >&2
  exit 2
fi

set -a
. "$ENV_FILE"
set +a

if [ -z "${DATABASE_URL:-}" ]; then
  echo "DATABASE_URL is not set in $ENV_FILE" >&2
  exit 3
fi

DB_NAME="$(psql "$DATABASE_URL" -Atc "select current_database();" 2>/dev/null || true)"
SERVER_VERSION="$(psql "$DATABASE_URL" -Atc "show server_version;" | tr -d '\r')"

pg_dump "$DATABASE_URL" --format=custom --blobs --verbose --file "$OUT_FILE"
chmod 600 "$OUT_FILE"

BYTES="$(stat -c '%s' "$OUT_FILE")"
printf 'database=%s\n' "${DB_NAME:-docdb}"
printf 'server_version=%s\n' "$SERVER_VERSION"
printf 'remote_file=%s\n' "$OUT_FILE"
printf 'bytes=%s\n' "$BYTES"
'@

$sshCommon = @(
    "-o", "BatchMode=yes",
    "-o", "ConnectTimeout=10",
    "-i", $sshKeyPath
)

Write-Host "Backing up remote production database from $Remote ..."
Write-Host "Remote env: $RemoteEnvFile"
Write-Host "Remote dump: $remoteDumpPath"

$remoteRunner = @"
tmp_script=`$(mktemp)
cat > "`$tmp_script"
sed -i '1s/^\xEF\xBB\xBF//' "`$tmp_script"
bash "`$tmp_script" '$RemoteEnvFile' '$remoteDumpPath'
rc=`$?
rm -f "`$tmp_script"
exit "`$rc"
"@

$remoteOutput = $remoteScript | & ssh @sshCommon $Remote $remoteRunner
if ($LASTEXITCODE -ne 0) {
    throw ($remoteOutput | Out-String)
}

$metadata = @{}
$remoteOutput | ForEach-Object {
    Write-Host $_
    if ($_ -match "^([^=]+)=(.*)$") {
        $metadata[$Matches[1]] = $Matches[2]
    }
}

$serverVersion = if ($metadata.ContainsKey("server_version")) { $metadata["server_version"] } else { "unknown" }
$versionDirName = ($serverVersion -replace "[^A-Za-z0-9._-]", "_")
$localDir = Join-Path $OutputRoot $versionDirName
New-Item -ItemType Directory -Force -Path $localDir | Out-Null

$database = if ($metadata.ContainsKey("database") -and $metadata["database"]) { $metadata["database"] } else { "docdb" }
$localFile = Join-Path $localDir "$database-prod-$timestamp.dump"

Write-Host "Copying backup to $localFile ..."
Invoke-ScpChecked -Arguments ($sshCommon + @("${Remote}:$remoteDumpPath", $localFile))

try {
    Invoke-SshChecked -Arguments ($sshCommon + @($Remote, "rm -f '$remoteDumpPath'"))
} catch {
    Write-Warning "Backup copied, but remote cleanup failed: $($_.Exception.Message)"
}

$backups = Get-ChildItem -Path $localDir -Filter "$database-prod-*.dump" -File |
    Sort-Object LastWriteTime -Descending

$backups | Select-Object -Skip $Retain | ForEach-Object {
    Write-Host "Removing old backup: $($_.FullName)"
    Remove-Item -LiteralPath $_.FullName -Force
}

Write-Host "Remote backup complete: $localFile"
Write-Host "Kept latest $Retain backup file(s) in $localDir"
