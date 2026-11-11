---
id: "SPEC-034VEOYg1GvMKRp6MWAh4D"
type: "spec"
status: "approved"
title: "Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane scaffold"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-253-os-midi-backend.md"
    - "docs/specs/issue-188-timed-midi.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/impl/ir/rf-253-l1-report-verb.md"
    - "docs/impl/ir/issue-253-os-midi-input-backend-adapter-windows-winmm-os-unsupported.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-os-midi-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/251"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane scaffold

## What Should Be Built

Issue #253 の残存受入「実デバイス/opt-in 検証 lane（#114 l1-real-verify
パターン準拠）」の scaffold と、先行 2 slice の impl record が「別
slice」と明記した既知ギャップの解消を行う。

1. **bridge `unsupported` 計数の透過**: `MidiReadStats.unsupported`
   （`ruviform_audio_core::midi_input` — adapter が「受信したが
   channel-voice 語彙へ翻訳不能」と報告する計数）は現状、
   `InputTx::pump` / `InputStage::read` で読み捨てられている。
   `dropped` と同じ経路で `PumpStats`・`InputTx` 累積 getter・
   `InputStageCounters`・`NodeStatus` mirror（`status.rs`）まで
   透過させ、backend の語彙外 traffic を control/status surface
   から観測可能にする。
2. **mock の unsupported 注入手段**: `midi_input::mock` の
   `MockPort` は `inject_dropped` のみを持ち `read` が常に
   `unsupported=0` を返すため、透過経路を deterministic に検証
   できない。`inject_unsupported` を追加する。
3. **opt-in 実機検証の scaffold**: `ruviform-midi-device` の
   `#[ignore]` E2E を enumerate→open→read→timestamp/dropped/
   unsupported 観測→close→`StreamInvalidated` の全経路へ拡張し、
   bridge 側に `SystemBackend`→`InputTx::pump`→`InputStage` の
   `#[ignore]` 実機 E2E を追加する（dev-dependency として
   `ruviform-midi-device` を bridge に追加 — 依存方向は
   bridge→adapter で合法）。

## Why Now

- `MidiReadStats.unsupported` は PR #561（OS MIDI backend slice）
  で port 契約へ追加済みだが、bridge 側の透過が未実装のまま残っ
  ている。2 つの impl record がこのギャップを「別 slice」と明記
  している:
  - `docs/impl/ir/rf-253-l1-report-verb.md` L311-312 —
    「`ruviform-audio-plugin-bridge` の `unsupported` 計数透過欠落
    — 別 slice（編集範囲外、PR 本文「未対応の作業」に記録）」、
    L335-336 — 「bridge 側 `unsupported` 計数の透過欠落 —
    capability 行列の設計 slice と一緒に扱う」。
  - `docs/impl/ir/issue-253-os-midi-input-backend-adapter-windows-winmm-os-unsupported.md`
    L269-270 — 「host-graph consumer 配線（`InputTx::pump` 経路と
    の結線確認は別 slice で実施）」。
- Issue #253 受入の「実デバイス/opt-in 検証 lane」は現状
  `#[ignore]` smoke（`tests/ports.rs` の 1 test と
  `examples/list_midi_ports.rs`）のみで、観測 coverage
  （timestamp/dropped/unsupported の実測記録）が無く、canonical
  lane も未登録である。
- 「unsupported を silent にしない」は port 契約の明示要件
  （`docs/specs/issue-253-os-midi-backend.md` R 系、`queue.rs`
  module doc）であり、bridge での読み捨てはその不変条件を
  consumer 直前で破っている。

## Users and Value

- engine/consumer 実装者: `InputTx`/`InputStage`/`NodeStatus`
  snapshot 経由で backend の語彙外 traffic を観測でき、実機の
  SysEx/system traffic を「dropped（過負荷）」と区別して運用
  判断できる。
- adapter 実装者: `MidiReadStats.unsupported` を報告しても
  bridge で消えないことが契約として固定され、macOS/Linux
  backend 追加時に同じ透過経路を再利用できる。
- 検証担当: `#[ignore]` E2E と evidence dir により、実機有無を
  Pass/NotRun で正直に記録する入口ができる。

## Scope

### In Scope

- `crates/ruviform-audio-plugin-bridge/src/input.rs`:
  `PumpStats::backend_unsupported`（`#[non_exhaustive]` additive）、
  `InputTx::unsupported_backend` 累積 + getter、`InputStageCounters::
  unsupported_backend` + `InputStage::read` での累積。
- `crates/ruviform-audio-plugin-bridge/src/status.rs`:
  `input_unsupported_backend` atomic + `set_input`/`snapshot` mirror。
- `crates/ruviform-audio-core/src/midi_input/mock.rs`:
  `PortState.unsupported` + `MockPort::inject_unsupported` +
  `MockInput::read` の `with_unsupported` 返却。
- `crates/ruviform-audio-plugin-bridge/tests/`: inject→pump→
  counter 伝播の unit/integration test と `#[ignore]` 実機 E2E。
- `crates/ruviform-audio-plugin-bridge/Cargo.toml`: dev-dependency
  として `ruviform-midi-device` を追加（実機 E2E 用・dev-dep のみ）。
- `crates/ruviform-midi-device/`: `tests/ports.rs` の `#[ignore]`
  E2E 拡張、`src/` の必要最小限の整備、`README.md` の opt-in 入口
  記録。
