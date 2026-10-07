#!/usr/bin/env bash
# Build script for Armbian Imager, all platforms. Run from anywhere: ./scripts/build/build-all.sh
set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
OUTPUT_DIR="$PROJECT_DIR/releases"
DEBUG_CONFIG="${PROJECT_DIR}/src-tauri/tauri.debug.conf.json"
CONTAINER_ROOT="/app"
CONTAINER_DEBUG_CONFIG="${CONTAINER_ROOT}/src-tauri/tauri.debug.conf.json"
BUILD_LOG="/tmp/tauri-build.log"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "$*"; }
log_error() { echo -e "${RED}Error: $*${NC}" >&2; }

usage() {
    echo "Usage: $0 [--macos] [--linux] [--windows] [--all] [--clean] [--dev] [--prod] [-h|--help]"
    echo ""
    echo "  Platforms:"
    echo "    --macos    Build for macOS (x64 and ARM64)"
    echo "    --linux    Build for Linux (x64 and ARM64, requires Docker)"
    echo "    --windows  Build for Windows (x64, requires Docker)"
    echo "    --all      Build all platforms (default if no platform is given)"
    echo ""
    echo "  Build mode:"
    echo "    --dev      Development build (debug, dev scenarios panel)"
    echo "    --prod     Production build (optimized) [default]"
    echo ""
    echo "  Other:"
    echo "    --clean    Clean build artifacts before building"
    echo "    -h, --help Show this help"
}

