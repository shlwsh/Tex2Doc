#!/usr/bin/env bash
# backup.sh — back up the PostgreSQL database
# Usage:  ./backup.sh [OPTIONS]
#
# Options:
#   -d, --db-name      Database name            (default: docdb)
#   -u, --db-user      PostgreSQL user          (default: postgres)
#   -o, --output-root  Output directory         (default: database)
#   -r, --retain       Number of backups to keep (default: 2)
#   -h, --help         Show this help
#
# Environment variables (all optional):
#   PGHOST      PostgreSQL host     (default: localhost)
#   PGPORT      PostgreSQL port     (default: 5432)
#   PGUSER      PostgreSQL user     (default: postgres)
#   PGPASSWORD  PostgreSQL password (default: postgres)
#   PGDATABASE  Database name      (default: docdb)

# shellcheck disable=SC1091,SC2012,SC2312
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
. "${SCRIPT_DIR}/lib.sh"

# ── defaults ──────────────────────────────────────────────────────────────────
DB_NAME="${PGDATABASE:-docdb}"
OUTPUT_ROOT="database"
RETAIN=2

# ── usage ──────────────────────────────────────────────────────────────────────
usage() {
  sed -n 's/^# //p' "$0"
}

# ── CLI parsing ───────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    -d|--db-name)     DB_NAME="$2"; shift 2 ;;
    -u|--db-user)     DB_USER="$2"; shift 2 ;;
    -o|--output-root) OUTPUT_ROOT="$2"; shift 2 ;;
    -r|--retain)
      RETAIN="$2"
      if ! [[ "${RETAIN}" =~ ^[0-9]+$ ]] || [[ "${RETAIN}" -lt 1 ]]; then
        die "Retain must be a positive integer, got: ${RETAIN}"
      fi
      shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) die "Unknown option: $1  (use --help)" ;;
  esac
done

export PGDATABASE="${DB_NAME}" DB_HOST DB_PORT DB_USER

require_pg_tools

# ── detect PostgreSQL version ─────────────────────────────────────────────────
info "Querying PostgreSQL server version..."
SERVER_VERSION=$(pg_server_version)
[[ -n "${SERVER_VERSION}" ]] || die "Failed to read server_version from PostgreSQL."
info "PostgreSQL server version: ${SERVER_VERSION}"

VERSION_DIR_NAME=$(pg_version_dir "${SERVER_VERSION}")
VERSION_DIR="${OUTPUT_ROOT}/${VERSION_DIR_NAME}"
mkdir -p "${VERSION_DIR}"

# ── dump ──────────────────────────────────────────────────────────────────────
TIMESTAMP=$(date '+%Y%m%d-%H%M%S')
BACKUP_FILE="${VERSION_DIR}/${DB_NAME}-${TIMESTAMP}.dump"

info "Backing up PostgreSQL database '${DB_NAME}' (server ${SERVER_VERSION}) ..."
info "Output: ${BACKUP_FILE}"

pg_dump -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" \
  --format=custom --blobs \
  --file="${BACKUP_FILE}" \
  "${DB_NAME}"

# ── prune old backups ─────────────────────────────────────────────────────────
BACKUP_COUNT=$(ls -1t "${VERSION_DIR}"/"${DB_NAME}"-*.dump 2>/dev/null | wc -l || true)
if [[ "${BACKUP_COUNT}" -gt "${RETAIN}" ]]; then
  info "Pruning old backups (keeping latest ${RETAIN} of ${BACKUP_COUNT})..."
  ls -1t "${VERSION_DIR}"/"${DB_NAME}"-*.dump 2>/dev/null \
    | tail -n +$((RETAIN + 1)) \
    | while read -r old; do
        info "  Removing: ${old}"
        rm -f "${old}"
      done
fi

info "$( _green "Backup complete." )  Kept latest ${RETAIN} backup file(s) in ${VERSION_DIR}"
