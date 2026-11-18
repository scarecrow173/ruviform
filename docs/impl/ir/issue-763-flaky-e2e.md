---
id: "IMPL-034ZkB2vQsRwT6xYzIcD"
type: "impl"
status: "completed"
title: "Issue #763 l1_real_test_plugin_e2e instrument tests flaky — test oracle 修復"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-763-flaky-e2e.md"
    - "docs/discovery/issue-763-flaky-e2e.md"
    - "evidence/rf763/verification.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-763-flaky-e2e.md"
      - type: "file"
        path: "docs/impl/exp/issue-763-flaky-e2e.md"
      - type: "file"
        path: "docs/impl/exp/issue-763-flaky-e2e-exp.jsonl"
      - type: "file"
        path: "evidence/rf763/reproduction.md"
      - type: "file"
        path: "evidence/rf763/verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "docs/test-specs/issue-630-test-instrument.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-763-flaky-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-763-flaky-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-763-flaky-e2e.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: ["docs/impl/exp/issue-763-flaky-e2e-exp.jsonl"]
    rejected: []
---

# Issue #763 l1_real_test_plugin_e2e instrument tests flaky — test oracle 修復

## Summary

両 adapter crate の `l1_real_test_plugin_e2e` test file を、
`PluginHost::audio_counters` の per-call delta による
matched/substituted 分類へ改修した。production code は一切変更
していない（test-file-only slice）。

## 実装内容

### 共有 helper（各 file 内・crate 間共有 util なし）

- `host_block_transport` が `(Result<ProcessResult>, LegDeltas)` を
  返すよう変更 — `process` 前後で `audio_counters(handle)` を読み、
  `passthrough_submits` / `dropped_submits` の delta を記録。
- `LegDeltas { passthrough, dropped }`、`L1_LOOKAHEAD = 2`、
  `RELEASE_ARRIVED_BLOCKS = 6`、`NOTE_OFF_RESEND_CALLS = 8`、
  `arrived_between(drops, a, b)`（`(a, b]` の arrived block 数 =
  span − drops 合計）を追加。

### 直接経路 test

- `note_lifecycle`: matched block のみ `delivered` 計上・content
  assert。note-on は毎 call 再送されるため自己修復 — resend 不要。
- `note_off_releases_to_silence`: call 8 から `NOTE_OFF_RESEND_CALLS`
  までの窓で NoteOff を `dropped_submits` delta = 0 になるまで
  resend（最初の着地で打ち切り — child へ届く NoteOff は高々 1 個）。
  `drops[call]` を全 call で記録し、tail（calls 36..=43）は matched
  かつ `arrived_between(landed, source) >= 6` の block のみ
  silence assert + `tail_verified >= 2` gate。未着地なら
  `off_landed = None` → gate が正直に fail。
- `cc_and_offset_gating`: 毎 call 全 event 再送で自己修復。
  matched block は `source = call − L1_LOOKAHEAD` の phase で
  assert — muted source（< 8）は全 landed block が CC7=0 のため
  無条件 silence、full source（≥ 8）は block 自身が CC7 max +
  offset-100 note-on を運ぶため `[100] == -1`・dual mono・
  post-offset determinism を assert。run を 16→22 call に延長し
  matched sample を確保。`verified >= 2` gate は不変。
- `transport_clicks`: substituted block は stale phase の click を
  replay し得るため全面 skip。matched block は source phase
  （`source / PHASE_CALLS`）で期待を決定 — stopped source は
  silence、playing source はその phase の click frame
  （100/500/250）を assert。4 phase × 12 call = 48 call に延長。
  matched all-zero in click phase は child defect として正直に
  fail（旧 all-zero skip が mask していた）。

### attach 経路（`attach_integration` mod）

- `AttachRig` に `host` + `handle` を保持（`audio_counters` アクセス用）。
- `pump_block` が `(Vec<Vec<f32>>, LegDeltas)` を返す。
- `attach_midi_note_off_releases_to_silence`: NoteOff cue を tick
  1500+512k（block 2..=9）に resend — 各 cue は別 block の event
  list に乗り、落ちた block の分は次 block が再武装。最後に着地
  した block（`drops[j]==0` の最大 j）を release 起点とし、matched
  block のうち `source > off_landed` かつ `arrived_between >= 6` の
  ものを silence assert（`tail_verified >= 2`）。`input.mapped` /
  `delivered` の期待値は 9（note-on + resend 8）に更新 — node 側
  delivery の証明としては強化。