# ============================================
# macOS Builds (Native)
# ============================================
build_macos() {
    local current_arch native_name other_target other_name f base

    log_info "\n${GREEN}========================================${NC}"
    log_info "${GREEN}  Building for macOS${NC}"
    log_info "${GREEN}========================================${NC}"

    # Disable GUI interactions for hdiutil/DMG creation
    export CI=true
    export NONINTERACTIVE=1

    current_arch="$(uname -m)"
    if [ "$current_arch" = "arm64" ]; then
        native_name="arm64"
        other_target="x86_64-apple-darwin"
        other_name="x64"
    else
        native_name="x64"
        other_target="aarch64-apple-darwin"
        other_name="arm64"
    fi

    log_info "\n${YELLOW}Building macOS $native_name (native)...${NC}"
    cargo tauri build ${TAURI_BUILD_FLAGS[@]+"${TAURI_BUILD_FLAGS[@]}"} --bundles dmg 2>&1 | tee "$BUILD_LOG"

    log_info "\n${YELLOW}Copying $native_name artifacts to releases...${NC}"
    for f in src-tauri/target/"$PROFILE"/bundle/dmg/*.dmg; do
        if [ -f "$f" ]; then
            base="$(basename "$f" .dmg)"
            cp -v "$f" "$OUTPUT_DIR/${base}_macos_${native_name}.dmg"
        fi
    done

    log_info "\n${YELLOW}Building macOS $other_name (cross-compile)...${NC}"
    rustup target add "$other_target" 2>/dev/null || true
    cargo tauri build ${TAURI_BUILD_FLAGS[@]+"${TAURI_BUILD_FLAGS[@]}"} --target "$other_target" --bundles dmg 2>&1 | tee "$BUILD_LOG"

    log_info "\n${YELLOW}Copying $other_name artifacts to releases...${NC}"
    for f in src-tauri/target/"$other_target"/"$PROFILE"/bundle/dmg/*.dmg; do
        if [ -f "$f" ]; then
            base="$(basename "$f" .dmg)"
            cp -v "$f" "$OUTPUT_DIR/${base}_macos_${other_name}.dmg"
        fi
    done

    log_info "${GREEN}macOS builds complete!${NC}"
    log_info "${GREEN}Files in: $OUTPUT_DIR${NC}"
    ls -la "$OUTPUT_DIR"/*.dmg 2>/dev/null || true
}

# ============================================
# Linux Builds (via Docker)
# ============================================
# $1: docker platform
run_linux_docker() {
    docker run --rm -v "${PROJECT_DIR}:${CONTAINER_ROOT}" -w "$CONTAINER_ROOT" \
        --platform "$1" \
        -e VITE_MODE="$VITE_MODE" \
        -e TAURI_ARGS="$CONTAINER_TAURI_ARGS" \
        debian:bookworm-slim \
        bash -c '
            apt-get update && apt-get install -y \
                curl build-essential \
                libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
                librsvg2-dev patchelf libssl-dev libgtk-3-dev \
                squashfs-tools pkg-config nodejs npm && \
            curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
            . "$HOME/.cargo/env" && \
            npm ci && npm run build -- --mode "$VITE_MODE" && \
            cargo install tauri-cli --version "^2" --locked && \
            cargo tauri build $TAURI_ARGS --bundles deb || true
        '
}

build_linux() {
    local bundle_dir="src-tauri/target/${PROFILE}/bundle" f base

    log_info "\n${GREEN}========================================${NC}"
    log_info "${GREEN}  Building for Linux (via Docker)${NC}"
    log_info "${GREEN}========================================${NC}"

    if ! command -v docker >/dev/null 2>&1; then
        log_info "${RED}Docker not found, skipping Linux builds${NC}"
        return
    fi

    log_info "\n${YELLOW}Building Linux x86_64...${NC}"
    run_linux_docker "linux/amd64"

    for f in "$bundle_dir"/deb/*.deb; do
        if [ -f "$f" ]; then
            base="$(basename "$f" .deb)"
            cp -v "$f" "$OUTPUT_DIR/${base}_linux_x64.deb"
        fi
    done
    for f in "$bundle_dir"/appimage/*.AppImage; do
        if [ -f "$f" ]; then
            base="$(basename "$f" .AppImage)"
            cp -v "$f" "$OUTPUT_DIR/${base}_linux_x64.AppImage"
        fi
    done

    # Clean target for arm64 build
    rm -rf "$bundle_dir"

    log_info "\n${YELLOW}Building Linux ARM64 (this will be slow via emulation)...${NC}"
    run_linux_docker "linux/arm64"

    for f in "$bundle_dir"/deb/*.deb; do
        if [ -f "$f" ]; then
            base="$(basename "$f" .deb)"
            cp -v "$f" "$OUTPUT_DIR/${base}_linux_arm64.deb"
        fi
    done

    log_info "${GREEN}Linux builds complete!${NC}"
}

# ============================================
# Windows Build - Not supported locally
# ============================================
build_windows() {
    log_info "\n${YELLOW}========================================${NC}"
    log_info "${YELLOW}  Windows Build${NC}"
    log_info "${YELLOW}========================================${NC}"
    log_info ""
    log_info "${RED}Windows builds are not supported locally from macOS/Linux.${NC}"
    log_info ""
    log_info "Tauri requires native Windows tools (WebView2, MSVC) that cannot"
    log_info "be cross-compiled. Use one of these options instead:"
    log_info ""
    log_info "  ${GREEN}1. GitHub Actions (recommended)${NC}"
    log_info "     Push a tag like 'v0.1.0' to trigger automatic builds"
    log_info "     See: .github/workflows/build.yml"
    log_info ""
    log_info "  ${GREEN}2. Build on Windows directly${NC}"
    log_info "     Run on a Windows machine or VM:"
    log_info "     cargo tauri build --bundles msi,nsis"
    log_info ""
}

# ============================================
# Main
# ============================================

BUILD_MACOS=false
BUILD_LINUX=false
BUILD_WINDOWS=false
BUILD_ALL=false
DO_CLEAN=false
BUILD_MODE="production"  # Default to production

while [[ $# -gt 0 ]]; do
    case "$1" in
        --macos) BUILD_MACOS=true; shift ;;
        --linux) BUILD_LINUX=true; shift ;;
        --windows) BUILD_WINDOWS=true; shift ;;
        --all) BUILD_ALL=true; shift ;;
        --clean) DO_CLEAN=true; shift ;;
        --dev) BUILD_MODE="development"; shift ;;
        --prod) BUILD_MODE="production"; shift ;;
        -h|--help) usage; exit 0 ;;
        *) usage >&2; exit 1 ;;
    esac
done

if ! $BUILD_MACOS && ! $BUILD_LINUX && ! $BUILD_WINDOWS; then
    BUILD_ALL=true
fi

if $BUILD_ALL; then
    BUILD_MACOS=true
    BUILD_LINUX=true
    BUILD_WINDOWS=true
fi

# The debug config swaps beforeBuildCommand to build:dev, which keeps the dev scenarios panel.
# CONTAINER_TAURI_ARGS is word-split inside the container; CONTAINER_ROOT has no spaces.
if [ "$BUILD_MODE" = "development" ]; then
    log_info "${YELLOW}Building in DEVELOPMENT mode${NC}"
    TAURI_BUILD_FLAGS=(--debug --config "$DEBUG_CONFIG")
    CONTAINER_TAURI_ARGS="--debug --config ${CONTAINER_DEBUG_CONFIG}"
    PROFILE="debug"
    VITE_MODE="development"
else
    log_info "${GREEN}Building in PRODUCTION mode${NC}"
    TAURI_BUILD_FLAGS=()
    CONTAINER_TAURI_ARGS=""
    PROFILE="release"
    VITE_MODE="production"
fi

cd "$PROJECT_DIR"

if $DO_CLEAN; then
    log_info "${YELLOW}Cleaning build artifacts...${NC}"
    rm -rf "$OUTPUT_DIR"
    rm -rf "$PROJECT_DIR/src-tauri/target"
    rm -rf "$PROJECT_DIR/dist"
fi

# Don't clean the output directory: allow incremental builds
mkdir -p "$OUTPUT_DIR"

log_info "${BLUE}========================================${NC}"
log_info "${BLUE}  Armbian Imager - Multi-Platform Build ${NC}"
log_info "${BLUE}========================================${NC}"

log_info "\n${YELLOW}Checking prerequisites...${NC}"

if ! command -v cargo >/dev/null 2>&1; then
    log_error "Rust/Cargo not found. Install from https://rustup.rs"
    exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
    log_error "npm not found. Install Node.js first."
    exit 1
fi

if ! cargo tauri --version >/dev/null 2>&1; then
    log_info "${YELLOW}Installing tauri-cli...${NC}"
    cargo install tauri-cli --version "^2"
fi

log_info "\n${YELLOW}Building frontend (mode: $VITE_MODE)...${NC}"
npm ci
npm run build -- --mode "$VITE_MODE"

if $BUILD_MACOS; then build_macos; fi
if $BUILD_LINUX; then build_linux; fi
if $BUILD_WINDOWS; then build_windows; fi

log_info "\n${BLUE}========================================${NC}"
log_info "${BLUE}  Build Complete!${NC}"
log_info "${BLUE}========================================${NC}"
log_info "\nArtifacts in: ${GREEN}$OUTPUT_DIR${NC}\n"
ls -la "$OUTPUT_DIR"
