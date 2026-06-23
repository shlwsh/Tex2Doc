#!/usr/bin/env bash
# restore.sh — restore the PostgreSQL database from a .dump backup
# Usage:  ./restore.sh [OPTIONS]
#
# Options:
#   -d, --db-name      Database name          (default: docdb)
#   -u, --db-user      PostgreSQL user        (default: postgres)
#   -i, --input-root   Backup root directory  (default: database)
#   -b, --backup-file  Specific backup file   (optional; defaults to latest)
#   -f, --force        Skip confirmation prompt
#   -h, --help         Show this help
#
# WARNING: This will DROP and recreate the target database. All existing data
# will be lost. Active connections to the database are terminated first.
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
INPUT_ROOT="database"
BACKUP_FILE=""
FORCE=0

# ── usage ──────────────────────────────────────────────────────────────────────
usage() {
  sed -n 's/^# //p' "$0"
}

# ── CLI parsing ───────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    -d|--db-name)      DB_NAME="$2"; shift 2 ;;
    -u|--db-user)      DB_USER="$2"; shift 2 ;;
    -i|--input-root)   INPUT_ROOT="$2"; shift 2 ;;
    -b|--backup-file)  BACKUP_FILE="$2"; shift 2 ;;
    -f|--force)        FORCE=1; shift ;;
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
VERSION_DIR="${INPUT_ROOT}/${VERSION_DIR_NAME}"

# ── locate backup ─────────────────────────────────────────────────────────────
if [[ -n "${BACKUP_FILE}" ]]; then
  SELECTED_BACKUP="${BACKUP_FILE}"
else
  [[ -d "${VERSION_DIR}" ]] || die "No backup directory for version '${SERVER_VERSION}': ${VERSION_DIR}"
  SELECTED_BACKUP=$(find_latest_backup "${VERSION_DIR}" "${DB_NAME}") \
    || die "No backup files found for database '${DB_NAME}' in ${VERSION_DIR}"
fi

[[ -f "${SELECTED_BACKUP}" ]] || die "Backup file does not exist: ${SELECTED_BACKUP}"

# ── confirm ───────────────────────────────────────────────────────────────────
info "Restoring PostgreSQL database '${DB_NAME}' (server ${SERVER_VERSION}) ..."
info "Input: ${SELECTED_BACKUP}"
warn "WARNING: This will terminate active connections, DROP '${DB_NAME}', recreate it, and restore the backup."

if [[ "${FORCE}" = 0 ]]; then
  confirm "Type RESTORE (or y) to continue:" || { info "Restore cancelled."; exit 0; }
fi

# ── terminate connections + drop database ─────────────────────────────────────
pg_terminate_and_drop "${DB_NAME}"

# ── create fresh database ──────────────────────────────────────────────────────
info "Creating database '${DB_NAME}' (owner: ${DB_USER})..."
ident=$(sql_quote_ident "${DB_NAME}")
owner=$(sql_quote_ident "${DB_USER}")
pg_exec -c "CREATE DATABASE ${ident} OWNER ${owner};"

# ── restore ───────────────────────────────────────────────────────────────────
info "Restoring from: ${SELECTED_BACKUP}"
pg_restore -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" \
  --dbname="${DB_NAME}" \
  --no-owner --verbose \
  "${SELECTED_BACKUP}"

info "$( _green "Restore complete." )"
