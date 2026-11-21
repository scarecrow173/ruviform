#!/bin/bash
# rf767 observation loop — dual-binary concurrent run of the two
# instrumented l1_real_test_plugin_e2e binaries (clap + vst3) with
# RUVIFORM_L1_PROBE=1 so the temporary probes print per-call counter
# deltas and per-drop branch attribution. Canonical verification is
# `mise run test-l1-real-plugin`; this loop is auxiliary evidence.
cd "$(dirname "$0")"
export RUVIFORM_L1_PROBE=1
mkdir -p obs-logs
for i in $(seq 1 30); do
  ./bin/e2e-clap.exe --ignored --nocapture >"obs-logs/iter-$i-clap.log" 2>&1 &
  PA=$!
  ./bin/e2e-vst3.exe --ignored --nocapture >"obs-logs/iter-$i-vst3.log" 2>&1 &
  PB=$!
  wait $PA
  RA=$?
  wait $PB
  RB=$?
  echo "iter $i: clap=$RA vst3=$RB"
done
