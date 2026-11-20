---
id: "TSPEC-630clapOutputDrain01"
type: "test-spec"
status: "approved"
title: "Issue #842 CLAP output-drain slice — test specification"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-clap-output-drain.md"
    - "docs/designs/issue-630-clap-output-drain.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-clap-output-drain.md"
  implements:
    - "docs/specs/issue-630-clap-output-drain.md"
    - "docs/designs/issue-630-clap-output-drain.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-fault-modes.md"
  related:
    - "docs/test-specs/issue-630-fault-modes.md"
    - "evidence/rf630/clap-output-drain-verification.md"
  verifies:
    - "docs/specs/issue-630-clap-output-drain.md"
    - "docs/designs/issue-630-clap-output-drain.md"
---

# Issue #842 CLAP output-drain slice — test specification

## Feature: `BoundedOutputBuffer`（`host.rs` unit test）

- **Rule**: `try_push` は `MAX_EVENTS_PER_BLOCK` 件まで受理し、
  それ以降は全て `Err(TryPushError)` — 再確保しない。
  **Example**: 256 push 後の 2 連続 push が Err、`take_refused`
  が 2 を返し block ごとに reset される。
- **Rule**: `clear` は capacity を保持して次 block の受理上界を
  reset する（`clear` 後の push は受理される）。

## Feature: `translate_output_event`（`adapt.rs` unit test）

- **Rule**: `ParamValue`（既知 id）は `EventKind::ParamValue` +
  `frame_offset` 保持で翻訳される — plain `1.0` ∈ `[0,2]` →
  normalized `0.5`。
- **Rule**: `u32::MAX` スタンプは `frame_offset` に忠実に届く
  （faithful-stamp 法則）。
- **Rule**: 未知パラメータ id と非有限値は defined skip
  （`None`）— 値の捏造はしない。
- **Rule**: `NoteOn`/`NoteOff`（`Match::Specific` channel/key）は
  `EventKind::Midi` の note kind へ翻訳 — channel・key・velocity
  （`0.5` → u7 `64`）・group（`Specific(p)` → `MidiGroup(p)`、
  `All` → `ZERO`）が保持される。wildcard channel/key 対象と
  非有限 velocity は `None`。
- **Rule**: `Midi`/`Midi2` は入力パス同一の MIDI 翻訳へ委譲
  （channel-voice → `EventKind::Midi`）。
- **Rule**: `NoteChoke`/`ParamGestureBegin`/`ParamMod` 等、
  canonical report payload を持たない kind は `None`。

## Feature: host_path（in-process 実 ABI、`tests/host_path.rs`）

- **Rule**: `fault_bad_time_output_event_is_host_observable` —
  normalized `0.75` arm + 1 block で `drain_output_reports` が
  丁度 1 レポートを返す: `frame_offset == u32::MAX`、
  `EventKind::ParamValue`、`ParamId(0)`、normalized `0.0`。2 回目
  の drain は空（重複しない）。
- **Rule**: `fault_invalid_output_event_is_defined_skip` — まず
  `bad_time` レポートを drain で観測して drain が live である
  ことを証明し、normalized `0.875` arm の block 後に drain が
  空を返す（NaN は defined skip — `OutputEvents::void()` では
  この証明が原理上不可能）。

## Feature: L1 E2E（`tests/l1_fault_test_plugin_e2e.rs`、`--ignored`、
`mise run test-l1-real-plugin` が選択）

- **Rule**: `real_test_fault_l1_output_events_drain` —
  `PluginHost` + 実 runner 経由で `test_fault` を processing へ
  駆動し:
  1. `bad_time` arm で `TAIL_INFINITE` claim が 2 つの matched
     block に着弾するまで poll（armed 判定は claim の着弾 —
     arm イベントは子の `process()` で非同期に発火するため
     matched≠armed）。
  2. `host.drain_output_reports` → `Ok(ReportDrain::Drained)`、
     レポート数 ≥2、全て `frame_offset == u32::MAX` +
     `EventKind::ParamValue` + `ParamId(0)` + 値 `0.0`。
  3. `invalid` arm で matched all-NaN block を 2 つ着弾させ、
     flush drain + 追加 2 NaN block の後に drain が空
     （遷移窓の `bad_time` straggler は flush で分離）。
  4. lifecycle は `Processing` を維持し、disarm →
     StopProcessing/Deactivate/Destroy が clean。

## Feature: RT・回帰面

- **Rule**: `host::tests::rt_process_drives_fixture_block_
  allocation_free` — `assert_no_alloc` 窓内の `process` が永続
  バッファ経路でも alloc-free（`mise run test-rt`）。
- **Rule**: 既存テスト（`bad_time` TAIL claim、`invalid` NaN
  block、全 spawn/fault lane）が無改悪で通る — wire/機構層は
  zero-diff。