- `attach_midi_note_on_*` / `attach_midi_cc7_*`: 断言は substituted
  replay に対して既に安全（first non-silent は必ず attack block
  またはその replay、cc7 は全 silent 期待）なため `.0`/`_deltas`
  で delta を破棄 — 分類追加は note-off のみに限定。

### review-protocol round（Reviewer A 指摘への対応）

- **attach liveness assert 追加**: mirror `Failed` / endpoint `Err`
  の defined-silence 経路は `passthrough_submits` を increment
  しないため、child 死亡が「matched silence」として tail /
  cc7-mute assert を false-pass し得た（direct test は teardown
  `HostCommand` unwrap が `Err(Failed)` で救済済み）。attach 3
  test 全てに `assert_liveness`（`lifecycle_state == Processing`
  を assert — `rt_state` mirror は driver が child exit を
  `Failed` に収束させる live 値）を追加。
- **attach note-on sustained count を matched-only 化**:
  `last_output` replay 連鎖でも `>= 2` を満たし得たため、
  `deltas[i].passthrough == 0` の non-silent block のみ計上。
- **spec R2 の陳腐化修正**: 「test は単一 NoteOff のみ送る」が
  resend 実装と矛盾していたため R7 参照へ更新。

### review-protocol round 2（Reviewer B2 指摘への対応）

- **direct note-off に liveness assert 追加**: attach 側と同じ
  機構 — `Failed` leg は passthrough counter が動く前に defined
  silence を返すため、child 死亡後の block が「matched」として
  tail oracle を vacuous に pass し得た。teardown の
  `HostCommand` unwrap が最終的には失敗させるが、assert を
  明示して早期に正直に落とす。driver の exit-poll 収束
  （~200µs）をカバーする 5ms settle sleep を挟む。
- **`off_landed` の resend window gate**: resend window 終了後も
  `off_pending` が残り、後続の non-dropped submit（`empty`
  events）が `off_landed` を誤記録し得た。landing 判定を
  `8..8 + NOTE_OFF_RESEND_CALLS` に限定 — window 内で landed
  しなければ `off_landed = None` のまま tail oracle が正直に
  失敗する。
- **`arrived_between` の `a > b` guard**: attach twin と同じ
  `source <= landed → continue` を direct にも追加
  （現状 landed ≤ 15 < source ≥ 34 で到達不能だが、helper に
  内部 guard がないため parity のために明示）。

### spec doc

- `docs/test-specs/issue-630-test-instrument.md`: R7 を
  substituted-output semantics（matched/substituted 分類・exact
  source 帰属・bounded resend）に書き換え、Risk 節に substituted
  output 誤 oracle・dropped submit event 喪失・NoteOff 再送窓の
  3 項目を追加/更新。`updated:` を 2026-11-18 に更新。

## 設計上の判断

- **window 維持 → exact source-phase attribution への pivot**:
  初版は spec の「window 維持 + 分類追加」最小整合変更を採用したが、
  post-fix 40-iter loop で `verified >= 2` gate 割れが 3 件観測された
  （window 内 matched sample が substitution burst 下で不足）。
  `next_block` が drop 時も seq を消費するため seq 空間 = call 空間
  が保たれ、matched ⇒ source = call − 2 が厳密に決まる — window は
  帰属の曖昧さを吸収するためのものだったので、exact attribution に
  切り替えて window を撤去した。assert 対象は全 matched block に
  拡大（強化方向）、gate 値・assert 内容は不変、run 長のみ延長して
  matched sample を確保した。同一 loop が 40/40 green に回復
  （`evidence/rf763/window-variant-failures.md`・`logs3/`・`logs4/`）。
- **resend の意味論**: fixture law は NoteOff 毎に release を再武装
  するため、直接経路は「最初の着地で打ち切り」（child へ届く
  NoteOff 高々 1 個）、attach 経路は「窓内 cue の最後に着地した
  ものが最終 release を開始」で law-abiding。
- **arrived-block 判定**: dropped submit は seq を消費するが child
  には届かないため、release の進行は call 数ではなく arrived block
  数で測る必要がある — `drops[]` の per-call 記録から算出。

## 残件

- `dropped_submits` の周期的発生は production 側の調査候補
  （submit channel / slot busy）— test-file-only slice の範囲外。
- `l1_fault_test_plugin_e2e.rs` / `l1_stateful_test_plugin_e2e.rs`
  の同 staleness class は未精査 — lease 外のため残件化。
