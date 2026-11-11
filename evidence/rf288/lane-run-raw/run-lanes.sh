#!/usr/bin/env bash
cd "C:/Users/akiyoshi/orca/workspaces/ruviform/rf288-gaterun" || exit 1
LOG=/tmp/rf288/lanes.log
: > "$LOG"
run() {
  local name="$1"; shift
  echo "===== $name :: $* =====" | tee -a "$LOG"
  "$@" >> "$LOG" 2>&1
  local ec=$?
  echo "===== $name exit=$ec =====" | tee -a "$LOG"
}
export CARGO_BUILD_JOBS=2
run fmt-check mise run fmt-check
run check-io-file mise run check-io-file
run lint-io-file mise run lint-io-file
run test-io-file mise run test-io-file
run check-io-structured mise run check-io-structured
run lint-io-structured mise run lint-io-structured
run test-io-structured mise run test-io-structured
run check-io-compression mise run check-io-compression
run lint-io-compression mise run lint-io-compression
run test-io-compression mise run test-io-compression
run check-io-e2e mise run check-io-e2e
run lint-io-e2e mise run lint-io-e2e
run test-io-e2e mise run test-io-e2e
run check mise run check
run lint mise run lint
run test mise run test
run test-rt mise run test-rt
run test-io-msrv mise run test-io-msrv
echo "ALL DONE" | tee -a "$LOG"
