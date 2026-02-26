#!/usr/bin/env bash
# Usage: ./build-scripts/test/all.sh [--omit deb,rpm]
# Runs all package tests in parallel (deb, rpm, pacman)
set -euo pipefail

cd "$(dirname "$0")/../.."

OMIT=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --omit) OMIT="$2"; shift 2 ;;
        *) echo "Unknown flag: $1"; exit 1 ;;
    esac
done

should_run() { [[ ! ",$OMIT," == *",$1,"* ]]; }

PIDS=()
LABELS=()
LOGS=()

run() {
    local label="$1" script="$2"
    local log
    log=$(mktemp /tmp/sone-test-${label}-XXXXXX.log)
    LABELS+=("$label")
    LOGS+=("$log")
    bash "$script" > "$log" 2>&1 &
    PIDS+=($!)
}

echo "=== Running all package tests in parallel ==="
echo ""

should_run deb      && run "deb"      "build-scripts/test/deb.sh"
should_run rpm      && run "rpm"      "build-scripts/test/rpm.sh"
should_run pacman   && run "pacman"   "build-scripts/test/pacman.sh"

FAILED=0
for i in "${!PIDS[@]}"; do
    if wait "${PIDS[$i]}"; then
        printf "%-12s PASSED\n" "${LABELS[$i]}"
    else
        printf "%-12s FAILED\n" "${LABELS[$i]}"
        FAILED=$((FAILED + 1))
        echo "--- ${LABELS[$i]} output ---"
        tail -30 "${LOGS[$i]}"
        echo ""
    fi
    rm -f "${LOGS[$i]}"
done

echo ""
if [[ $FAILED -gt 0 ]]; then
    echo "$FAILED test suite(s) failed."
    exit 1
fi
echo "All tests passed."
