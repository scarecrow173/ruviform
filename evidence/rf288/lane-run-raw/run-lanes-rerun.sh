#!/usr/bin/env bash
# rf288 lane re-run: lanes whose salvaged lanes.log lacks exit= records
# (test, test-rt, test-io-msrv) plus check/lint whose recorded failures
# show infrastructure signatures (rustc "metadata stub" cache corruption /
# empty-output exit=66) and need a clean re-run for an honest verdict.
cd "C:/Users/akiyoshi/orca/workspaces/ruviform/rf288-gaterun" || exit 1
LOG=evidence/rf288/lane-run-raw/lanes-rerun.log
: > "$LOG"
run() {
  local name="$1"; shift
  echo "===== $name :: $* =====" | tee -a "$LOG"
  "$@" >> "$LOG" 2>&1
  local ec=$?
  echo "===== $name exit=$ec =====" | tee -a "$LOG"
}
export CARGO_BUILD_JOBS=2
run check mise run check
run lint mise run lint
run test mise run test
run test-rt mise run test-rt
run test-io-msrv mise run test-io-msrv
echo "ALL DONE" | tee -a "$LOG"
