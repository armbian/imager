#!/bin/bash
set -e

# Quick Linux build script - builds via Docker
cd "$(dirname "$0")/.."

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Parse arguments
BUILD_MODE="production"
BUILD_ARCH="all"  # all, x64, arm64

CLEAN_BUILD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --dev) BUILD_MODE="development"; shift ;;
        --prod) BUILD_MODE="production"; shift ;;
        --x64) BUILD_ARCH="x64"; shift ;;
        --arm64) BUILD_ARCH="arm64"; shift ;;
        --clean) CLEAN_BUILD=true; shift ;;
        *)
            echo "Usage: $0 [--dev] [--prod] [--x64] [--arm64] [--clean]"
            echo ""
            echo "  Build mode:"
            echo "    --dev   Development build (debug symbols, context menu enabled)"
            echo "    --prod  Production build (optimized) [default]"
            echo ""
            echo "  Architecture:"
            echo "    --x64   Build only x86_64"
            echo "    --arm64 Build only ARM64"
            echo "    (default: build both)"
            echo ""
            echo "  Options:"
            echo "    --clean Clean build artifacts before building"
            exit 1
            ;;
    esac
done

# Clean if requested
if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${YELLOW}Cleaning build artifacts...${NC}"
    rm -rf src-tauri/target
    rm -rf dist
    rm -rf releases
fi

# Check Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker not found. Install Docker first.${NC}"
    exit 1
fi

# Set build flags
if [ "$BUILD_MODE" = "development" ]; then
    echo -e "${YELLOW}Building in DEVELOPMENT mode${NC}"
    TAURI_FLAGS="--debug"
    VITE_MODE="development"
else
    echo -e "${GREEN}Building in PRODUCTION mode${NC}"
    TAURI_FLAGS=""
    VITE_MODE="production"
fi

# Output directory
OUTPUT_DIR="./releases"
mkdir -p "$OUTPUT_DIR"

# Build x64
build_x64() {
    echo -e "\n${GREEN}========================================${NC}"
    echo -e "${GREEN}  Building Linux x86_64${NC}"
    echo -e "${GREEN}========================================${NC}"

    docker run --rm -v "$(pwd)":/app -w /app \
        --platform linux/amd64 \
        rust:bookworm \
        bash -c "
            apt-get update && apt-get install -y \
                libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
                librsvg2-dev patchelf libssl-dev libgtk-3-dev \
                squashfs-tools pkg-config nodejs npm && \
            npm ci && \
            npm run build -- --mode $VITE_MODE && \
            cargo install tauri-cli --version '^2' --locked && \
            cargo tauri build $TAURI_FLAGS --bundles deb || true
        "

    # Copy artifacts
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

    echo -e "${GREEN}Linux x64 build complete!${NC}"
}

# Build ARM64
build_arm64() {
    echo -e "\n${GREEN}========================================${NC}"
    echo -e "${GREEN}  Building Linux ARM64${NC}"
    echo -e "${GREEN}========================================${NC}"

    docker run --rm -v "$(pwd)":/app -w /app \
        --platform linux/arm64 \
        rust:bookworm \
        bash -c "
            apt-get update && apt-get install -y \
                libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
                librsvg2-dev patchelf libssl-dev libgtk-3-dev \
                squashfs-tools pkg-config nodejs npm && \
            npm ci && \
            npm run build -- --mode $VITE_MODE && \
            cargo install tauri-cli --version '^2' --locked && \
            cargo tauri build $TAURI_FLAGS --bundles deb || true
        "

    # Copy artifacts
    for f in src-tauri/target/release/bundle/deb/*.deb; do
        if [ -f "$f" ]; then
            BASENAME=$(basename "$f" .deb)
            cp -v "$f" "$OUTPUT_DIR/${BASENAME}_linux_arm64.deb"
        fi
    done
    for f in src-tauri/target/release/bundle/appimage/*.AppImage; do
        if [ -f "$f" ]; then
            BASENAME=$(basename "$f" .AppImage)
            cp -v "$f" "$OUTPUT_DIR/${BASENAME}_linux_arm64.AppImage"
        fi
    done

    echo -e "${GREEN}Linux ARM64 build complete!${NC}"
}

# Run builds based on architecture selection
case $BUILD_ARCH in
    x64) build_x64 ;;
    arm64) build_arm64 ;;
    all)
        build_x64
        # Clean between builds
        rm -rf src-tauri/target/release/bundle
        build_arm64
        ;;
esac

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Linux Build Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "Output directory: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR"/*.deb "$OUTPUT_DIR"/*.AppImage 2>/dev/null || true
