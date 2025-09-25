#!/usr/bin/env bash
# Room MVP example helper
# Usage:
#   bin/examples.sh list [--all] # list available examples (active/prototype by default)
#   bin/examples.sh explain ID  # show docs for a specific example (try ID from list)
#   bin/examples.sh run ID [-- extra cargo args]
# Examples:
#   bin/examples.sh run chat_demo
#   bin/examples.sh run workshop_layout_fundamentals -- nested
#   bin/examples.sh explain workshop_layout_fundamentals

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

EXAMPLES_DIR="examples"
GUIDES_DIR="docs/ref/workshops"

usage() {
    sed -n '2,12p' "$0"
}

example_status() {
    case "$1" in
        audit_demo|bootstrap_helper|runtime_first_paint|chat_demo|control_room|workshop_lifecycle_trace)
            echo "active"
            ;;
        mud_mini_game|mud_boxy_game|boxy_dashboard_runtime|boxy_dashboard)
            echo "prototype"
            ;;
        workshop_lifecycle_trace_01|pilot_mini_editor)
            echo "legacy"
            ;;
        *)
            echo "legacy"
            ;;
    esac
}

status_label() {
    case "$1" in
        active) echo "active" ;;
        prototype) echo "prototype (WIP)" ;;
        legacy) echo "legacy (deprecated)" ;;
    esac
}

list_examples() {
    local show_all="$1"
    printf "Available examples:\n"
    (cd "$EXAMPLES_DIR" && ls *.rs 2>/dev/null) \
        | sort \
        | sed 's/\.rs$//' \
        | while read -r name; do
            local status
            status="$(example_status "$name")"
            if [[ "$status" == "legacy" && "$show_all" != "true" ]]; then
                continue
            fi
            guide="$GUIDES_DIR/${name}.md"
            if [[ -f "$guide" ]]; then
                printf "  %-35s [%s] (guide: docs/ref/workshops/%s.md)\n" \
                    "$name" "$(status_label "$status")" "$name"
            else
                printf "  %-35s [%s]\n" "$name" "$(status_label "$status")"
            fi
        done
}

explain_example() {
    local id="$1"
    local guide="$GUIDES_DIR/${id}.md"
    local source="$EXAMPLES_DIR/${id}.rs"

    if [[ -f "$guide" ]]; then
        printf "\n== Guide: docs/ref/workshops/%s.md ==\n\n" "$id"
        cat "$guide"
    elif [[ -f "$source" ]]; then
        printf "\nNo workshop guide found; showing source header for %s:\n\n" "$id"
        sed -n '1,80p' "$source"
    else
        printf "Example '%s' not found. Run 'bin/examples.sh list'.\n" "$id" >&2
        exit 1
    fi
}

run_example() {
    local id="$1"; shift
    local source="$EXAMPLES_DIR/${id}.rs"

    if [[ ! -f "$source" ]]; then
        printf "Example '%s' not found. Run 'bin/examples.sh list'.\n" "$id" >&2
        exit 1
    fi

    local status
    status="$(example_status "$id")"
    if [[ "$status" == "legacy" ]]; then
        printf "Example '%s' is marked legacy and is omitted from the default list.\n" "$id" >&2
        printf "Use 'bin/examples.sh list --all' to inspect legacy entries." >&2
        exit 1
    fi

    printf "Running example '%s'...\n\n" "$id"
    cargo run --example "$id" "$@"
}

main() {
    local cmd=${1:-help}
    case "$cmd" in
        list)
            shift || true
            local show_all="false"
            if [[ ${1:-} == "--all" ]]; then
                show_all="true"
            fi
            list_examples "$show_all"
            ;;
        explain)
            shift || true
            [[ $# -ge 1 ]] || { printf "Usage: bin/examples.sh explain <example>\n" >&2; exit 1; }
            explain_example "$1"
            ;;
        run)
            shift || true
            [[ $# -ge 1 ]] || { printf "Usage: bin/examples.sh run <example> [-- cargo args]\n" >&2; exit 1; }
            run_example "$1" "${@:2}"
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