- `evidence/issue-253-real-midi-lane/`: 実機 lane 実行結果または
  `NotRun`+理由の記録。
- 暫定入口 `cargo test -p ruviform-midi-device -- --ignored` の
  文書化（canonical lane 登録は `mise.toml` が別 Worker 占有の
  ため follow-up 行として残す）。

### Out of Scope

- `mise.toml` への canonical lane（`test-midi-real-input` 等）
  登録 — 別 Worker が同ファイルを占有中。follow-up 行として残す。
- macOS（CoreMIDI）/ Linux（ALSA・JACK・PipeWire）backend 実装。
- 実 DAW 往復、`Declared`→`Verified` の capability 昇格。
- V9 truncation / V11 ledger / V12 anchor republish / V13 CC/param
  chase の deferred contract。
- video report verb、MIDI2/UMP/MPE（Issue #317）、Synvelis/
  AudioLens 等の consumer 消費実装。
- Issue #253 の Close — 残存受入のため open 維持（PR 本文は
  `Refs #253`）。

## Requirements

- R1: `MidiReadStats.unsupported` は `InputTx::pump` と
  `InputStage::read` の両経路で累積され、`PumpStats::
  backend_unsupported`（per-call）と `InputTx::unsupported_backend()`
  / `InputStageCounters::unsupported_backend`（累積）から観測
  可能でなければならない。`dropped` 系計数とは別カウンタとする
  （unsupported は過負荷ではなく語彙設計上の除外 traffic）。
- R2: `NodeStatus` は `input_unsupported_backend` atomic を持ち、
  `set_input` が `InputStageCounters::unsupported_backend` を
  absolute store で mirror、`snapshot` が `input.unsupported_backend`
  として返さなければならない。
- R3: `MockPort::inject_unsupported(n)` は次回 `read` で
  `MidiReadStats.unsupported` に `n` を一度だけ surface させる
  （`dropped` と同じ since-last-read 消費意味論）。
- R4: 全ての新規 public field/type は additive でなければならない
  （`#[non_exhaustive]` struct への field 追加のみ。既存
  constructor・field・戻り値意味論は不変）。
- R5: `#[ignore]` E2E は MIDI input port が無い環境では skip
  （失敗ではない）し、非対応 OS では `DeviceError::Unsupported`
  を honest に記録して終了しなければならない。実デバイスが
  ある場合は open→read→close→`StreamInvalidated` を検証し、
  timestamp domain・provenance・dropped/unsupported 観測値を
  記録する。
- R6: 既存 lane（`fmt-check`/`check`/`lint`/`test`/`test-rt`）は
  回帰してはならない。

## Acceptance Criteria

- [ ] bridge 経路で `unsupported` 計数が捨てられず `PumpStats`/
  `InputTx`/`InputStage`/`status.rs` mirror まで透過すること
  （上記 2 IR の「別 slice」行を本 slice で閉じる）。
- [ ] 実デバイス opt-in 経路が `#[ignore]` E2E（enumerate→open→
  read→観測→close）として整備され、本環境で実行可能なら実測、
  不可なら `NotRun`+理由を `evidence/issue-253-real-midi-lane/`
  に記録。
- [ ] canonical lane（mise task）登録は本 slice 対象外 — 暫定入口
  `cargo test -p ruviform-midi-device -- --ignored` と follow-up
  行を docs に明記。
- [ ] 既存 lane が回帰しない（`mise run fmt-check`/`check`/`lint`/
  `test`/`test-rt` 全 Pass）。
- [ ] macOS/Linux backend・実 DAW・`Verified` 昇格・V9-V13・video
  verb・MIDI2・consumer 実装は対象外（Pass 扱いしない）。

## Deferred Design Concerns

- canonical lane（`mise.toml` の `test-midi-real-input` 相当）の
  登録 | 別 Worker が `mise.toml` を占有中のため本 slice では
  登録不可 | `mise.toml` 解放後に Issue #251 `test-audio-real-input`
  パターン（feature gate + `-- --ignored --test-threads=1` + lint
  lane）で登録 | 未登録のままでは opt-in lane が `cargo test` 直接
  実行に依存し canonical 入口と乖離する。
- `InputTx` 累積計数の `NodeStatus` への統合 mirror | 現状
  `dropped_backend` も `InputTx` 側 getter のみで status mirror
  無し — 同じ形状を踏襲（stage 側 mirror は `InputStageCounters`
  経由で既にある） | consumer が status snapshot だけで pump 側
  計数も必要になった時点 | pump 側 unsupported/dropped が control
  surface に出ないまま残る（getter で観測可能ではある）。

## Sources and Relations

- 上流契約: `docs/specs/issue-188-timed-midi.md`（R1/R3/R8 —
  `MidiInput` port と observable loss）、`docs/specs/
  issue-253-os-midi-backend.md`（`unsupported` 計数の設計由来）、
  `docs/designs/issue-253-os-midi-backend.md`（`MidiReadStats::
  with_unsupported` の追加設計）。
- 既知ギャップの出典: `docs/impl/ir/rf-253-l1-report-verb.md`
  （L311-312, L335-336）、`docs/impl/ir/issue-253-os-midi-input-
  backend-adapter-windows-winmm-os-unsupported.md`（L269-270）。
- opt-in lane パターン: Issue #114 l1-real-verify（mise task
  `test-l1-real-plugin`）、Issue #251 `test-audio-real-input`
  （`#![cfg(feature)]`+`#[ignore]`+`--test-threads=1`）。
