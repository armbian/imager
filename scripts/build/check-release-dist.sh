#!/usr/bin/env bash
#
# Fail if a release frontend bundle carries the debug-only dev scenarios panel.
#
# App.tsx lazy-loads the panel behind the __DEV_SCENARIOS__ define, which folds to false in
# `vite build`, so the chunk and every name it uses must be absent from dist/. A string the
# release bundle always holds is the positive control. The Rust binary embeds dist/
# compressed, so check-release-binary.sh cannot see these names: grep dist/ itself.
#
# Usage: scripts/build/check-release-dist.sh <path-to-dist>
#
set -euo pipefail
IFS=$'\n\t'

# EVENTS.SETTINGS_CHANGED in src/config/constants.ts; every release bundle uses it.
readonly POSITIVE_CONTROL='armbian-settings-changed'
# Names that exist only in the dev scenarios panel and the debug backend.
readonly FORBIDDEN=(
    'armbian-dev-scenarios'
    'dev_scenarios_status'
    'dev_set_scenario'
    'hideRealDevices'
    'dev-vdisks'
    'dev-test-images'
)

log_info() { echo "[check-release-dist] $*"; }
log_error() { echo "[check-release-dist] ERROR: $*" >&2; }

usage() {
    sed -n '3,10p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
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

    local dist="$1"
    if [[ ! -d "$dist" ]]; then
        log_error "dist directory not found: ${dist}"
        exit 2
    fi

    if ! LC_ALL=C grep -r -q -F -- "$POSITIVE_CONTROL" "$dist"; then
        log_error "positive control '${POSITIVE_CONTROL}' missing from ${dist}: wrong directory or an empty build"
        exit 1
    fi

    local failed=0
    local needle
    for needle in "${FORBIDDEN[@]}"; do
        if LC_ALL=C grep -r -l -F -- "$needle" "$dist"; then
            log_error "'${needle}' found in ${dist}: the dev scenarios panel leaked into a release bundle"
            failed=1
        fi
    done

    if [[ "$failed" -ne 0 ]]; then
        exit 1
    fi
    log_info "OK: ${dist} has no dev scenarios code"
}

main "$@"
