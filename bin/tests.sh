#!/usr/bin/env bash
# Room MVP test helper
# Usage:
#   bin/tests.sh all [-- extra cargo args]
#   bin/tests.sh lifecycle [-- extra cargo args]
#   bin/tests.sh run <pattern> [-- extra cargo args]
# Examples:
#   bin/tests.sh lifecycle -- --nocapture
#   bin/tests.sh run runtime -- -- --nocapture

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
    sed -n '2,11p' "$0"
}

run_all() {
    cargo test "$@"
}

run_lifecycle() {
    cargo test --test workshop_lifecycle_trace "$@"
}

run_pattern() {
    local pattern="$1"; shift
    cargo test "$pattern" "$@"
}

main() {
    local cmd=${1:-help}
    case "$cmd" in
        all)
            shift || true
            run_all "$@"
            ;;
        lifecycle)
            shift || true
            run_lifecycle "$@"
            ;;
        run)
            shift || true
            [[ $# -ge 1 ]] || { printf "Usage: bin/tests.sh run <pattern> [-- extra args]\n" >&2; exit 1; }
            local pattern="$1"
            shift
            run_pattern "$pattern" "$@"
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            printf "Unknown command '%s'.\n\n" "$cmd" >&2
            usage
            exit 1
            ;;
    esac
}

main "$@"

