#!/usr/bin/env bash
#
# Un-bundle the Wayland client libraries from the freshly built AppImage(s).
#
# linuxdeploy pulls libwebkit2gtk's transitive libwayland-{client,cursor,egl,server}
# into the AppImage. Those copies come from the build container, which is older than
# the Mesa the *user* is running, so when libEGL_mesa initialises the Wayland platform
# it hits a protocol/ABI mismatch and aborts:
#
#     Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
#
# The webview never paints and the window stays blank white. This happens inside EGL
# display init, before WebKit ever reads WEBKIT_DISABLE_DMABUF_RENDERER, which is why
# none of the usual WEBKIT_*/GDK_BACKEND/LIBGL_* workarounds have any effect.
#
# libwayland — like libGL/libEGL/libgbm/libdrm, which linuxdeploy already excludes —
# has to come from the running system so it matches the loaded EGL stack. Its soname
# has been frozen for over a decade, so deferring to the host is safe. Only the
# AppImage bundles libraries; the .deb links the system libwayland and is unaffected.
#
# See: https://github.com/armbian/imager/issues/67
#
# Usage: scripts/build/appimage-unbundle-wayland.sh [bundle-dir]
#
set -euo pipefail

BUNDLE_DIR="${1:-src-tauri/target/release/bundle/appimage}"
ARCH="${ARCH:-$(uname -m)}"

if [[ ! -d "$BUNDLE_DIR" ]]; then
    echo "No AppImage bundle directory at $BUNDLE_DIR — nothing to do."
    exit 0
fi

shopt -s nullglob
APPIMAGES=("$BUNDLE_DIR"/*.AppImage)
if [[ ${#APPIMAGES[@]} -eq 0 ]]; then
    echo "No AppImage found in $BUNDLE_DIR — nothing to do."
    exit 0
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

APPIMAGETOOL="$WORK_DIR/appimagetool"
echo "Fetching appimagetool for $ARCH"
curl -fsSL -o "$APPIMAGETOOL" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage"
chmod +x "$APPIMAGETOOL"

for APP in "${APPIMAGES[@]}"; do
    echo "==> Repacking $(basename "$APP") without bundled libwayland"

    EXTRACT_DIR="$WORK_DIR/extract"
    rm -rf "$EXTRACT_DIR"
    mkdir -p "$EXTRACT_DIR"
    # Resolve before the cd: realpath inside the subshell would run in $EXTRACT_DIR
    # and fail to find the relative $APP. The AppImage self-extracts, no FUSE needed.
    APP_ABS="$(realpath "$APP")"
    (cd "$EXTRACT_DIR" && "$APP_ABS" --appimage-extract >/dev/null)

    # Path-tolerant, and a no-op if a future linuxdeploy stops bundling these.
    find "$EXTRACT_DIR/squashfs-root" -type f \( \
        -name 'libwayland-client.so.*' -o \
        -name 'libwayland-cursor.so.*' -o \
        -name 'libwayland-egl.so.*' -o \
        -name 'libwayland-server.so.*' \) -print -delete

    # --appimage-extract-and-run so appimagetool itself doesn't need FUSE either.
    ARCH="$ARCH" "$APPIMAGETOOL" --appimage-extract-and-run \
        "$EXTRACT_DIR/squashfs-root" "$APP.new" >/dev/null
    mv -f "$APP.new" "$APP"
    chmod +x "$APP"

    # Repacking invalidates the updater signature Tauri produced during the build,
    # so mint a fresh one over the new bytes. Without this the in-app updater would
    # reject every Linux update.
    if [[ -f "$APP.sig" ]]; then
        if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
            echo "ERROR: $APP.sig exists but TAURI_SIGNING_PRIVATE_KEY is unset;" \
                 "the AppImage would ship with a stale signature." >&2
            exit 1
        fi
        echo "    re-signing $(basename "$APP")"
        # `cargo tauri build` accepts the key inline or as a path to a key file, so
        # accept both here too. Note the `signer` subcommand reads TAURI_PRIVATE_KEY*,
        # not the TAURI_SIGNING_* names the bundler uses.
        KEY="$TAURI_SIGNING_PRIVATE_KEY"
        if [[ -f "$KEY" ]]; then
            KEY="$(<"$KEY")"
        fi
        TAURI_PRIVATE_KEY="$KEY" \
        TAURI_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" \
            cargo tauri signer sign "$APP" >/dev/null
    fi

    echo "    done — $(basename "$APP") now defers libwayland to the host"
done
