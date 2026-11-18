#!/bin/bash
cd "$(dirname "$0")"
mkdir -p logs2
for i in $(seq 1 60); do
  ./bin/e2e-a.exe --ignored > "logs2/iter-$i-e2e-a.log" 2>&1 &
  PA=$!
  ./bin/e2e-b.exe --ignored > "logs2/iter-$i-e2e-b.log" 2>&1 &
  PB=$!
  wait $PA; RA=$?
  wait $PB; RB=$?
  echo "iter $i: a=$RA b=$RB"
done
