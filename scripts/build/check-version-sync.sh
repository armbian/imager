#!/usr/bin/env bash
# Fails when the app version differs across the four files that carry it.
set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${VERSION_SYNC_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

log_info() { printf '[INFO] %s\n' "$*"; }
log_error() { printf '[ERROR] %s\n' "$*" >&2; }

usage() {
  cat <<USAGE
Usage: $(basename "$0") [-h|--help]

Checks that package.json, package-lock.json (root and packages[""]),
src-tauri/Cargo.toml and src-tauri/tauri.conf.json carry the same version.
Set VERSION_SYNC_ROOT to check another checkout.
USAGE
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  "") ;;
  *) log_error "Unknown argument: $1"; usage >&2; exit 2 ;;
esac

if ! command -v jq >/dev/null 2>&1; then
  log_error "jq is required (macOS: brew install jq, Debian/Ubuntu: apt install jq)"
  exit 2
fi

cargo_version="$(awk '/^\[package\]/{p=1;next} /^\[/{p=0} p && /^version[[:space:]]*=/{gsub(/.*=[[:space:]]*"|".*/,""); print; exit}' "$ROOT/src-tauri/Cargo.toml")"

labels=("package.json" "package-lock.json" "package-lock.json packages[\"\"]" "src-tauri/Cargo.toml" "src-tauri/tauri.conf.json")
versions=(
  "$(jq -r '.version' "$ROOT/package.json")"
  "$(jq -r '.version' "$ROOT/package-lock.json")"
  "$(jq -r '.packages[""].version' "$ROOT/package-lock.json")"
  "$cargo_version"
  "$(jq -r '.version' "$ROOT/src-tauri/tauri.conf.json")"
)

expected="${versions[0]}"
status=0
for i in "${!labels[@]}"; do
  if [[ -z "${versions[$i]}" || "${versions[$i]}" == "null" || "${versions[$i]}" != "$expected" ]]; then
    log_error "${labels[$i]}: '${versions[$i]}' (expected '$expected')"
    status=1
  fi
done

if [[ $status -eq 0 ]]; then
  log_info "Version sync OK: $expected"
fi
exit "$status"
