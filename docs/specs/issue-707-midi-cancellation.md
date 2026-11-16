---
id: "SPEC-034YJhpMyzjNwAu0RkAPam"
type: "spec"
status: "approved"
title: "Issue #707 — MIDI input path への cancellation fault 拡張"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/707"
    - "docs/specs/issue-628-close-out.md"
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-707-midi-cancellation.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-707-midi-cancellation.md"
  depends-on:
    - "docs/specs/issue-628-close-out.md"
    - "docs/designs/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-707-midi-cancellation.md"
    - "docs/test-specs/issue-707-midi-cancellation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-close-out.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-707-midi-cancellation.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #707 — MIDI input path への cancellation fault 拡張

## What Should Be Built

Issue #628 close-out slice で audio stream（`StreamFault::Cancellation`、
merge `0b510cf8` / PR `scarecrow173/ruviform#669`）に追加した
cancellation fault を、MIDI input path の fault 体系へ同型拡張する。

`crates/ruviform-audio-core/src/midi_input/mock.rs` の
`MockPort`/`MockInput` fixture に read-level fault injector を追加し、
`MidiInput::read → Err(DeviceError::Backend("cancelled"))`（配送中断・
非配送）+ 後続 read の `Err(DeviceError::StreamInvalidated)`
（subscription 終了）という observable を deterministic に再現する。
`crates/ruviform-audio-test` 側では `MidiLeg`（`pump_inputs`/
`pump_next`/`pump_block`）経由で `PipelineError::Device` として
観測する正負 test を追加する。

## Why Now

#628 AC3 の棚卸し（`evidence/rf628/ac-inventory.md` Deferred 表）で
「MIDI 側 fault への cancellation 拡張」が owner なし残件として
`scarecrow173/ruviform#707` に起票された。本 Issue がその追跡対象を
解消する。audio 側の写像規約（production 契約に cancel 概念がない
ため既存 observable — aborted delivery + `Backend("cancelled")` +
stream 終了 — で表現）は確立済みであり、MIDI 側も同じ規約に倣う。

## Users and Value

- consumer harness（AudioLens/Synvelis）が MIDI input 経路の
  cancel/中断系障害を deterministic backend 上で再現できる。
- `ruviform-audio-plugin-bridge` の `midi_wiring` test や
  plugin-test e2e など `MockPort` の他 consumer も同じ fault 面を
  利用できる。

## Scope

### In Scope

- `crates/ruviform-audio-core/src/midi_input/mock.rs`:
  `MockPort` への read-level fault injector 追加
  （`fail_next_read` — 1-shot 非終端 read error、
  `inject_cancelled` — cancelled read + subscription 終了）。
  production trait（`MidiInput`/`MidiInputPort`/`MidiInputEnumerator`）
  は不変。
- `crates/ruviform-audio-test/tests/` への正負 test 追加
  （新規 `tests/midi_faults.rs` — `MidiLeg` 経由の pipeline 観測）。
- `mock.rs` 内 `#[cfg(test)]` unit test の追加。
- 本 slice の docs chain（spec→design→plan→tasks→test-specs→
  impl/ir→impl/exp）と `evidence/rf707/` の検証記録。

### Out of Scope

- audio stream 側の cancellation — #628 close-out で完了済み
  （`src/backend.rs` の `StreamFault`・`src/backend/stream.rs`・
  `tests/backend_faults.rs` は read-only 先例）。
- `MidiFeed`/`MidiLeg`/`InputPump`/`InputStage` 等の production
  経路・配線コードの変更 — 既存の `DeviceError` 伝播で observable
  を再現できるため変更しない（設計判断は design doc に記録）。
- 実 MIDI デバイス・実 endpoint 上の fault 実 timing（EXT-L3・
  `scarecrow173/ruviform#629`/`#630` 領域 — NotRun 簿記のみ）。
- `crates/ruviform-midi-device/` の変更（#629 占有・read-only 参照）。
- `mise.toml`/`docs/development-environment.md`・CI lane 追加・
  `.github/**`（新規 `tests/*.rs` は既存 `test` lane が自動収集）。
- 依存追加・`Cargo.lock` 再生成。
- Issue の Close（PR 本文は `Refs #707` のみ）。

## Requirements

- cancellation の写像は #628 close-out と同じ規約に従う:
  production 契約（`MidiInput::read` の `DeviceResult`・
  `DeviceError`）に新 variant・新 API を追加せず、既存 observable
  （read error + subscription 終了 → 後続 `StreamInvalidated`）で
  表現する。test-only 境界を維持する。
- 負例として「`Backend("cancelled")` を値に持つ 1-shot 非終端
  read error では subscription が終了しない」ことを検証できる
  面が必要（`fail_next_read` が担う）。
- armed fault の消費順は決定的（FIFO）とし、ordered error 報告を
  test で pin する。
- 実行していない検証を Pass と書かない。実 MIDI デバイス上の
  fault 実 timing は EXT-L3 として NotRun 簿記のみ行う。

## Acceptance Criteria

- [ ] MIDI input path で cancellation fault が expected/observed
  一致する正負 test が `crates/ruviform-audio-test` に追加されて
  いる（deterministic OS-L1 範囲）。
- [ ] `mise run test ruviform-audio-test` / `check` / `lint` で
  収集・実行される。
- [ ] 対応する docs chain（spec/design/plan/tasks/test-specs/impl）
  が追加されている。
- [ ] 実 MIDI デバイス上の fault 実 timing は EXT-L3 として本
  Issue では NotRun 扱い（#629/#630 領域）。

## Deferred Design Concerns

- cue 単位の cancel（`MidiFeed` の位置 X 以降の cue 未配送）—
  subscription-level cancel がこれを内包する（cancel 後の cue は
  port queue に滞留し read されない）ため、本 slice では fault と
  しては要求しない | reason: cue 未配送は scenario 内容であり
  fault ではない。port-level cancel の observable で同じ結末を
  観測できる | re-engagement trigger: consumer が cue 粒度の
  中断 semantics を要求した時点 | risk if ignored: なし（観測面は
  既存で足りる）。
- 実 MIDI デバイス・実 endpoint 上の fault 実 timing — EXT-L3・
  `scarecrow173/ruviform#629`/`#630` 領域 | reason: 仮想優先受入
  方針（POLICIES.md §2.1）で物理機材・実 endpoint は通常必須で
  はない | re-engagement trigger: #629/#630 の EXT-L3 lane 整備 |
  risk if ignored: 実 backend の cancel timing 差異は本 fixture の
  範囲外として明示済み。

## Sources and Relations

- Issue `scarecrow173/ruviform#707`（本 slice の起票 Issue）。
- #628 close-out の写像規約・正負 test 先例:
  `docs/specs/issue-628-close-out.md`（Deferred 表の本件行）、
  `docs/designs/issue-628-scripted-backend.md`、
  `crates/ruviform-audio-test/src/backend.rs`（`StreamFault::
  Cancellation`）、`src/backend/stream.rs`、
  `tests/backend_faults.rs`。
- `evidence/rf628/ac-inventory.md`（Deferred 表 — 本 Issue が
  解消対象の行）。
- `POLICIES.md`（OS-L1/GUI-L2/EXT-L3、Required/Deferred/
  NotApplicable × Pass/Fail/NotRun/Unsupported）。
- 実装対象: `crates/ruviform-audio-core/src/midi_input/mock.rs`、
  `crates/ruviform-audio-test/tests/`。
