# ruviform#763 再現記録 — l1_real_test_plugin_e2e instrument tests flaky

## 目的

`l1_real_test_plugin_e2e` の instrument 系 test が負荷下で
frame-exact PCM assertion により断続 fail する defect
（`ruviform#763`）を再現し、失敗機構を分類する。

## 方法

- 未修正コード（`origin/main` `e4eb117f`）で build した
  `l1_real_test_plugin_e2e` test binary（clap / vst3 の 2 本）を
  `bin/` に配置し、`loop2.sh` で 60 iteration・両 binary 同時実行
  （dual-binary concurrent — out-of-process L1 child の負荷を
  相互に掛ける補助証拠。canonical 検証は `mise run
  test-l1-real-plugin`）。
- 各 binary は `--ignored` で 10 test 全件を実行。各 run の
  stdout/stderr は `logs2/iter-<N>-e2e-<a|b>.log`。

## 結果

- 120 binary-runs 中 **12 run failure**（約 10%）。
- 失敗 test: `note_off_releases_to_silence` ×7、
  `transport_clicks` ×5（iter-40 は両 test 失敗の 1 run）。
  clap / vst3 両 crate で観測。
- 失敗箇所は全て frame-exact content assert
  （release tail の非零 sample、click impulse frame）。

## 失敗機構の分類（一時 instrumentation による観測）

`host_block_transport` に `passthrough_submits` /
`dropped_submits` の per-call delta を記録する一時
`[rf763-repro]` eprintln を挿入して観測（commit には含めない）。

1. **substituted output**: `passthrough_submits` delta > 0 の call
   は `last_output` の stale replay を返す。負荷下では phase を
   跨いだ古い非零 block が release tail / click frame assert を
   踏む。旧 oracle は「全 zero block のみ skip」で、underrun
   （`have_output` 前は全 zero）と substituted replay
   （`have_output` 後は stale 非零）を区別できていなかった。
2. **dropped submit による event 喪失**: `dropped_submits` delta
   > 0 の call は input block（queue 済み event 含む）が child
   に未到達。単発 NoteOff が落ちると voice が sustain し、
   **matched block（`pt_delta=0`）でさえ oracle に反する出力**
   になることを iter-40-e2e-b で実証 — `pt` 分類のみでは
   不十分。

## 補足: seq 帰属の確認

`crates/ruviform-plugin-host/src/l1/data.rs:683-711`:
`next_block` は submit 試行**前**に increment され、drop 時も
seq を消費する。`expected = block − lookahead` の result が
drain された call の output は source call `call − 2` の
child rendering に厳密帰属（seq 空間 = call 空間）。

## 関連ファイル

- `logs/` — 初回 loop の生ログ（60 iter × 2 binary）
- `logs2/` — 再現 loop の生ログ（12 FAILED を含む）
- `loop2.sh` — 再現 loop script
- `window-variant-failures.md` — 修正後 window 版の残存 gate
  割れ記録（superseded、exact-attribution 版で解消）
