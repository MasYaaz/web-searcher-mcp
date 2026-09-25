#!/bin/bash

set -e

# Installer for web-searcher
# Builds the project and installs the binary to ~/.mcp/web-searcher by default

PROJECT_NAME="web-searcher"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.mcp}"
VERBOSE="${VERBOSE:-0}"

log() { echo -e "[INFO] $1"; }
log_success() { echo -e "[OK] $1"; }
log_err() { echo -e "[ERR] $1"; }

print_help() {
  cat <<EOF
Usage: $0 [OPTIONS]

Options:
  -h, --help        Show help
  -d, --dir DIR     Installation dir (default: $INSTALL_DIR)
  --dev             Build debug (dev) instead of release
  --no-strip        Do not strip binary after building in release
  -v, --verbose     Show cargo output

EOF
}

BUILD_MODE="release"
STRIP_BINARY=1

while [[ $# -gt 0 ]]; do
  case $1 in
    -h|--help) print_help; exit 0 ;;
    -d|--dir) INSTALL_DIR="$2"; shift 2 ;;
    --dev) BUILD_MODE="dev"; shift ;;
    --no-strip) STRIP_BINARY=0; shift ;;
    -v|--verbose) VERBOSE=1; shift ;;
    *) echo "Unknown option: $1"; print_help; exit 1 ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

log "Script dir: $SCRIPT_DIR"
log "Build mode: $BUILD_MODE"
log "Install dir: $INSTALL_DIR"

if ! command -v cargo &>/dev/null; then
  log_err "cargo not found. Install Rust toolchain from https://rustup.rs/"
  exit 1
fi

if [ ! -f "$SCRIPT_DIR/Cargo.toml" ]; then
  log_err "Cargo.toml not found in $SCRIPT_DIR"
  exit 1
fi

# Build
log "Building..."
if [ "$VERBOSE" == "1" ]; then
  if [ "$BUILD_MODE" == "release" ]; then
    cargo build --release
  else
    cargo build
  fi
else
  if [ "$BUILD_MODE" == "release" ]; then
    cargo build --release --quiet
  else
    cargo build --quiet
  fi
fi

# Binary path
if [ "$BUILD_MODE" == "release" ]; then
  BIN_PATH="$SCRIPT_DIR/target/release/$PROJECT_NAME"
else
  BIN_PATH="$SCRIPT_DIR/target/debug/$PROJECT_NAME"
fi

if [ ! -f "$BIN_PATH" ]; then
  log_err "Built binary not found: $BIN_PATH"
  exit 1
fi

# Optionally strip
if [ "$STRIP_BINARY" == "1" ] && [ "$BUILD_MODE" == "release" ]; then
  if command -v strip &>/dev/null; then
    strip "$BIN_PATH" || true
  fi
fi

# Install
mkdir -p "$INSTALL_DIR"
cp "$BIN_PATH" "$INSTALL_DIR/$PROJECT_NAME"
chmod +x "$INSTALL_DIR/$PROJECT_NAME"

log_success "Installed $PROJECT_NAME to $INSTALL_DIR/$PROJECT_NAME"

if [[ ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
  log "$INSTALL_DIR is in PATH"
else
  echo "Add to PATH: export PATH=\"\$PATH:$INSTALL_DIR\""
fi

exit 0
