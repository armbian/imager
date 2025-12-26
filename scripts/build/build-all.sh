#!/bin/bash
set -e

# Build script for Armbian Imager - all platforms
# Run from project root: ./scripts/build-all.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$PROJECT_DIR/releases"

cd "$PROJECT_DIR"

# Create output directory (don't clean - allow incremental builds)
mkdir -p "$OUTPUT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Armbian Imager - Multi-Platform Build ${NC}"
echo -e "${BLUE}========================================${NC}"

# Check prerequisites
echo -e "\n${YELLOW}Checking prerequisites...${NC}"

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo not found. Install from https://rustup.rs${NC}"
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo -e "${RED}Error: npm not found. Install Node.js first.${NC}"
    exit 1
fi

# Install tauri-cli if needed
if ! cargo tauri --version &> /dev/null 2>&1; then
    echo -e "${YELLOW}Installing tauri-cli...${NC}"
    cargo install tauri-cli --version "^2"
fi

# Build frontend first
echo -e "\n${YELLOW}Building frontend (mode: $VITE_MODE)...${NC}"
npm ci
npm run build -- --mode "$VITE_MODE"

# ============================================
# macOS Builds (Native)
# ============================================
build_macos() {
    echo -e "\n${GREEN}========================================${NC}"
    echo -e "${GREEN}  Building for macOS${NC}"
    echo -e "${GREEN}========================================${NC}"

    # Disable GUI interactions for hdiutil/DMG creation
    export CI=true
    export NONINTERACTIVE=1

    # Determine current architecture
    CURRENT_ARCH=$(uname -m)
    if [ "$CURRENT_ARCH" = "arm64" ]; then
        NATIVE_TARGET="aarch64-apple-darwin"
        NATIVE_NAME="arm64"
        OTHER_TARGET="x86_64-apple-darwin"
        OTHER_NAME="x64"
    else
        NATIVE_TARGET="x86_64-apple-darwin"
        NATIVE_NAME="x64"
        OTHER_TARGET="aarch64-apple-darwin"
        OTHER_NAME="arm64"
    fi

    # Build native architecture
    echo -e "\n${YELLOW}Building macOS $NATIVE_NAME (native)...${NC}"
    cargo tauri build $TAURI_BUILD_FLAGS --bundles dmg 2>&1 | tee /tmp/tauri-build.log

    # Copy and rename native DMG immediately
    echo -e "\n${YELLOW}Copying $NATIVE_NAME artifacts to releases...${NC}"
    for f in src-tauri/target/release/bundle/dmg/*.dmg; do
        if [ -f "$f" ]; then
            BASENAME=$(basename "$f" .dmg)
            cp -v "$f" "$OUTPUT_DIR/${BASENAME}_macos_${NATIVE_NAME}.dmg"
        fi
    done

    # Build other architecture
    echo -e "\n${YELLOW}Building macOS $OTHER_NAME (cross-compile)...${NC}"
    rustup target add "$OTHER_TARGET" 2>/dev/null || true
    cargo tauri build $TAURI_BUILD_FLAGS --target "$OTHER_TARGET" --bundles dmg 2>&1 | tee /tmp/tauri-build.log

    # Copy and rename cross-compiled DMG immediately
    echo -e "\n${YELLOW}Copying $OTHER_NAME artifacts to releases...${NC}"
    for f in src-tauri/target/"$OTHER_TARGET"/release/bundle/dmg/*.dmg; do
        if [ -f "$f" ]; then
            BASENAME=$(basename "$f" .dmg)
            cp -v "$f" "$OUTPUT_DIR/${BASENAME}_macos_${OTHER_NAME}.dmg"
        fi
    done

    echo -e "${GREEN}macOS builds complete!${NC}"
    echo -e "${GREEN}Files in: $OUTPUT_DIR${NC}"
    ls -la "$OUTPUT_DIR"/*.dmg 2>/dev/null || true
}

# ============================================
# Linux Builds (via Docker)
# ============================================
build_linux() {
    echo -e "\n${GREEN}========================================${NC}"
    echo -e "${GREEN}  Building for Linux (via Docker)${NC}"
    echo -e "${GREEN}========================================${NC}"

    if ! command -v docker &> /dev/null; then
        echo -e "${RED}Docker not found, skipping Linux builds${NC}"
        return
    fi

    # Linux x86_64
    echo -e "\n${YELLOW}Building Linux x86_64...${NC}"
    docker run --rm -v "$PROJECT_DIR":/app -w /app \
        --platform linux/amd64 \
        debian:bookworm-slim \
        bash -c '
            apt-get update && apt-get install -y \
                curl build-essential \
                libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
                librsvg2-dev patchelf libssl-dev libgtk-3-dev \
                squashfs-tools pkg-config nodejs npm && \
            curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
            . "$HOME/.cargo/env" && \
            npm ci && npm run build && \
            cargo install tauri-cli --version "^2" --locked && \
            cargo tauri build --bundles deb || true
        '

    # Copy x64 artifacts
    for f in src-tauri/target/release/bundle/deb/*.deb; do
        if [ -f "$f" ]; then
            BASENAME=$(basename "$f" .deb)
            cp -v "$f" "$OUTPUT_DIR/${BASENAME}_linux_x64.deb"
        fi
    done
    for f in src-tauri/target/release/bundle/appimage/*.AppImage; do
        if [ -f "$f" ]; then
            BASENAME=$(basename "$f" .AppImage)
            cp -v "$f" "$OUTPUT_DIR/${BASENAME}_linux_x64.AppImage"
        fi
    done

    # Clean target for arm64 build
    rm -rf src-tauri/target/release/bundle

    # Linux ARM64
    echo -e "\n${YELLOW}Building Linux ARM64 (this will be slow via emulation)...${NC}"
    docker run --rm -v "$PROJECT_DIR":/app -w /app \
        --platform linux/arm64 \
        debian:bookworm-slim \
        bash -c '
            apt-get update && apt-get install -y \
                curl build-essential \
                libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
                librsvg2-dev patchelf libssl-dev libgtk-3-dev \
                squashfs-tools pkg-config nodejs npm && \
            curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
            . "$HOME/.cargo/env" && \
            npm ci && npm run build && \
            cargo install tauri-cli --version "^2" --locked && \
            cargo tauri build --bundles deb || true
        '

    # Copy ARM64 artifacts
    for f in src-tauri/target/release/bundle/deb/*.deb; do
        if [ -f "$f" ]; then
            BASENAME=$(basename "$f" .deb)
            cp -v "$f" "$OUTPUT_DIR/${BASENAME}_linux_arm64.deb"
        fi
    done

    echo -e "${GREEN}Linux builds complete!${NC}"
}

# ============================================
# Windows Build - Not supported locally
# ============================================
build_windows() {
    echo -e "\n${YELLOW}========================================${NC}"
    echo -e "${YELLOW}  Windows Build${NC}"
    echo -e "${YELLOW}========================================${NC}"
    echo -e ""
    echo -e "${RED}Windows builds are not supported locally from macOS/Linux.${NC}"
    echo -e ""
    echo -e "Tauri requires native Windows tools (WebView2, MSVC) that cannot"
    echo -e "be cross-compiled. Use one of these options instead:"
    echo -e ""
    echo -e "  ${GREEN}1. GitHub Actions (recommended)${NC}"
    echo -e "     Push a tag like 'v0.1.0' to trigger automatic builds"
    echo -e "     See: .github/workflows/build.yml"
    echo -e ""
    echo -e "  ${GREEN}2. Build on Windows directly${NC}"
    echo -e "     Run on a Windows machine or VM:"
    echo -e "     cargo tauri build --bundles msi,nsis"
    echo -e ""
}

# ============================================
# Main
# ============================================

# Parse arguments
BUILD_MACOS=false
BUILD_LINUX=false
BUILD_WINDOWS=false
BUILD_ALL=false
DO_CLEAN=false
BUILD_MODE="production"  # Default to production

if [ $# -eq 0 ]; then
    BUILD_ALL=true
fi

while [[ $# -gt 0 ]]; do
    case $1 in
        --macos) BUILD_MACOS=true; shift ;;
        --linux) BUILD_LINUX=true; shift ;;
        --windows) BUILD_WINDOWS=true; shift ;;
        --all) BUILD_ALL=true; shift ;;
        --clean) DO_CLEAN=true; shift ;;
        --dev) BUILD_MODE="development"; shift ;;
        --prod) BUILD_MODE="production"; shift ;;
        *)
            echo "Usage: $0 [--macos] [--linux] [--windows] [--all] [--clean] [--dev] [--prod]"
            echo ""
            echo "  Platforms:"
            echo "    --macos    Build for macOS (x64 and ARM64)"
            echo "    --linux    Build for Linux (x64 and ARM64, requires Docker)"
            echo "    --windows  Build for Windows (x64, requires Docker)"
            echo "    --all      Build all platforms (default if no args)"
            echo ""
            echo "  Build mode:"
            echo "    --dev      Development build (debug, context menu enabled)"
            echo "    --prod     Production build (optimized, context menu disabled) [default]"
            echo ""
            echo "  Other:"
            echo "    --clean    Clean build artifacts before building"
            exit 1
            ;;
    esac
done

# Set build flags based on mode
if [ "$BUILD_MODE" = "development" ]; then
    echo -e "${YELLOW}Building in DEVELOPMENT mode${NC}"
    TAURI_BUILD_FLAGS="--debug"
    VITE_MODE="development"
else
    echo -e "${GREEN}Building in PRODUCTION mode${NC}"
    TAURI_BUILD_FLAGS=""
    VITE_MODE="production"
fi

# Clean if requested
if $DO_CLEAN; then
    echo -e "${YELLOW}Cleaning build artifacts...${NC}"
    rm -rf "$OUTPUT_DIR"
    rm -rf "$PROJECT_DIR/src-tauri/target"
    rm -rf "$PROJECT_DIR/dist"
fi

if $BUILD_ALL; then
    BUILD_MACOS=true
    BUILD_LINUX=true
    BUILD_WINDOWS=true
fi

# Run builds
$BUILD_MACOS && build_macos
$BUILD_LINUX && build_linux
$BUILD_WINDOWS && build_windows

# Summary
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}  Build Complete!${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "\nArtifacts in: ${GREEN}$OUTPUT_DIR${NC}\n"
ls -la "$OUTPUT_DIR"
