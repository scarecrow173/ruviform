---
id: "DISC-034Zk9QmXvBnM4sTuWgYjA"
type: "discovery"
status: "resolved"
title: "Issue #763 l1_real_test_plugin_e2e instrument tests flaky — frame-exact PCM assertion の断続 fail 診断"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/763"
    - "crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs"
    - "crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs"
    - "crates/ruviform-plugin-host/src/l1/data.rs"
    - "crates/ruviform-plugin-host/src/composite.rs"
    - "docs/test-specs/issue-630-test-instrument.md"
    - "evidence/rf630/abi-validator-rerun-verification.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/tasks/issue-763-flaky-e2e.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-instrument.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/tasks/issue-763-flaky-e2e.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #763 l1_real_test_plugin_e2e instrument tests flaky — 診断

## Summary

`l1_real_test_plugin_e2e` の instrument 系 test（両 adapter crate）が
負荷下で frame-exact PCM assertion により断続 fail する
（`evidence/rf630/abi-validator-rerun-verification.md:159-171` で
実観測 — 失敗 test は run ごとに異なり clap/vst3 両方）。診断の結果、
**production defect ではなく test oracle の設計欠陥**と確定した。
機構は 2 系統ある:

1. **substituted output（機構 1）**: L1 leg は output 不達時に
   `pass_through` で `last_output`（最後に matched した child
   output）を再生する（`data.rs` decision table、`:65-68`）。
   `have_output` 以後の underrun は全 zero ではなく**任意に古い
   非零 block** になり得るため、phase window を跨いだ stale replay
   が release tail / click frame の assert を踏む。旧 spec R7 の
   「underrun（全 zero block）は skip」はこの事実を見落としていた。
2. **dropped submit（機構 2）**: `dropped_submits` が立つ call は
   input block — queue 済み event を含む — が child に未到達。
   単発 NoteOff が落ちると voice が sustain し、**matched block
   （`passthrough_submits` delta = 0）でさえ oracle に反する出力**
   になる。counter 分類だけでは検出できない第 2 の欠陥経路。

## Exploration Goal

flaky の root cause を「timing/buffer 境界の仮定 vs 実 defect」に
切り分け、owning layer（test oracle）で決定的な修正方針を確定する。

## Findings

- `next_block` は process call ごとに単調増加し、submit drop 時も
  seq を消費する（`data.rs:683-711`）— **seq 空間 = call 空間**で
  あり、matched block（`expected` の result が drain された call）
  は source call `call − lookahead_blocks`（= 2）の child rendering
  に厳密帰属できる。
- `PluginHost::audio_counters(handle)` が `passthrough_submits` /
  `dropped_submits` 等の `AudioCounterSnapshot` を公開
  （`composite.rs:845`）— `host.process` 前後の delta で 1 call =
  高々 +1 の分類が決定的に取れる（非 RT API・同一 thread で安全）。
- 60 iteration の dual-binary 負荷 loop（補助証拠）で 12 run
  failure を再現。内訳: `note_off_releases_to_silence` ×7
  （`dropped_submits` による NoteOff 喪失が主因、一部は substituted
  replay）、`transport_clicks` ×6（全て substituted stale replay）。
  `cc_and_offset_gating` / `note_lifecycle` の失敗も観測。
- drain は stale seq（`seq < expected`）を discard するため、
  matched と見える結果が落ちる production defect 経路は走査上
  確認されなかった — hold-last replay は `data.rs` module docs の
  decision table どおりの設計意図。

## Decision

Coordinator 承認の **案 A**: test oracle を counter 分類 +
上限付き resend-until-landed に改修する。

- content assert は matched block（`passthrough_submits` delta = 0）
  のみに適用。substituted block は分類して skip（最低到達数 gate・
  assertion は一切緩めない）。
- 永続 state event（単発 NoteOff）は `dropped_submits` delta = 0
  になるまで resend — fixture law が NoteOff 再送で同一 release
  を再武装するため law-abiding。上限 `NOTE_OFF_RESEND_CALLS = 8`、
  未着地なら正直に fail（無限 loop で別欠陥を隠さない）。
- release tail の silence assert は「landed note-off 以後の
  **arrived block 数**」で rendered space の release 完了を判定
  （drop が seq 空間を引き伸ばすため call 数では不足）。
- attach 経路は `AttachRig` に `host`+`handle` を保持し
  `pump_block` で per-block delta を記録。NoteOff cue を block 毎に
  窓内 resend し、最後に着地した block を release 起点とする。

## Residual

- `dropped_submits` が負荷下で周期的に発生する現象自体は
  production 側の調査候補（submit channel / slot busy の頻度）—
  本 slice は test-file-only であり、PR 本文「未対応の作業」に
  記録する。
- `l1_fault_test_plugin_e2e.rs` / `l1_stateful_test_plugin_e2e.rs`
  （両 crate）に同 staleness class の content-inferred assert が
  あるかは未精査 — lease 外のため残件化。
