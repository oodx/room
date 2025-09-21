#!/bin/bash
# Room MVP documentation validator
# Silent success, noisy failure: only emit output when something needs attention.

set -uo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

missing=()
warnings=()

current_time=$(date +%s)
one_week=$((7 * 24 * 60 * 60))
one_month=$((30 * 24 * 60 * 60))

get_mtime() {
    stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null || echo ""
}

check_exists() {
    local path="$1"
    local label="$2"
    if [[ ! -e "$path" ]]; then
        missing+=("❌ missing: $label ($path)")
        return 1
    fi
    return 0
}

check_file_age() {
    local path="$1"
    local label="$2"
    local threshold="$3"
    local mtime
    mtime=$(get_mtime "$path")
    if [[ -z "$mtime" ]]; then
        warnings+=("⚠️ cannot read timestamp for $label ($path)")
        return
    fi
    local age=$((current_time - mtime))
    if (( age > threshold )); then
        local days=$((age / 86400))
        warnings+=("⚠️ stale: $label ($path) last updated ${days}d ago")
    fi
}

# Required directories
check_exists "docs/procs" "process directory"
check_exists "docs/ref" "reference directory"
check_exists ".analysis" "analysis directory"

# Ensure no legacy process docs linger at the repo root
for legacy in BACKLOG.md CONTINUE.txt ROADMAP.txt TASKS.txt; do
    if [[ -e "$legacy" ]]; then
        missing+=("❌ legacy file should be moved under docs/procs/: $legacy")
    fi
done

# Critical process docs (7-day freshness)
critical_docs=(
    "START.txt:START entry point"
    "docs/procs/PROCESS.md:workflow guide"
    "docs/procs/CONTINUE.md:session log"
    "docs/procs/QUICK_REF.md:quick reference"
    "docs/procs/SPRINT.md:active sprint plan"
)

for entry in "${critical_docs[@]}"; do
    IFS=":" read -r path label <<<"$entry"
    if check_exists "$path" "$label"; then
        check_file_age "$path" "$label" "$one_week"
    fi
done

# Supporting process docs (30-day freshness)
support_docs=(
    "docs/procs/TASKS.md:task backlog"
    "docs/procs/BACKLOG.md:pending tickets"
    "docs/procs/ROADMAP.md:roadmap"
    "docs/procs/DONE.md:done log"
    "docs/procs/archive:TASK archive directory"
)

for entry in "${support_docs[@]}"; do
    IFS=":" read -r path label <<<"$entry"
    if [[ -d "$path" ]]; then
        : # directories already confirmed above
    else
        if check_exists "$path" "$label"; then
            check_file_age "$path" "$label" "$one_month"
        fi
    fi
done

# Reference docs that should exist (no freshness check here)
ref_docs=(
    "docs/ref/strat/LAYOUT_ENGINE_STRATEGY.md"
    "docs/ref/strat/RUNTIME_STRATEGY.md"
    "docs/ref/PLUGIN_API.md"
    "docs/ref/FEATURES_RUNTIME_PHASE2.md"
    "docs/ref/strat/LOGGING_STRATEGY.md"
    "docs/ref/strat/SHARED_RUNTIME_STRATEGY.md"
    "docs/ref/strat/SOCKET_STRATEGY.md"
    "docs/ref/strat/CORE_PLUGIN_STRATEGY.md"
    "docs/ref/strat/BENCHMARKING_STRATEGY.md"
    "docs/ref/strat/SCREEN_ZONE_STRATEGY.md"
    "docs/ref/METEOR_TOKENS.md"
    "docs/ref/RESEARCH.md"
    "docs/ref/workshops/workshop_layout_fundamentals.md"
    "docs/ref/workshops/workshop_boxy_dashboard_runtime.md"
    "docs/ref/workshops/workshop_boxy_grid.md"
)
for path in "${ref_docs[@]}"; do
    check_exists "$path" "$path" > /dev/null || true
done

# Analysis artefacts (14-day freshness)
two_weeks=$((14 * 24 * 60 * 60))
analysis_docs=(
    ".analysis/consolidated_wisdom.txt:consolidated wisdom"
    ".analysis/technical_debt.txt:technical debt snapshot"
)
for entry in "${analysis_docs[@]}"; do
    IFS=":" read -r path label <<<"$entry"
    if check_exists "$path" "$label"; then
        check_file_age "$path" "$label" "$two_weeks"
        if [[ ! -s "$path" ]]; then
            warnings+=("⚠️ empty: $label ($path)")
        fi
    fi
done

if (( ${#missing[@]} == 0 && ${#warnings[@]} == 0 )); then
    exit 0
fi

printf '=== DOCUMENTATION VALIDATION ===\n'
for err in "${missing[@]}"; do
    printf '%s\n' "$err"
done
for warn in "${warnings[@]}"; do
    printf '%s\n' "$warn"
done

if (( ${#missing[@]} > 0 )); then
    exit 1
fi
exit 0
