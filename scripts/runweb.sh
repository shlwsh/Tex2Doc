#!/usr/bin/env bash
# runweb.sh — Start the Tex2Doc Flutter Web app.
#
# Usage: ./scripts/runweb.sh [OPTIONS]
#
# Options:
#   --port <port>            Web server port (default: 2625)
#   --host-address <addr>    Host address for the web-server target (default: 127.0.0.1)
#   --device <device>        Flutter web device: web-server, chrome, or edge (default: web-server)
#   --release                Run the app in release mode
#   -h, --help               Show this help message

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_APP="$ROOT/flutter_app"
PUBSPEC="$FLUTTER_APP/pubspec.yaml"

if [[ ! -f "$PUBSPEC" ]]; then
  echo "[runweb] ERROR: Flutter app not found at: $FLUTTER_APP" >&2
  exit 1
fi

# ── defaults ──────────────────────────────────────────────────────────────────
PORT=2625
HOST_ADDRESS="127.0.0.1"
DEVICE="web-server"
RELEASE=0

# ── CLI parsing ───────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT="$2"
      shift 2
      ;;
    --host-address)
      HOST_ADDRESS="$2"
      shift 2
      ;;
    --device)
      DEVICE="$2"
      if [[ "$DEVICE" != "web-server" && "$DEVICE" != "chrome" && "$DEVICE" != "edge" ]]; then
        echo "[runweb] ERROR: Invalid device: $DEVICE. Must be web-server, chrome, or edge." >&2
        exit 1
      fi
      shift 2
      ;;
    --release)
      RELEASE=1
      shift
      ;;
    -h|--help)
      sed -n 's/^# //p' "$0" | head -n 11
      exit 0
      ;;
    *)
      echo "[runweb] ERROR: Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

info() { echo -e "\033[36m[runweb] $1\033[0m"; }
err() { echo -e "\033[31m[runweb] ERROR: $1\033[0m" >&2; }

assert_flutter_installed() {
  if ! command -v flutter >/dev/null 2>&1; then
    err "'flutter' not found in PATH. Install Flutter SDK and retry."
    exit 1
  fi
  info "Flutter: $(command -v flutter)"
}

assert_flutter_installed

flutter_args=("run" "-d" "$DEVICE")
if [[ "$DEVICE" = "web-server" ]]; then
  flutter_args+=("--web-hostname" "$HOST_ADDRESS" "--web-port" "$PORT")
else
  flutter_args+=("--web-port" "$PORT")
fi

if [[ "$RELEASE" -eq 1 ]]; then
  flutter_args+=("--release")
fi

info "Working directory: $FLUTTER_APP"
info "Starting: flutter ${flutter_args[*]}"
if [[ "$DEVICE" = "web-server" ]]; then
  info "URL: http://$HOST_ADDRESS:$PORT/"
fi
info "Press Ctrl+C to stop. Inside flutter run: r=hot reload, R=hot restart, q=quit."
echo ""

cd "$FLUTTER_APP"
flutter "${flutter_args[@]}"
