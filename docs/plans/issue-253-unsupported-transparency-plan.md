---
id: "PLAN-034VERRPegu2dLBq0VqYol"
type: "plan"
status: "approved"
title: "Implement Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane scaffold"
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
  implements:
    - "docs/specs/issue-253-unsupported-transparency.md"
    - "docs/designs/issue-253-unsupported-transparency-design.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-253-unsupported-transparency-design.md"
    - "docs/test-specs/issue-253-unsupported-transparency.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-253-task-unsupported-transparency.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-unsupported-transparency-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-unsupported-transparency.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issues: [253]
---

# Implement Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane scaffold Implementation Plan

## Goal

`MidiReadStats.unsupported` を bridge の全計数面（`PumpStats`/
`InputTx`/`InputStageCounters`/`NodeStatus` snapshot）まで透過させ、
`midi_input::mock` に `inject_unsupported` 注入面を追加し、
`#[ignore]` 実機 E2E（midi-device + bridge 経路）と evidence 記録
で Issue #253 の「実デバイス/opt-in 検証 lane」scaffold を整備
する。

## Scope

単一実装ストリーム — counter 透過（mock→bridge→status mirror）+
deterministic test + `#[ignore]` 実機 E2E + evidence。canonical
lane（`mise.toml`）登録・macOS/Linux backend・実 DAW・`Verified`
昇格・V9-V13・video verb・MIDI2・consumer 実装は対象外。

## File Map

| File | Change |
| --- | --- |
| `crates/ruviform-audio-core/src/midi_input/mock.rs` | `PortState.unsupported` + `inject_unsupported` + `read` が `with_unsupported` を返す + unit test |
| `crates/ruviform-audio-plugin-bridge/src/input.rs` | `InputStageCounters::unsupported_backend` + `read` 累積、`PumpStats::backend_unsupported` + `InputTx::unsupported_backend` + getter |
| `crates/ruviform-audio-plugin-bridge/src/status.rs` | `input_unsupported_backend` atomic + `set_input`/`snapshot` + mirror unit test |
| `crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs` | `ScriptedInput.unsupported` + `read` 計数透過 test（ST-01/02） |
| `crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs` | mock 経路 test 拡張（TX-01/02/03）+ `#[ignore]` 実機 E2E（RL-01/02/04） |
| `crates/ruviform-audio-plugin-bridge/Cargo.toml` | dev-dep `ruviform-midi-device` |
| `crates/ruviform-midi-device/tests/ports.rs` | `#[ignore]` E2E 全経路拡張（RL-01/02/03） |
| `crates/ruviform-midi-device/README.md` | opt-in 入口（暫定コマンド）記録 |
| `evidence/issue-253-real-midi-lane/` | 実機 lane 実行結果 or `NotRun`+理由 |
| `docs/impl/ir/issue-253-unsupported-transparency-slice.md` | 実装記録（暫定入口・follow-up 明記） |
| `Cargo.lock` | dev-dep 追加に伴う再生成（生成物） |

## Task Breakdown

### T1 — mock 注入面（UT-01/02）

1. `mock.rs`: `PortState` に `unsupported: u64` を追加し
   `MockPort::inject_unsupported(n)` を実装、`MockInput::read` が
   `MidiReadStats::with_unsupported(take(dropped), take(unsupported))`
   を返すよう変更。module doc の注入面記述を更新。
2. `mock.rs` unit test: `inject_unsupported` が一度だけ surface
   し、`dropped` と独立であることを検証。

### T2 — bridge counter 透過（ST/TX/NS）

1. `input.rs`: `InputStageCounters::unsupported_backend` を
   `dropped_backend` 直後に追加、`InputStage::read` で累積。
   `PumpStats::backend_unsupported` 追加、`InputTx` に
   `unsupported_backend` field + `unsupported_backend()` getter、
   `pump` で累積 + `PumpStats` へ格納。doc 更新。
2. `status.rs`: `input_unsupported_backend: AtomicU64` を
   `input_dropped_backend` 直後に追加、`set_input`/`snapshot` に
   mirror 行追加。`#[cfg(test)]` で mirror unit test（NS-01）。
