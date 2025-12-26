#!/bin/bash
set -e

# Quick macOS build script - builds both ARM64 and x86_64
cd "$(dirname "$0")/.."

# Parse arguments
BUILD_MODE="production"
CLEAN_BUILD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --dev) BUILD_MODE="development"; shift ;;
        --prod) BUILD_MODE="production"; shift ;;
        --clean) CLEAN_BUILD=true; shift ;;
        *)
            echo "Usage: $0 [--dev] [--prod] [--clean]"
            echo "  --dev   Development build (debug symbols, context menu enabled)"
            echo "  --prod  Production build (optimized) [default]"
            echo "  --clean Clean build artifacts before building"
            exit 1
            ;;
    esac
done

# Clean if requested
if [ "$CLEAN_BUILD" = true ]; then
    echo "Cleaning build artifacts..."
    rm -rf src-tauri/target
    rm -rf dist
    rm -rf releases
fi

# Set build flags
if [ "$BUILD_MODE" = "development" ]; then
    echo "Building in DEVELOPMENT mode"
    TAURI_FLAGS="--debug"
    VITE_MODE="development"
else
    echo "Building in PRODUCTION mode"
    TAURI_FLAGS=""
    VITE_MODE="production"
fi

# Disable GUI interactions
export CI=true
export NONINTERACTIVE=1

# Output directory
OUTPUT_DIR="./releases"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

echo "Building frontend (mode: $VITE_MODE)..."
npm run build -- --mode "$VITE_MODE"

echo ""
echo "========================================="
echo "Building macOS ARM64 (Apple Silicon)..."
echo "========================================="
cargo tauri build $TAURI_FLAGS --bundles dmg 2>&1 || {
    echo "DMG failed, trying app bundle only..."
    cargo tauri build $TAURI_FLAGS --bundles app
}

# Copy ARM64 artifacts
for f in src-tauri/target/release/bundle/dmg/*.dmg; do
    [ -f "$f" ] && cp -v "$f" "$OUTPUT_DIR/$(basename "${f%.dmg}")_macos_arm64.dmg"
done
for f in src-tauri/target/release/bundle/macos/*.app; do
    [ -d "$f" ] && cp -rv "$f" "$OUTPUT_DIR/$(basename "${f%.app}")_macos_arm64.app"
done

echo ""
echo "========================================="
echo "Building macOS x86_64 (Intel)..."
echo "========================================="
rustup target add x86_64-apple-darwin 2>/dev/null || true
cargo tauri build $TAURI_FLAGS --target x86_64-apple-darwin --bundles dmg 2>&1 || {
    echo "DMG failed, trying app bundle only..."
    cargo tauri build $TAURI_FLAGS --target x86_64-apple-darwin --bundles app
}

# Copy x64 artifacts
for f in src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/*.dmg; do
    [ -f "$f" ] && cp -v "$f" "$OUTPUT_DIR/$(basename "${f%.dmg}")_macos_x64.dmg"
done
for f in src-tauri/target/x86_64-apple-darwin/release/bundle/macos/*.app; do
    [ -d "$f" ] && cp -rv "$f" "$OUTPUT_DIR/$(basename "${f%.app}")_macos_x64.app"
done

echo ""
echo "========================================="
echo "Build complete!"
echo "========================================="
echo "Output directory: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR/"
