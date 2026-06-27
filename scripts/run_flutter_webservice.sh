#!/usr/bin/env bash
# run_flutter_webservice.sh — Start Flutter Web dev server with hot reload, auto-clearing port conflicts.
#
# Usage: ./scripts/run_flutter_webservice.sh [OPTIONS]
#
# Options:
#   --port <port>   Web server port (default: 2626)
#   -h, --help      Show this help message

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_APP="$ROOT/flutter_app"

# ── defaults ──────────────────────────────────────────────────────────────────
PORT=2626

# ── CLI parsing ───────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT="$2"
      shift 2
      ;;
    -h|--help)
      sed -n 's/^# //p' "$0" | head -n 8
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

info() { echo -e "\033[36m[run-flutter-web] $1\033[0m"; }
warn() { echo -e "\033[33m[run-flutter-web] WARN: $1\033[0m"; }
err() { echo -e "\033[31m[run-flutter-web] ERROR: $1\033[0m" >&2; }

test_port_open() {
  nc -z 127.0.0.1 "$PORT" >/dev/null 2>&1
}

clear_port() {
  local pids
  pids=$(lsof -t -i :"$PORT" || true)
  if [[ -n "$pids" ]]; then
    warn "Port $PORT occupied by PID(s) $pids. Force-killing..."
    for pid in $pids; do
      kill -9 "$pid" 2>/dev/null || true
    done
    sleep 0.5
  fi

  if test_port_open; then
    err "Port $PORT still occupied after kill."
    return 1
  fi

  info "Port $PORT cleared."
  return 0
}

assert_flutter_installed() {
  if ! command -v flutter >/dev/null 2>&1; then
    err "'flutter' not found in PATH. Install Flutter SDK and retry."
    exit 1
  fi
  info "Flutter: $(command -v flutter)"
}

assert_flutter_app_dir() {
  if [[ ! -d "$FLUTTER_APP" ]]; then
    err "flutter_app directory not found at: $FLUTTER_APP"
    exit 1
  fi
}

# ---------- Main ----------
info "Flutter Web dev service starter"
info "Target port: $PORT"

if test_port_open; then
  warn "Port $PORT is in use."
  if ! clear_port; then
    err "Failed to free port $PORT. Exiting."
    exit 1
  fi
else
  info "Port $PORT is free."
fi

assert_flutter_installed
assert_flutter_app_dir

cd "$FLUTTER_APP"
info "Working directory: $FLUTTER_APP"
info "Starting: flutter run -d chrome --web-port $PORT"
info "Press Ctrl+C to stop. Inside flutter run: r=hot reload, R=hot restart, q=quit."
echo ""

flutter run -d chrome --web-port "$PORT"
