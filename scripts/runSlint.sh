#!/usr/bin/env bash
# runSlint.sh — Build and run the Tex2Doc Slint desktop application.
#
# Usage: ./scripts/runSlint.sh [OPTIONS]
#
# Options:
#   --profile <dev|release>  Build profile (default: dev)
#   --no-build               Skip building the binary
#   --build-only             Build the binary and exit without launching
#   --no-server              Skip starting the backend doc-server
#   --server-port <port>     doc-server port (default: 2624)
#   --cargo-path <path>      Explicit path to cargo executable
#   -h, --help               Show this help message

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="$ROOT/apps/slint-user/Cargo.toml"

if [[ ! -f "$CARGO_TOML" ]]; then
  echo "slint-user Cargo.toml not found at: $CARGO_TOML" >&2
  exit 1
fi

# ── defaults ──────────────────────────────────────────────────────────────────
PROFILE="dev"
NO_BUILD=0
BUILD_ONLY=0
NO_SERVER=0
SERVER_PORT=2624
CARGO_PATH=""

# ── CLI parsing ───────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      PROFILE="$2"
      if [[ "$PROFILE" != "dev" && "$PROFILE" != "release" ]]; then
        echo "Invalid profile: $PROFILE. Must be dev or release." >&2
        exit 1
      fi
      shift 2
      ;;
    --no-build)
      NO_BUILD=1
      shift
      ;;
    --build-only)
      BUILD_ONLY=1
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
    --cargo-path)
      CARGO_PATH="$2"
      shift 2
      ;;
    -h|--help)
      sed -n 's/^# //p' "$0" | head -n 12
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

if [[ "$NO_BUILD" -eq 1 && "$BUILD_ONLY" -eq 1 ]]; then
  echo "--no-build and --build-only cannot be used together." >&2
  exit 1
fi

PACKAGE_NAME="doc-desktop-slint"
SERVER_PACKAGE_NAME="doc-server"
SERVER_HOST="127.0.0.1"
SERVER_HEALTH_URL="http://${SERVER_HOST}:${SERVER_PORT}/api/v1/health"

TARGET_DIR="$ROOT/target"
if [[ "$PROFILE" = "release" ]]; then
  PROFILE_DIR="$TARGET_DIR/release"
else
  PROFILE_DIR="$TARGET_DIR/debug"
fi

EXE_NAME="$PACKAGE_NAME"
EXE_PATH="$PROFILE_DIR/$EXE_NAME"

# ── helper functions ──────────────────────────────────────────────────────────
resolve_cargo_path() {
  local requested_path="$1"
  if [[ -n "$requested_path" ]]; then
    if [[ ! -f "$requested_path" ]]; then
      echo "CargoPath is not a file: $requested_path" >&2
      exit 1
    fi
    echo "$requested_path"
    return 0
  fi

  if command -v cargo >/dev/null 2>&1; then
    command -v cargo
    return 0
  fi

  local candidates=(
    "$HOME/.cargo/bin/cargo"
  )

  for candidate in "${candidates[@]}"; do
    if [[ -f "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  done

  echo "Cargo was not found. Install Rust from https://rustup.rs/ or pass --cargo-path <path-to-cargo>." >&2
  exit 1
}

stop_existing_slint() {
  local pids
  pids=$(pgrep -f "target/(debug|release)/doc-desktop-slint" || true)
  if [[ -n "$pids" ]]; then
    echo "[runSlint] stopping existing doc-desktop-slint process(es): $pids"
    kill -9 $pids 2>/dev/null || true
    sleep 0.5
  fi
}

start_slint() {
  if [[ ! -f "$EXE_PATH" ]]; then
    echo "Binary not found (run without --no-build first): $EXE_PATH" >&2
    exit 1
  fi

  stop_existing_slint
  echo "[runSlint] launching $EXE_PATH ..."

  (
    cd "$PROFILE_DIR"
    export ICU4X_DATA_DIR=""
    export LANG="en_US.UTF-8"
    export LC_ALL="en_US.UTF-8"
    
    ./"$EXE_NAME" >/dev/null 2>&1 &
    local slint_pid=$!
    sleep 0.8
    if ! kill -0 "$slint_pid" 2>/dev/null; then
      echo "Slint app exited immediately." >&2
      exit 1
    fi
    echo "[runSlint] started PID $slint_pid"
  )
}

test_server_port_open() {
  nc -z "$SERVER_HOST" "$SERVER_PORT" >/dev/null 2>&1
}

clear_server_port() {
  local pids
  # Find processes listening on the port
  pids=$(lsof -t -i :"$SERVER_PORT" || true)
  if [[ -n "$pids" ]]; then
    echo "[runSlint] clearing ${SERVER_HOST}:${SERVER_PORT}, stopping process(es): $pids"
    for pid in $pids; do
      kill -9 "$pid" 2>/dev/null || true
    done

    for ((attempt = 0; attempt < 20; attempt++)); do
      if ! test_server_port_open; then
        return 0
      fi
      sleep 0.25
    done

    echo "Port ${SERVER_HOST}:${SERVER_PORT} is still occupied after cleanup." >&2
    exit 1
  fi
}

start_local_server() {
  local cargo_exe="$1"
  clear_server_port

  echo "[runSlint] starting local $SERVER_PACKAGE_NAME on ${SERVER_HOST}:${SERVER_PORT} ..."

  (
    cd "$ROOT"
    export DOC_SERVER_ADDR="${SERVER_HOST}:${SERVER_PORT}"
    export TEX2DOC_BOOTSTRAP_ADMIN_EMAIL="demo@example.com"
    export TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD="demo"

    "$cargo_exe" run -p "$SERVER_PACKAGE_NAME" >/dev/null 2>&1 &
    local server_pid=$!

    for ((attempt = 0; attempt < 45; attempt++)); do
      if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "doc-server exited immediately." >&2
        exit 1
      fi

      local response
      response=$(curl -s --max-time 1 "$SERVER_HEALTH_URL" || true)
      if [[ "$response" == *'"status":"ok"'* ]]; then
        echo "[runSlint] doc-server ready (PID $server_pid)"
        return 0
      fi
      sleep 0.7
    done

    echo "doc-server did not become healthy at $SERVER_HEALTH_URL." >&2
    exit 1
  )
}

# ── main flow ─────────────────────────────────────────────────────────────────
if [[ "$NO_BUILD" -eq 1 ]]; then
  if [[ "$NO_SERVER" -eq 0 ]]; then
    CARGO_EXE=$(resolve_cargo_path "$CARGO_PATH")
    start_local_server "$CARGO_EXE"
  fi
  start_slint
  exit 0
fi

stop_existing_slint

display_profile="$PROFILE"
echo "[runSlint] building ($display_profile) $PACKAGE_NAME ..."
CARGO_EXE=$(resolve_cargo_path "$CARGO_PATH")
echo "[runSlint] using cargo: $CARGO_EXE"

if [[ "$PROFILE" = "release" ]]; then
  (cd "$ROOT" && "$CARGO_EXE" build --profile=release -p "$PACKAGE_NAME")
else
  (cd "$ROOT" && "$CARGO_EXE" build -p "$PACKAGE_NAME")
fi

if [[ ! -f "$EXE_PATH" ]]; then
  echo "build succeeded but binary not found at: $EXE_PATH" >&2
  exit 1
fi

if [[ "$BUILD_ONLY" -eq 1 ]]; then
  echo "[runSlint] build completed: $EXE_PATH"
  exit 0
fi

if [[ "$NO_SERVER" -eq 0 ]]; then
  start_local_server "$CARGO_EXE"
fi

start_slint
