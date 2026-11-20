#!/usr/bin/env bash
# evidence/rf841/loop.sh — SpawnFailed{Timeout} spawn-deadline flake
# quantification loop for ruviform#841 (auxiliary evidence; the
# canonical lane is `mise run test-l1-real-plugin`).
#
# The flake: `command_timeout=500ms` on the hang-contract tests also
# bounds the spawn hand-off wait and the Hello wait, so under load
# `Instantiate` can return `Err(HostError::SpawnFailed{cause:
# Timeout})` before the plugin is minted. Measured here as
# pass/fail of copied `l1_fault_test_plugin_e2e` binaries for both
# ABIs (vst3/clap), stored under BIN_DIR/ (bin/ default) — the
# copies stay valid because the runner binary + fixture cdylib
# resolve via the baked `CARGO_BIN_EXE_*` path into target/debug
# (rf763/rf768 precedent).
#
#   ARM=cold  — per iter: <bin> --ignored --exact <PROBE>
#               exactly one spawn per binary run, always the
#               process's first (cold) instantiate.
#   ARM=warm  — per iter: <bin> --ignored --test-threads=1 \
#                         <WARMUP> <PROBE>
#               WARMUP (default 10s timeout) runs first serially, so
#               the probe's instantiate is a warmed spawn.
#   ARM=warm3 — per iter: three WARMUP3 filters serially before the
#               probe — the probe is the 4th spawn in the process,
#               filling the cold(1st)→warm(2nd)→serial(7th+)
#               gradient.
#   ARM=full  — per iter: <bin> --ignored
#               whole suite at default parallelism — the
#               canonical-lane exposure shape.
#   ARM=serial — per iter: <bin> --ignored --test-threads=1
#               whole suite serially — every probe spawn is warmed
#               by earlier tests but no spawns race, isolating
#               "warm" from "concurrent".
#
#   LOAD=1 adds a concurrent `cargo build` loop in the sibling main
#   checkout MAIN_CHECKOUT (rf763/rf768 precedent — a different
#   worktree/target dir, so fixture DLLs the running binaries hold
#   are never rebuilt, avoiding os error 5).
#
# Usage:
#   BIN_DIR=bin ARM=cold N=30 LOAD=0 ./loop.sh
#   BIN_DIR=bin ARM=warm N=20 LOAD=1 ./loop.sh
#
# Per-iter logs → logs-<arm>-<load0|load1>[/<abi>-<i>.log];
# classification per iter (pass / spawn_timeout / warmup_fail /
# other_fail) + per-arm summary → logs-*/summary.txt.
# A zero-fail arm is recorded as "0/N observed", not skipped.

set -u
cd "$(dirname "$0")"

ARM="${ARM:-cold}"
BIN_DIR="${BIN_DIR:-bin}"
N="${N:-30}"
LOAD="${LOAD:-0}"
MAIN_CHECKOUT="${MAIN_CHECKOUT:-/g/repositry/audio-products/ruviform}"
PROBE="${PROBE:-real_test_fault_l1_hang_converges_on_timeout}"
WARMUP="${WARMUP:-real_test_fault_l1_descriptor_and_clean_block}"
WARMUP3="${WARMUP3:-real_test_fault_l1_abort_fails_instance real_test_fault_l1_exit_zero_is_failure $WARMUP}"
LOG_DIR="logs-${ARM}-load${LOAD}"
mkdir -p "$LOG_DIR"

case "$ARM" in
cold)
    ARGS=(--ignored --exact "$PROBE")
    ;;
warm)
    ARGS=(--ignored --test-threads=1 "$WARMUP" "$PROBE")
    ;;
warm3)
    # shellcheck disable=SC2086
    ARGS=(--ignored --test-threads=1 $WARMUP3 "$PROBE")
    ;;
full)
    ARGS=(--ignored)
    ;;
serial)
    ARGS=(--ignored --test-threads=1)
    ;;
*)
    echo "unknown ARM=$ARM (cold|warm|warm3|full|serial)" >&2
    exit 2
    ;;
esac

if [ "${LOAD}" = "1" ]; then
    (
        cd "$MAIN_CHECKOUT" || exit 1
        for _ in $(seq 1 "$N"); do
            mise exec -- cargo build -p ruviform-plugin-test-vst3 \
                -p ruviform-plugin-test-clap --features l1-runner --tests \
                >/dev/null 2>&1
        done
    ) &
    LOAD_PID=$!
    echo "load lane started (pid $LOAD_PID, $MAIN_CHECKOUT)"
fi

# Classify one iteration log: prints "<class>|<failing tests>".
classify() {
    local log="$1"
    local failed_tests
    failed_tests=$(grep -oE 'test [a-z_0-9]+ \.\.\. FAILED' "$log" | sed 's/^test //; s/ \.\.\. FAILED$//' | tr '\n' ',' | sed 's/,$//')
    if grep -q 'SpawnFailed { cause: Timeout' "$log"; then
        echo "spawn_timeout|${failed_tests:-unknown}"
    elif grep -q "test ${WARMUP} \.\.\. FAILED" "$log"; then
        echo "warmup_fail|${failed_tests:-unknown}"
    else
        echo "other_fail|${failed_tests:-unknown}"
    fi
}

run_one() {
    local bin="$1" abi="$2"
    local i fails=0 spawn_to=0 warm=0 other=0
    : >"$LOG_DIR/${abi}-classes.txt"
    for i in $(seq 1 "$N"); do
        local log="$LOG_DIR/${abi}-${i}.log"
        if "$BIN_DIR/$bin" "${ARGS[@]}" >"$log" 2>&1; then
            echo "pass|" >>"$LOG_DIR/${abi}-classes.txt"
            continue
        fi
        fails=$((fails + 1))
        local cls
        cls=$(classify "$log")
        echo "$cls" >>"$LOG_DIR/${abi}-classes.txt"
        case "$cls" in
        spawn_timeout\|*) spawn_to=$((spawn_to + 1)) ;;
        warmup_fail\|*) warm=$((warm + 1)) ;;
        *) other=$((other + 1)) ;;
        esac
        echo "${abi} iter ${i}: FAIL ${cls} (${log})"
    done
    {
        echo "${abi}: $((N - fails))/${N} pass"
        echo "${abi} signatures: spawn_timeout=${spawn_to} warmup_fail=${warm} other_fail=${other}"
    } | tee "$LOG_DIR/${abi}-summary.txt"
}

run_one vst3-fault.exe vst3 &
run_one clap-fault.exe clap &
wait

echo "done — ${LOG_DIR}/"
