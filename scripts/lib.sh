#!/usr/bin/env bash
# lib.sh — shared cross-platform utilities for PostgreSQL backup/restore
# Source this file from sibling scripts:
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   . "${SCRIPT_DIR}/lib.sh"
#
# Disable strict-mode side-effects when sourced so caller retains control.
# shellcheck disable=SC2034,SC2310
(set -euo pipefail 2>/dev/null) || true

# ── colors ────────────────────────────────────────────────────────────────────
_colored=0
if [[ -t 1 ]]; then
  _colored=1
fi

# shellcheck disable=SC2310  # || true is intentional: log functions must not fail in -e mode
_red()   { [[ "${_colored}" = 1 ]] && printf '\033[1;31m%s\033[0m' "$*" || printf '%s' "$*"; }
_green() { [[ "${_colored}" = 1 ]] && printf '\033[1;32m%s\033[0m' "$*" || printf '%s' "$*"; }
_yellow(){ [[ "${_colored}" = 1 ]] && printf '\033[1;33m%s\033[0m' "$*" || printf '%s' "$*"; }
_blue()  { [[ "${_colored}" = 1 ]] && printf '\033[1;34m%s\033[0m' "$*" || printf '%s' "$*"; }

# ── logging ───────────────────────────────────────────────────────────────────
# shellcheck disable=SC2310,SC2312
log()  { printf '%s\n' "$*" || true; }
info() { log "$( _blue INFO: "$*" )" || true; }
warn() { log "$( _yellow WARN: "$*" )" >&2 || true; }
err()  { log "$( _red ERROR: "$*" )" >&2 || true; }
die()  { err "$@" || true; exit 1; }

# ── environment defaults ────────────────────────────────────────────────────────
# shellcheck disable=SC2154
DB_HOST="${PGHOST:-localhost}"
DB_PORT="${PGPORT:-5432}"
DB_USER="${PGUSER:-postgres}"
: "${PGPASSWORD:=postgres}"

export PGPASSWORD

# ── helpers ───────────────────────────────────────────────────────────────────
pg_version_dir() {
  # Normalise version string to a safe directory name.
  # Examples:
  #   "17.10"                                  → "17.10"
  #   "Ubuntu 17.10-1.pgdg24.04+1 / PostgreSQL 17.10" → "Ubuntu_17.10-1.pgdg24.04_1___PostgreSQL_17.10"
  echo "$1" | sed -E 's/[^A-Za-z0-9._-]+/_/g' | sed 's/_*$//'
}

find_latest_backup() {
  # $1 = version_dir, $2 = database name
  local version_dir="$1" db="$2"
  local latest
  # ls is intentional here — glob expansion is already safe; find adds no value for this use case.
  # shellcheck disable=SC2012
  latest=$(ls -1t "${version_dir}"/"${db}"-*.dump 2>/dev/null | head -n1) || return 1
  [[ -n "${latest}" ]] && echo "${latest}"
}

require_pg_tools() {
  local missing=""
  for tool in psql pg_dump pg_restore; do
    command -v "${tool}" >/dev/null 2>&1 || missing="${missing} ${tool}"
  done
  [[ -z "${missing}" ]] || die "Required PostgreSQL tools not found:${missing}  (install postgresql-client)"
}

pg_exec() {
  # "$@" = arguments passed to psql
  psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres "$@"
}

pg_exec_out() {
  # "$@" = arguments passed to psql
  # Strip wsl-host discovery messages and blank lines from output
  pg_exec "$@" 2>&1 | grep -v '^discover_other_daemon:' | grep -v '^$'
}

pg_server_version() {
  pg_exec_out -Atc "SHOW server_version;" | head -n1
}

confirm() {
  # $1 = prompt message
  local prompt="$1"
  local reply
  printf '%s ' "${prompt}" >&2 || true
  # shellcheck disable=SC2310
  read -r reply < /dev/tty 2>/dev/null || { log "" || true; return 1; }
  case "${reply}" in
    RESTORE|y|Y|yes|YES) return 0 ;;
    *) return 1 ;;
  esac
}

# ── SQL helpers ───────────────────────────────────────────────────────────────
sql_escape_literal() {
  # Escape single quotes for safe SQL string interpolation
  printf '%s' "$1" | sed "s/'/''/g"
}

sql_quote_ident() {
  # Double-quote and escape embedded double-quotes for a SQL identifier
  # shellcheck disable=SC2312
  printf '"%s"' "$(printf '%s' "$1" | sed 's/"/""/g')"
}

pg_terminate_and_drop() {
  # $1 = database name
  local db="$1"
  local lit ident
  lit=$(sql_escape_literal "${db}")
  ident=$(sql_quote_ident "${db}")

  info "Terminating existing connections to '${db}'..."
  pg_exec -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '${lit}' AND pid <> pg_backend_pid();" \
    | { grep -v '^pg_terminate_backend' | grep -v '^$' || true; }

  info "Dropping database '${db}' (if exists)..."
  pg_exec -c "DROP DATABASE IF EXISTS ${ident};"
}
