# rf767 reproduction — dropped_submits の発生条件と帰属

補助証拠(canonical = `mise run test-l1-real-plugin`)。環境は Windows OS-L1。

## 観測 instrumentation(一時 probe — commit 前に除去済み)

counter は drop 条件を区別しないため、以下の env-gated(`RUVIFORM_L1_PROBE`)probe で帰属を取った。`ruviform#763` の一時 instrumentation と同じ方針。

- `crates/ruviform-plugin-host/src/l1/data.rs` — 2 drop 分岐に `probe_drop(kind, seq, slot, last_pushed, cleared)`。`kind` は `slot-busy` または `chan-full`。
- `crates/ruviform-plugin-test-{clap,vst3}/tests/l1_real_test_plugin_e2e.rs` — `host_block`/`host_block_transport`/attach `pump_block` の per-call probe。`passthrough_submits`/`dropped_submits`/`results_failed_observed`/`results_dropped` の delta + timestamp。

## 再現手順

1. probe 差分を適用した状態で test binary を build:

   ```sh
   mise exec -- cargo test -p ruviform-plugin-test-vst3 -p ruviform-plugin-test-clap \
     --features l1-runner --test l1_real_test_plugin_e2e --no-run
   ```

2. fixture cdylib が必要(初回は canonical lane `mise run test-l1-real-plugin` で build される)。
3. 生成された `target/debug/deps/l1_real_test_plugin_e2e-*.exe` を `evidence/rf767/bin/e2e-{clap,vst3}.exe` として stage。
4. `./loop-observe.sh` — 両 binary を並行で 30 iter 実行し `obs-logs/iter-*-{clap,vst3}.log` に記録。
   - **注意: `--nocapture` が必須。** rust test harness は pass した test の stderr を捨てるため、`--nocapture` なしでは probe 出力が残らない(本調査で初回 30 iter が無出力だった原因)。
   - `RUVIFORM_L1_PROBE=1` は script 内で export 済み。

## 結果(60 run、全件 pass)

`counter-summary.txt` に集計。要約:

- drop 1,885 件、**全 60 run で発生**(run あたり 28〜35 件)。
- 帰属: `slot-busy` 1,884 件 / `chan-full` 0 件(1 件は並行出力の行間欠落と判断 — per-call `drop=1` 行と 1:1 対応)。
- `results_failed_observed` = 0、`results_dropped` = 0(全 12,299/12,297 call)。
- drop seq は `3..9` に最多(各 test 系列の先頭 D..2D ブロック = child spawn + 初回 shm 往復の transient)、`~0x2b` まで裾を引く jitter 成分。

probe サンプル(`iter-5-vst3.log` より):

```text
[rf767-probe] slot-busy seq=0x8000000000000003 slot=2 last_pushed=Some(
    0x8000000000000000,
) cleared=0x0 t=Instant { ... }
[rf767] block pt=0 drop=1 failed_obs=0 results_dropped=0 t=Instant { ... }
```

`last_pushed=Some(seq0)`・`cleared=0` → slot 2 の前 occupant(seq 0)の completion が未着 = child round-trip が slot 再利用窓(D=3 ブロック)を超過、という直接証拠。

## 判定

drop は child process の処理・スケジューリング遅延時に bounded slot pipeline が seq を消費しつつ substitute output を返す **queue shedding** であり、module doc の decision table が定義する挙動と一致。`cleared` 追跡・`mark_failed` 合成・result channel の不整合は観測されず、**設計意図**と判定。契約は `docs/specs/issue-767-submit-drop.md`。
