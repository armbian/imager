#!/usr/bin/env bash
# Quick Linux build script: builds x86_64 and ARM64 .deb bundles via Docker into ./releases.
set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
OUTPUT_DIR="${REPO_ROOT}/releases"
CONTAINER_ROOT="/app"
CONTAINER_DEBUG_CONFIG="${CONTAINER_ROOT}/src-tauri/tauri.debug.conf.json"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "$*"; }
log_error() { echo -e "${RED}Error: $*${NC}" >&2; }

usage() {
    echo "Usage: $0 [--dev] [--prod] [--x64] [--arm64] [--clean] [-h|--help]"
    echo ""
    echo "  Build mode:"
    echo "    --dev   Development build (debug symbols, dev scenarios panel)"
    echo "    --prod  Production build (optimized) [default]"
    echo ""
    echo "  Architecture:"
    echo "    --x64   Build only x86_64"
    echo "    --arm64 Build only ARM64"
    echo "    (default: build both)"
    echo ""
    echo "  Options:"
    echo "    --clean Clean build artifacts before building"
    echo "    -h, --help Show this help"
}

BUILD_MODE="production"
BUILD_ARCH="all"  # all, x64, arm64
CLEAN_BUILD=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dev) BUILD_MODE="development"; shift ;;
        --prod) BUILD_MODE="production"; shift ;;
        --x64) BUILD_ARCH="x64"; shift ;;
        --arm64) BUILD_ARCH="arm64"; shift ;;
        --clean) CLEAN_BUILD=true; shift ;;
        -h|--help) usage; exit 0 ;;
        *) usage >&2; exit 1 ;;
    esac
done

if ! command -v docker >/dev/null 2>&1; then
    log_error "Docker not found. Install Docker first: https://docs.docker.com/get-docker/"
    exit 1
fi

cd "$REPO_ROOT"

if [ "$CLEAN_BUILD" = true ]; then
    log_info "${YELLOW}Cleaning build artifacts...${NC}"
    rm -rf src-tauri/target dist "$OUTPUT_DIR"
fi

# The debug config swaps beforeBuildCommand to build:dev, which keeps the dev scenarios panel.
# TAURI_ARGS is word-split inside the container; CONTAINER_ROOT has no spaces.
if [ "$BUILD_MODE" = "development" ]; then
    log_info "${YELLOW}Building in DEVELOPMENT mode${NC}"
    TAURI_ARGS="--debug --config ${CONTAINER_DEBUG_CONFIG}"
    PROFILE="debug"
    VITE_MODE="development"
else
    log_info "${GREEN}Building in PRODUCTION mode${NC}"
    TAURI_ARGS=""
    PROFILE="release"
    VITE_MODE="production"
fi
BUNDLE_DIR="src-tauri/target/${PROFILE}/bundle"

mkdir -p "$OUTPUT_DIR"

# $1: docker platform, $2: artifact suffix
build_in_docker() {
    local platform="$1" suffix="$2" f base

    docker run --rm -v "${REPO_ROOT}:${CONTAINER_ROOT}" -w "$CONTAINER_ROOT" \
        --platform "$platform" \
        -e VITE_MODE="$VITE_MODE" \
        -e TAURI_ARGS="$TAURI_ARGS" \
        rust:bookworm \
        bash -c '
            apt-get update && apt-get install -y \
                libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
                librsvg2-dev patchelf libssl-dev libgtk-3-dev \
                squashfs-tools pkg-config nodejs npm && \
            npm ci && \
            npm run build -- --mode "$VITE_MODE" && \
            cargo install tauri-cli --version "^2" --locked && \
            cargo tauri build $TAURI_ARGS --bundles deb || true
        '

    for f in "$BUNDLE_DIR"/deb/*.deb; do
        if [ -f "$f" ]; then
            base="$(basename "$f" .deb)"
            cp -v "$f" "$OUTPUT_DIR/${base}_linux_${suffix}.deb"
        fi
    done
    for f in "$BUNDLE_DIR"/appimage/*.AppImage; do
        if [ -f "$f" ]; then
            base="$(basename "$f" .AppImage)"
            cp -v "$f" "$OUTPUT_DIR/${base}_linux_${suffix}.AppImage"
        fi
    done
}

build_x64() {
    log_info "\n${GREEN}========================================${NC}"
    log_info "${GREEN}  Building Linux x86_64${NC}"
    log_info "${GREEN}========================================${NC}"
    build_in_docker "linux/amd64" "x64"
    log_info "${GREEN}Linux x64 build complete!${NC}"
}

build_arm64() {
    log_info "\n${GREEN}========================================${NC}"
    log_info "${GREEN}  Building Linux ARM64${NC}"
    log_info "${GREEN}========================================${NC}"
    build_in_docker "linux/arm64" "arm64"
    log_info "${GREEN}Linux ARM64 build complete!${NC}"
}

case "$BUILD_ARCH" in
    x64) build_x64 ;;
    arm64) build_arm64 ;;
    all)
        build_x64
        # Clean between builds
        rm -rf "$BUNDLE_DIR"
        build_arm64
        ;;
esac

log_info ""
log_info "${GREEN}========================================${NC}"
log_info "${GREEN}  Linux Build Complete!${NC}"
log_info "${GREEN}========================================${NC}"
log_info "Output directory: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR"/*.deb "$OUTPUT_DIR"/*.AppImage 2>/dev/null || true
