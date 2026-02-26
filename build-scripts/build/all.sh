#!/usr/bin/env bash
# Usage: ./build-scripts/build/all.sh [--omit deb,rpm] [--no-cache]
# Builds all package formats in parallel (deb, rpm, pacman)
set -euo pipefail

cd "$(dirname "$0")/../.."

OMIT=""
PASSTHROUGH=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --omit) OMIT="$2"; shift 2 ;;
        *) PASSTHROUGH+=("$1"); shift ;;
    esac
done

should_run() { [[ ! ",$OMIT," == *",$1,"* ]]; }

PIDS=()
LABELS=()
LOGS=()

run() {
    local label="$1" script="$2"
    local log
    log=$(mktemp /tmp/sone-build-${label}-XXXXXX.log)
    LABELS+=("$label")
    LOGS+=("$log")
    bash "$script" "${PASSTHROUGH[@]+"${PASSTHROUGH[@]}"}" > "$log" 2>&1 &
    PIDS+=($!)
}

echo "=== Building all packages in parallel ==="
echo ""

should_run deb      && run "deb"      "build-scripts/build/deb.sh"
should_run rpm      && run "rpm"      "build-scripts/build/rpm.sh"
should_run pacman   && run "pacman"   "build-scripts/build/pacman.sh"

FAILED=0
for i in "${!PIDS[@]}"; do
    if wait "${PIDS[$i]}"; then
        printf "%-12s DONE\n" "${LABELS[$i]}"
    else
        printf "%-12s FAILED\n" "${LABELS[$i]}"
        FAILED=$((FAILED + 1))
        echo "--- ${LABELS[$i]} output ---"
        tail -20 "${LOGS[$i]}"
        echo ""
    fi
    rm -f "${LOGS[$i]}"
done

echo ""
if [[ $FAILED -gt 0 ]]; then
    echo "$FAILED build(s) failed."
    exit 1
fi
echo "All builds complete."
