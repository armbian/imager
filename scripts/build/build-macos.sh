#!/usr/bin/env bash
# Quick macOS build script: builds ARM64 and x86_64 bundles into ./releases.
set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DEBUG_CONFIG="${REPO_ROOT}/src-tauri/tauri.debug.conf.json"
OUTPUT_DIR="${REPO_ROOT}/releases"
X64_TARGET="x86_64-apple-darwin"

log_info() { echo "$*"; }
log_error() { echo "Error: $*" >&2; }

usage() {
    echo "Usage: $0 [--dev] [--prod] [--clean] [-h|--help]"
    echo "  --dev   Development build (debug symbols, dev scenarios panel)"
    echo "  --prod  Production build (optimized) [default]"
    echo "  --clean Clean build artifacts before building"
}

BUILD_MODE="production"
CLEAN_BUILD=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dev) BUILD_MODE="development"; shift ;;
        --prod) BUILD_MODE="production"; shift ;;
        --clean) CLEAN_BUILD=true; shift ;;
        -h|--help) usage; exit 0 ;;
        *) usage >&2; exit 1 ;;
    esac
done

for tool in cargo npm; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        log_error "$tool not found. Install Rust from https://rustup.rs and Node.js from https://nodejs.org"
        exit 1
    fi
done
if ! cargo tauri --version >/dev/null 2>&1; then
    log_error "tauri-cli not found. Install it with: cargo install tauri-cli --version '^2' --locked"
    exit 1
fi

cd "$REPO_ROOT"

if [ "$CLEAN_BUILD" = true ]; then
    log_info "Cleaning build artifacts..."
    rm -rf src-tauri/target dist "$OUTPUT_DIR"
fi

# The debug config swaps beforeBuildCommand to build:dev, which keeps the dev scenarios panel.
if [ "$BUILD_MODE" = "development" ]; then
    log_info "Building in DEVELOPMENT mode"
    TAURI_FLAGS=(--debug --config "$DEBUG_CONFIG")
    PROFILE="debug"
    VITE_MODE="development"
else
    log_info "Building in PRODUCTION mode"
    TAURI_FLAGS=()
    PROFILE="release"
    VITE_MODE="production"
fi

# Disable GUI interactions
export CI=true
export NONINTERACTIVE=1

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

log_info "Building frontend (mode: $VITE_MODE)..."
npm run build -- --mode "$VITE_MODE"

copy_artifacts() {
    local bundle_dir="$1" suffix="$2" f
    for f in "$bundle_dir"/dmg/*.dmg; do
        if [ -f "$f" ]; then
            cp -v "$f" "$OUTPUT_DIR/$(basename "${f%.dmg}")_macos_${suffix}.dmg"
        fi
    done
    for f in "$bundle_dir"/macos/*.app; do
        if [ -d "$f" ]; then
            cp -rv "$f" "$OUTPUT_DIR/$(basename "${f%.app}")_macos_${suffix}.app"
        fi
    done
}

log_info ""
log_info "========================================="
log_info "Building macOS ARM64 (Apple Silicon)..."
log_info "========================================="
cargo tauri build ${TAURI_FLAGS[@]+"${TAURI_FLAGS[@]}"} --bundles dmg 2>&1 || {
    log_info "DMG failed, trying app bundle only..."
    cargo tauri build ${TAURI_FLAGS[@]+"${TAURI_FLAGS[@]}"} --bundles app
}
copy_artifacts "src-tauri/target/${PROFILE}/bundle" "arm64"

log_info ""
log_info "========================================="
log_info "Building macOS x86_64 (Intel)..."
log_info "========================================="
rustup target add "$X64_TARGET" 2>/dev/null || true
cargo tauri build ${TAURI_FLAGS[@]+"${TAURI_FLAGS[@]}"} --target "$X64_TARGET" --bundles dmg 2>&1 || {
    log_info "DMG failed, trying app bundle only..."
    cargo tauri build ${TAURI_FLAGS[@]+"${TAURI_FLAGS[@]}"} --target "$X64_TARGET" --bundles app
}
copy_artifacts "src-tauri/target/${X64_TARGET}/${PROFILE}/bundle" "x64"

log_info ""
log_info "========================================="
log_info "Build complete!"
log_info "========================================="
log_info "Output directory: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR/"
