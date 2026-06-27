#!/usr/bin/env bash
# run_flutter_desk.sh — Build and run the Flutter desktop application.
#
# Usage: ./scripts/run_flutter_desk.sh [OPTIONS]
#
# Options:
#   --skip-build         Skip the build step entirely and run the existing build artifact
#   --no-server          Skip starting the local doc-server backend before launching the app
#   --server-port <port> Local doc-server port (default: 2624)
#   -h, --help           Show this help message

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_APP="$ROOT/flutter_app"

# ── defaults ──────────────────────────────────────────────────────────────────
SKIP_BUILD=0
NO_SERVER=0
SERVER_PORT=2624

# ── CLI parsing ───────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --no-server)
      NO_SERVER=1
      shift
      ;;
    --server-port)
      SERVER_PORT="$2"
      shift 2
      ;;
    -h|--help)
      sed -n 's/^# //p' "$0" | head -n 10
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

SERVER_HOST="127.0.0.1"
SERVER_HEALTH_URL="http://${SERVER_HOST}:${SERVER_PORT}/api/v1/health"

info() { echo -e "\033[36m[run-flutter-desk] $1\033[0m"; }
warn() { echo -e "\033[33m[run-flutter-desk] WARN: $1\033[0m"; }
err() { echo -e "\033[31m[run-flutter-desk] ERROR: $1\033[0m" >&2; }
succ() { echo -e "\033[32m[run-flutter-desk] OK:   $1\033[0m"; }

assert_flutter_installed() {
  if ! command -v flutter >/dev/null 2>&1; then
    err "'flutter' not found in PATH. Install Flutter SDK and retry."
    exit 1
  fi
  info "Flutter: $(command -v flutter)"
}

assert_cargo_installed() {
  if ! command -v cargo >/dev/null 2>&1; then
    if [[ -f "$HOME/.cargo/bin/cargo" ]]; then
      export PATH="$HOME/.cargo/bin:$PATH"
    fi
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    err "'cargo' not found in PATH. Install Rust toolchain or add '$HOME/.cargo/bin' to PATH, then retry."
    exit 1
  fi
  info "Cargo:  $(command -v cargo)"
}

# Detect OS
OS_NAME="$(uname -s)"
case "$OS_NAME" in
  Darwin)
    PLATFORM="macos"
    EXE_DIR="$FLUTTER_APP/build/macos/Build/Products/Release"
    EXE_PATH="$EXE_DIR/doc_engine.app"
    PROCESS_PATTERN="doc_engine"
    ;;
  Linux)
    PLATFORM="linux"
    EXE_DIR="$FLUTTER_APP/build/linux/x64/release/bundle"
    EXE_PATH="$EXE_DIR/doc_engine"
    PROCESS_PATTERN="doc_engine"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    PLATFORM="windows"
    EXE_DIR="$FLUTTER_APP/build/windows/x64/runner/Release"
    EXE_PATH="$EXE_DIR/doc_engine.exe"
    PROCESS_PATTERN="doc_engine.exe"
    ;;
  *)
    err "Unsupported OS: $OS_NAME"
    exit 1
    ;;
esac

test_server_port_open() {
  nc -z "$SERVER_HOST" "$SERVER_PORT" >/dev/null 2>&1
}

clear_server_port() {
  local pids
  pids=$(lsof -t -i :"$SERVER_PORT" || true)
  if [[ -n "$pids" ]]; then
    warn "Clearing ${SERVER_HOST}:${SERVER_PORT}, stopping PID(s): $pids"
    for pid in $pids; do
      kill -9 "$pid" 2>/dev/null || true
    done

    for ((attempt = 0; attempt < 20; attempt++)); do
      if ! test_server_port_open; then
        return 0
      fi
      sleep 0.25
    done

    err "Port ${SERVER_HOST}:${SERVER_PORT} is still occupied after cleanup."
    exit 1
  fi
}

start_local_server() {
  clear_server_port
  local cargo_exe
  cargo_exe=$(command -v cargo)
  info "Starting doc-server on ${SERVER_HOST}:${SERVER_PORT}..."

  (
    cd "$ROOT"
    export DOC_SERVER_ADDR="${SERVER_HOST}:${SERVER_PORT}"
    export TEX2DOC_BOOTSTRAP_ADMIN_EMAIL="demo@example.com"
    export TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD="demo"

    "$cargo_exe" run -p doc-server >/dev/null 2>&1 &
    local server_pid=$!

    for ((attempt = 0; attempt < 45; attempt++)); do
      if ! kill -0 "$server_pid" 2>/dev/null; then
        err "doc-server exited immediately."
        exit 1
      fi

      local response
      response=$(curl -s --max-time 1 "$SERVER_HEALTH_URL" || true)
      if [[ "$response" == *'"status":"ok"'* ]]; then
        succ "doc-server ready (PID $server_pid)"
        return 0
      fi
      sleep 0.7
    done

    err "doc-server did not become healthy at $SERVER_HEALTH_URL."
    exit 1
  )
}

build_rust_crate() {
  info "Building Rust crate 'doc-native'..."
  local start_time
  start_time=$(date +%s)
  
  (
    cd "$ROOT"
    cargo build -p doc-native
  )
  
  local end_time
  end_time=$(date +%s)
  succ "Rust crate built in $((end_time - start_time))s"
}

build_flutter_desktop() {
  info "Building Flutter $PLATFORM desktop app (release)..."
  local start_time
  start_time=$(date +%s)
  
  (
    cd "$FLUTTER_APP"
    if [[ ! -d "$PLATFORM" ]]; then
      info "Platform folder '$PLATFORM' not found. Bootstrapping platform support..."
      flutter create --platforms="$PLATFORM" .
    fi
    flutter build "$PLATFORM" --release
  )
  
  local end_time
  end_time=$(date +%s)
  succ "Flutter build complete in $((end_time - start_time))s"
  info "Output: $EXE_DIR"
}

launch_app() {
  if [[ ! -e "$EXE_PATH" ]]; then
    err "Executable not found: $EXE_PATH"
    info "Run without --skip-build to build first."
    exit 1
  fi

  # Kill any existing instance
  local pids
  pids=$(pgrep -f "$PROCESS_PATTERN" || true)
  if [[ -n "$pids" ]]; then
    warn "Existing $PROCESS_PATTERN process detected (PIDs: $pids). Terminating..."
    kill -9 $pids 2>/dev/null || true
    sleep 0.5
  fi

  info "Launching: $EXE_PATH"
  if [[ "$PLATFORM" = "macos" ]]; then
    open "$EXE_PATH"
  else
    (
      cd "$EXE_DIR"
      ./"$(basename "$EXE_PATH")" >/dev/null 2>&1 &
    )
  fi

  # Poll for 5s to confirm it didn't crash immediately
  sleep 5
  local live
  live=$(pgrep -f "$PROCESS_PATTERN" || true)
  if [[ -z "$live" ]]; then
    err "No $PROCESS_PATTERN process found after launch. The app may have crashed on startup."
    exit 1
  fi
  succ "App is running (PIDs: $live)"
}

# ---------- Main ----------
info "Flutter $PLATFORM Desktop launcher"
info "Project root: $ROOT"
info "Flutter app:  $FLUTTER_APP"

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  assert_flutter_installed
  assert_cargo_installed
  build_rust_crate
  build_flutter_desktop
else
  info "--skip-build specified; skipping all build steps."
  if [[ "$NO_SERVER" -eq 0 ]]; then
    assert_cargo_installed
  fi
fi

if [[ "$NO_SERVER" -eq 0 ]]; then
  start_local_server
fi

launch_app
