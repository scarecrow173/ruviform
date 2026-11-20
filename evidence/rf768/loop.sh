#!/usr/bin/env bash
# evidence/rf768/loop.sh — dual-binary concurrent loop for the l1_fault /
# l1_stateful staleness-oracle audit (auxiliary evidence; the canonical lane
# is `mise run test-l1-real-plugin`).
#
# Runs the copied `l1_stateful_test_plugin_e2e` binaries for both ABIs
# concurrently (the stateful single-shot mutation path is the highest-
# probability flake: one `dropped_submits` increment on the mutation block
# makes `wait_for_gain_output` structurally unable to observe the gain) and
# the `l1_fault_test_plugin_e2e` binaries for a smaller count (the fault
# sites are oracle soundness gaps rather than failing flakes — their loops
# run for opportunistic data). Test binaries are copied under bin-pre/
# (pre-fix) or bin-post/ (post-fix); the child runner + fixture cdylib
# resolve via the baked CARGO_BIN_EXE path into target/debug, so the copies
# stay valid as long as the fixture artifacts exist.
#
#   BIN_DIR=bin-pre N=30 FAULT_N=5 ./loop.sh
#   LOAD=1 adds a concurrent `cargo build` in the sibling main checkout
#   (rf763 precedent — a different worktree so fixture DLLs are never
#   rebuilt under a running binary, which would hit os error 5).

set -u
cd "$(dirname "$0")"

BIN_DIR="${BIN_DIR:-bin-pre}"
N="${N:-30}"
FAULT_N="${FAULT_N:-5}"
MAIN_CHECKOUT="${MAIN_CHECKOUT:-/g/repositry/audio-products/ruviform}"
LOG_DIR="logs-${BIN_DIR}"
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
            mise exec -- cargo build -p ruviform-plugin-test-vst3 \
                -p ruviform-plugin-test-clap --features l1-runner --tests \
                >/dev/null 2>&1
        done
    ) &
    echo "load lane started (pid $!, $MAIN_CHECKOUT)"
fi

run_one vst3-stateful.exe vst3-stateful "$N" &
run_one clap-stateful.exe clap-stateful "$N" &
wait

run_one vst3-fault.exe vst3-fault "$FAULT_N" &
run_one clap-fault.exe clap-fault "$FAULT_N" &
wait

echo "done — summaries in ${LOG_DIR}/*-summary.txt"
