#!/usr/bin/env bash
#
# Fail if a release binary carries the debug-only dev scenarios emulator.
#
# The emulator (src-tauri/src/dev_scenarios, commands/dev_scenarios.rs) is compiled
# only with debug_assertions. This checks two things:
#   1. nothing turns debug assertions on for the release profile (Cargo.toml,
#      .cargo/config.toml, CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS, RUSTFLAGS);
#   2. the raw cargo binary holds the strings of the release devsim:// guard (positive
#      control, proves its strings are greppable) and none of the emulator's names.
#
# Grep the raw target/<triple>/release/<bin>, never a bundle: bundles compress it.
#
# Usage: scripts/build/check-release-binary.sh <path-to-release-binary>
#
set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Formatted into flash::reject_simulated's error, so every release build carries them.
readonly POSITIVE_CONTROLS=(
    'devsim://'
    'simulated device path refused'
)
# Command names and cache dirs that exist only in debug builds.
readonly FORBIDDEN=(
    'dev_scenarios_status'
    'dev_set_scenario'
    'dev_create_vdisk'
    'dev_make_test_image'
    'dev-vdisks'
    'dev-test-images'
    'hideRealDevices'
    'armbianHost'
)

log_info() { echo "[check-release-binary] $*"; }
log_error() { echo "[check-release-binary] ERROR: $*" >&2; }

usage() {
    sed -n '3,15p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

contains() {
    LC_ALL=C grep -a -q -F -- "$1" "$2"
}

check_profile() {
    local failed=0
    local file
    for file in \
        "${REPO_ROOT}/src-tauri/Cargo.toml" \
        "${REPO_ROOT}/src-tauri/.cargo/config.toml" \
        "${REPO_ROOT}/.cargo/config.toml"; do
        if [[ -f "$file" ]] && grep -nE '^[[:space:]]*debug-assertions[[:space:]]*=' "$file"; then
            log_error "${file} sets debug-assertions; release builds must keep them off"
            failed=1
        fi
    done

    if [[ "${CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS:-}" == "true" ]]; then
        log_error "CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true"
        failed=1
    fi
    if [[ "${RUSTFLAGS:-} ${CARGO_ENCODED_RUSTFLAGS:-}" == *debug-assertions* ||
          "${RUSTFLAGS:-} ${CARGO_ENCODED_RUSTFLAGS:-}" == *debug_assertions* ]]; then
        log_error "RUSTFLAGS turn debug assertions on"
        failed=1
    fi
    return "$failed"
}

main() {
    if [[ $# -ne 1 || "$1" == "-h" || "$1" == "--help" ]]; then
        usage
        [[ $# -eq 1 ]] && exit 0
        exit 2
    fi

    if ! command -v grep >/dev/null 2>&1; then
        log_error "grep is required (install coreutils/grep)"
        exit 2
    fi

    local binary="$1"
    if [[ ! -f "$binary" && -f "${binary}.exe" ]]; then
        binary="${binary}.exe"
    fi
    if [[ ! -f "$binary" ]]; then
        log_error "release binary not found: $1"
        exit 2
    fi

    local failed=0
    check_profile || failed=1

    local needle
    for needle in "${POSITIVE_CONTROLS[@]}"; do
        if ! contains "$needle" "$binary"; then
            log_error "positive control '${needle}' missing from ${binary}: wrong file, or its strings are not searchable"
            exit 1
        fi
    done

    for needle in "${FORBIDDEN[@]}"; do
        if contains "$needle" "$binary"; then
            log_error "'${needle}' found in ${binary}: the dev scenarios emulator leaked into a release build"
            failed=1
        fi
    done

    if [[ "$failed" -ne 0 ]]; then
        exit 1
    fi
    log_info "OK: ${binary} has no dev scenarios code"
}

main "$@"