3. `tests/midi_delivery.rs`: `ScriptedInput` に `unsupported`
   field（全 literal 更新）、`with_unsupported` 返却、ST-01/02
   test 追加。
4. `tests/midi_wiring.rs`: `mock_backend_drives_input_stage_to_
   plugin` に `inject_unsupported` + TX-01/02/03 assertion 追加
   （`backend_unsupported`・`unsupported_backend()`・status
   mirror 0 境界）。

### T3 — 実機 E2E + dev-dep（RL-*）

1. `ruviform-midi-device/tests/ports.rs`: `real_device_open_read_
   close` を全経路へ拡張（enumerate→skip→open→複数 read→stamp
   検証→close→`StreamInvalidated`、観測値は eprintln 記録）。
2. bridge `Cargo.toml`: dev-dep `ruviform-midi-device`。
3. `tests/midi_wiring.rs`: `#[ignore]` 実機 E2E — `SystemBackend`
   →port→open→複数 `input_tx.pump`（計数一貫性 assert）→process→
   mirror 記録→close→`StreamInvalidated`。
4. `crates/ruviform-midi-device/README.md`: opt-in 暫定入口
   `cargo test -p ruviform-midi-device -- --ignored` を記録。

### T4 — evidence + 文書チェーン + 検証

1. `cargo test -p ruviform-midi-device -- --ignored` /
   `cargo test -p ruviform-audio-plugin-bridge -- --ignored` を
   実行し、結果を `evidence/issue-253-real-midi-lane/` に記録
   （実機あれば実測値、無ければ `NotRun`+理由）。暫定入口と
   canonical lane follow-up を IR に明記。
2. `mise run fmt-check` / `check` / `lint` / `test` / `test-rt`
   で全 lane 検証。`mise exec rust@1.89.0 -- cargo test -p
   ruviform-audio-core -p ruviform-audio-plugin-bridge -p
   ruviform-midi-device` を MSRV 補助証拠として実施（canonical
   lane `test-msrv` は本 workspace に無いため補助扱い）。
3. review-protocol レビュー → IR 作成 → commit/push/PR。

## Dependency Graph

```text
T1 (mock) ──▶ T2 (bridge counters + tests)
T1 ──▶ T3 (実機 E2E — mock 不要だが TX 系の bridge dev-dep が T2 に依存)
T2 ──▶ T3
T3 ──▶ T4
```

T1→T2 は mock の注入面が test の前提。T3 の bridge 実機 E2E は
dev-dep と `unsupported_backend()` getter（T2）に依存。

## Risk Register

| Risk | Impact | Mitigation |
| --- | --- | --- |
| `#[non_exhaustive]` field 追加で既存 test 破損 | Low | `status.rs::snapshot` の literal 構築箇所のみ更新対象。`cargo check` で即検出 |
| `ScriptedInput` field 追加で既存 literal 全滅 | Low | 同ファイル内の構造体 — 全構築箇所を一括更新 |
| bridge dev-dep による dep cycle | None | midi-device は bridge に依存しない（audio-core/media-core のみ） |
| 実機 E2E が環境依存で fail | Med | RL-01 で skip/Unsupported を honest 処理。hardware traffic は assert しない |
| `mise.toml` 不可により canonical lane 未登録 | Accepted | 暫定入口を IR/README に明記 + follow-up 行（→ 解消: `chore/253-midi-real-device-lane` slice で `test-midi-real-device` 登録済み） |

## Rollback Strategy

全変更は additive — revert は commit 単位。`Cargo.lock` は
`git checkout --` で戻せる。既存意味論の変更は `mock.rs::read`
の返却値（`unsupported=0`→注入値）のみで、注入しない呼出では
挙動不変。

## Verification Matrix

| AC | 検証 |
| --- | --- |
| unsupported 計数透過 | UT/ST/TX/NS deterministic test + `mise run test` |
| opt-in 実機経路 | `cargo test -p ruviform-midi-device -- --ignored` / `-p ruviform-audio-plugin-bridge -- --ignored` + evidence |
| canonical lane 未登録の明示 | IR + README + PR 本文の follow-up 行（→ lane 登録済み: `test-midi-real-device`） |
| 既存 lane 非回帰 | `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` |
| 対象外の明示 | spec/IR の Out of Scope + PR 本文 |
