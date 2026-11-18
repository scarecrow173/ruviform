#!/bin/bash
# Post-fix looped verification for ruviform#763 — dual-binary concurrent
# runs (auxiliary evidence; canonical = mise run test-l1-real-plugin).
# iters 1-30: normal concurrent dual-binary.
# iters 31-40: same + a concurrent `cargo build` load lane.
cd "$(dirname "$0")/bin-fixed"
mkdir -p ../logs3 ../logs4
for i in $(seq 1 40); do
  if [ "$i" -gt 30 ]; then
    LOGDIR=../logs4
    # artificial load: concurrent release-profile check of the workspace
    # member builds (CPU + linker pressure) for the duration of the iter
    (cd /g/orca/workspaces/ruviform/leatherjacket && \
      mise exec -- cargo build -p ruviform-plugin-test-clap -p ruviform-plugin-test-vst3 --features l1-runner --tests >/dev/null 2>&1) &
    LOADPID=$!
  else
    LOGDIR=../logs3
    LOADPID=
  fi
  ./e2e-a.exe --ignored > "$LOGDIR/iter-$i-e2e-a.log" 2>&1 &
  PA=$!
  ./e2e-b.exe --ignored > "$LOGDIR/iter-$i-e2e-b.log" 2>&1 &
  PB=$!
  wait $PA; RA=$?
  wait $PB; RB=$?
  if [ -n "$LOADPID" ]; then wait $LOADPID; fi
  echo "iter $i: a=$RA b=$RB"
done
