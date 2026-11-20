#!/usr/bin/env bash
# evidence/rf768/loop2.sh — concurrent loop for the l1_real_adapter /
# l1_real_test_plugin non-instrument staleness-oracle audit (auxiliary
# evidence; the canonical lane is `mise run test-l1-real-plugin`).
#
# Runs the copied `l1_real_adapter_e2e` (adapter crates, `host`
# feature) and `l1_real_test_plugin_e2e` (test crates, `l1-runner`
# feature) binaries for both ABIs concurrently under optional
# sibling-checkout build load. The full_series provenance gap is a
# soundness fix, not a failing flake: a pre-fix run cannot expose the
# inflation because stale `last_output` replays are counted silently.
# The loop therefore proves the post-fix matched-only gate stays
# satisfiable under real drop conditions, and captures substituted/
# dropped counts from any failure lines.
#
# Test binaries are copied under bin-pre/ (pre-fix) or bin-post/
# (post-fix); the child runner + fixture cdylib resolve via the
# baked CARGO_BIN_EXE path into target/debug, so the copies stay
# valid as long as the fixture artifacts exist.
#
#   BIN_DIR=bin-post N=20 ./loop2.sh
#   LOAD=1 adds a concurrent `cargo build` in the sibling main
#   checkout (rf763 precedent — a different worktree so fixture DLLs
#   are never rebuilt under a running binary, which would hit
#   os error 5).

set -u
cd "$(dirname "$0")"

BIN_DIR="${BIN_DIR:-bin-post}"
N="${N:-20}"
MAIN_CHECKOUT="${MAIN_CHECKOUT:-/g/repositry/audio-products/ruviform}"
LOG_DIR="logs2-${BIN_DIR}"
mkdir -p "$LOG_DIR"

run_one() {
    local bin="$1" tag="$2" iters="$3"
    local i fails=0
    for i in $(seq 1 "$iters"); do
        if ! "$BIN_DIR/$bin" --ignored >"$LOG_DIR/${tag}-${i}.log" 2>&1; then
            fails=$((fails + 1))
            echo "${tag} iter ${i}: FAIL (see ${LOG_DIR}/${tag}-${i}.log)"
        fi
    done
    echo "${tag}: $((iters - fails))/${iters} pass" | tee "$LOG_DIR/${tag}-summary.txt"
}

if [ "${LOAD:-0}" = "1" ]; then
    (
        cd "$MAIN_CHECKOUT" || exit 1
        for _ in $(seq 1 "$N"); do
            mise exec -- cargo build \
                -p ruviform-plugin-vst3 -p ruviform-plugin-clap \
                -p ruviform-plugin-test-vst3 -p ruviform-plugin-test-clap \
                --tests \
                --features 'ruviform-plugin-clap/host,ruviform-plugin-vst3/host,ruviform-plugin-test-vst3/l1-runner,ruviform-plugin-test-clap/l1-runner' \
                >/dev/null 2>&1
        done
    ) &
    echo "load lane started (pid $!, $MAIN_CHECKOUT)"
fi

run_one clap-adapter.exe clap-adapter "$N" &
run_one vst3-adapter.exe vst3-adapter "$N" &
run_one clap-testplugin.exe clap-testplugin "$N" &
run_one vst3-testplugin.exe vst3-testplugin "$N" &
wait

echo "done — summaries in ${LOG_DIR}/*-summary.txt"
