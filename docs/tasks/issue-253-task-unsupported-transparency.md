---
id: "TASK-034VEU3ZiDLQ7qcsk69rl0"
type: "task"
status: "in-progress"
title: "Issue #253 unsupported 計数透過 + 実機 lane scaffold（bridge counter + mock + tests + E2E）"
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
    - "docs/plans/issue-253-unsupported-transparency-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-253-unsupported-transparency-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-253-unsupported-transparency-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-253-unsupported-transparency.md"
  verified-by:
    - "docs/test-specs/issue-253-unsupported-transparency.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 unsupported 計数透過 + 実機 lane scaffold（bridge counter + mock + tests + E2E）

## Work

Issue #253 残存受入の 2 件を実装する: (a) `MidiReadStats.
unsupported` の bridge 計数透過（先行 2 IR の「別 slice」ギャップ
解消）、(b) 実デバイス/opt-in 検証 lane の scaffold（`#[ignore]`
E2E 全経路 + evidence）。`mise.toml` canonical lane 登録は別
Worker 占有のため対象外 — 暫定入口を IR/README に記録する。

- [ ] T1 `mock.rs`: `PortState.unsupported` +
      `MockPort::inject_unsupported` + `MockInput::read` の
      `with_unsupported` 返却 + unit test（UT-01/02）。
- [ ] T2 `input.rs`/`status.rs`: `PumpStats::backend_unsupported`・
      `InputTx::unsupported_backend` 累積+getter・
      `InputStageCounters::unsupported_backend` + `read` 累積・
      `NodeStatus::input_unsupported_backend` mirror +
      `#[cfg(test)]` mirror test（ST/TX/NS）。
- [ ] T2 `tests/midi_delivery.rs` + `tests/midi_wiring.rs`:
      `ScriptedInput.unsupported` 拡張と deterministic 透過 test
      （ST-01/02・TX-01/02/03）。
- [ ] T3 `ruviform-midi-device/tests/ports.rs`: `#[ignore]` E2E
      拡張（enumerate→skip→open→複数 read→stamp/dropped/
      unsupported 観測→close→`StreamInvalidated`）（RL-01/02/03）。
- [ ] T3 bridge `Cargo.toml` dev-dep `ruviform-midi-device` +
      `tests/midi_wiring.rs` に `#[ignore]` 実機 E2E
      （`SystemBackend`→`InputTx::pump`→`InputStage`、計数一貫性
      assert + close→`StreamInvalidated`）（RL-01/02/04）。
- [ ] T3 `crates/ruviform-midi-device/README.md`: opt-in 暫定入口
      `cargo test -p ruviform-midi-device -- --ignored` の記録。
- [ ] T4 `evidence/issue-253-real-midi-lane/`: 実機 lane 実行結果
      または `NotRun`+理由。暫定入口・canonical lane follow-up
      を IR `docs/impl/ir/issue-253-unsupported-transparency-slice.md`
      に明記。
- [ ] T4 検証: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`
      全 Pass + MSRV 補助（`mise exec rust@1.89.0`）+
      review-protocol 実施。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-253-unsupported-transparency-plan.md`
- `docs/specs/issue-253-unsupported-transparency.md`
- `docs/designs/issue-253-unsupported-transparency-design.md`
- `docs/test-specs/issue-253-unsupported-transparency.md`

## Dependencies

- 前提実装済み: `ruviform-midi-device`（PR #561）、
  `MidiReadStats::with_unsupported`（同）、`midi_input::mock`（同）、
  `InputTx`/`InputStage`/`NodeStatus` mirror 機構（Issue #188）。
- ブロッカーなし。`mise.toml` は別 Worker 占有 — canonical lane
  登録は follow-up 行として残す。

## Notes

- channel 経路（`InputTx::pump`）と direct-read 経路
  （`InputStage::read`）で計数 owner が別れるのは `dropped` と
  同一の既存責務境界 — TX-03 で `snapshot().input.
  unsupported_backend == 0`（channel 経路時）を assert して固定。
- 実機依存値（event 有無・非零計数）は assert せず evidence に
  記録する（hardware traffic を Pass 条件にしない）。
