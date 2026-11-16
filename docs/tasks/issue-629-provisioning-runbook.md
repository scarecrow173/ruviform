---
id: "TASK-W4O6BsG8WBPnXtsyy5hX8Y"
type: "task"
status: "done"
title: "Issue #629 provisioning runbook tasks"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/plans/issue-629-provisioning-runbook.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-provisioning-runbook.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-provisioning-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-provisioning-runbook.md"
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-provisioning-runbook.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-provisioning-runbook.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 provisioning runbook tasks

## Work

Issue #629 closeout-prep slice の実装 slice。AC6 残部の
`test-midi-virtual-endpoint` opt-in lane 登録と lane 説明
文書化、AC4/AC5/AC7 の headless 記録（provisioning
runbook・capability matrix 確定版・consumer 照合）、stale
参照の同期、3 lane self-run の fail-closed evidence 記録を
行う。

- T1: `mise.toml` に `[tasks.test-midi-virtual-endpoint]`
  を追加 + `test-midi-real-device` の description/コメント
  同期 + `docs/development-environment.md` の lane 表・
  opt-in 注記へ追記。
- T2: `loopback.rs` の doc 参照更新 + wasapi-endpoint・
  virtual-endpoint-lane 両 design の Deferred 同期 +
  survey doc Evidence 節への dated entry 追記。
- T3: 検証 — fmt/check/lint/test（`ruviform-midi-device`）・
  3 lane self-run・`prek`・`evidence/rf629/` 記録。
- T4: docs 仕上げ — impl ir/exp（+ `*-exp.jsonl`）・
  review-protocol。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-629-provisioning-runbook.md`
- `docs/specs/issue-629-provisioning-runbook.md`
- `docs/designs/issue-629-provisioning-runbook.md`
- `docs/test-specs/issue-629-provisioning-runbook.md`
- `docs/discovery/issue-629-virtual-endpoint-survey.md`

## Dependencies

- 前提 slice（merge 済み）: survey+capability probe（PR
  `scarecrow173/ruviform#640`）、`midi_output` port + WinMM
  `midiOut*` backend（PR `scarecrow173/ruviform#658`）、
  MIDI loopback round-trip verifier（PR
  `scarecrow173/ruviform#663`）、WASAPI streaming verifier
  （PR `scarecrow173/ruviform#673`）、
  `test-audio-virtual-endpoint` lane 登録 +
  `EnvironmentKind` 語彙補完（PR `scarecrow173/ruviform#680`）。
- Coordinator 確定: lane の `run` は `cargo test -p
  ruviform-midi-device --test loopback -- --ignored
  --test-threads=1`（target 全体収集・feature flag なし）。
  専用 lint lane は追加しない（既定 `lint
  ruviform-midi-device` が coverage）。
  `test-midi-real-device` は run 不変 + description 同期。

## Affected Files

- `mise.toml`
- `docs/development-environment.md`
- `crates/ruviform-midi-device/tests/loopback.rs`
- `docs/specs|designs|plans|tasks|test-specs|
  issue-629-provisioning-runbook.md`（本 chain）
- `docs/designs/issue-629-wasapi-endpoint.md`
- `docs/designs/issue-629-virtual-endpoint-lane.md`
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
- `docs/impl/ir|exp/issue-629-provisioning-runbook.md`
  （+ `*-exp.jsonl`）
- `evidence/rf629/`

## Done When

- [x] `mise.toml` に `test-midi-virtual-endpoint` が登録
  され、provisioning 必須・manual・never CI・収集対象
  2 テストの依存差が description に明記される。専用 lint
  lane は追加されない。
- [x] `test-midi-real-device` の run が不変で、
  description/注記が包括 lane と仮想 pair 専用 lane の
  関係を正直に記述する。
- [x] `docs/development-environment.md` に両 MIDI lane の
  行・注記が追記される。
- [x] `loopback.rs` の doc 参照が canonical lane
  `mise run test-midi-virtual-endpoint` に更新され、
  挙動は不変。
- [x] provisioning runbook・capability matrix 確定版・
  AC7 consumer 照合が design doc に記録される。
- [x] 両 design の Deferred と survey doc Evidence が
  本 slice の解消範囲に同期される。
- [x] 3 lane self-run が `evidence/rf629/` に記録され、
  Pass を主張しない。
- [x] `fmt-check`/`check`/`lint`/`test`
  （`ruviform-midi-device`）・`prek` が Pass。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check ruviform-midi-device`
- [x] `mise run lint ruviform-midi-device`
- [x] `mise run test ruviform-midi-device`
- [x] `mise run test-midi-real-device`（退行確認）
- [x] `mise run test-midi-virtual-endpoint`（self-run —
  未 provision で `verdict: NotRun`、evidence 記録）
- [x] `mise run test-audio-virtual-endpoint`（既存 lane
  挙動不変確認）
- [x] `mise exec -- prek run --all-files`
