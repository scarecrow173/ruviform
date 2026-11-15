---
id: "TASK-034YboiLqA86ymomdjc56s"
type: "task"
status: "done"
title: "Issue #629 WASAPI streaming verifier tasks"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/plans/issue-629-wasapi-endpoint.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-wasapi-endpoint.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-wasapi-endpoint.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-wasapi-endpoint.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-wasapi-endpoint.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-wasapi-endpoint.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 WASAPI streaming verifier tasks

## Work

Issue #629 4th slice の実装タスク（plan T1–T4 に対応）:

- T1: `crates/ruviform-audio-device/tests/virtual_endpoint.rs`
  新設 — `#![cfg(feature = "real-device")]` + `#[ignore]` の
  WASAPI render→capture streaming verifier。
- T2: `crates/ruviform-audio-device/examples/endpoint_probe.rs`
  拡張 — pair candidate 表・driver 署名/権限/cleanup の
  best-effort 記録・verdict 文言更新。
- T3: 検証 — fmt/check/lint/test・`lint-audio-real-input`・`prek`・
  probe/verifier の Windows 実機実行・`evidence/rf629/` 記録。
- T4: docs 仕上げ — impl ir/exp（+ `*-exp.jsonl`）・discovery
  更新・review-protocol。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-629-wasapi-endpoint.md`
- `docs/specs/issue-629-wasapi-endpoint.md`
- `docs/designs/issue-629-wasapi-endpoint.md`
- `docs/test-specs/issue-629-wasapi-endpoint.md`
- `docs/discovery/issue-629-virtual-endpoint-survey.md`

## Dependencies

- 前提 slice（merge 済み）: `endpoint_probe.rs`（1st）・
  `tests/loopback.rs`（3rd）・`tests/real_input_e2e.rs`（#251）。
- coordinator 判断: `ruviform-audio-test` dev-dep 不承認 — oracle
  はテスト内独立 generator で自足、#628 結線 IF は docs 記録。

## Affected Files

- `crates/ruviform-audio-device/tests/virtual_endpoint.rs`（新設）
- `crates/ruviform-audio-device/examples/endpoint_probe.rs`
- `docs/specs|designs|plans|tasks|test-specs/issue-629-wasapi-
  endpoint.md`（本 chain）
- `docs/impl/ir|exp/issue-629-wasapi-endpoint.md`（+ `*-exp.jsonl`）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
- `evidence/rf629/`

## Done When

- [x] verifier が production port API のみで pair candidate を
  選定・open・stream し、独立 oracle（marker/level/tolerance）で
  照合する。pair 未 provision 環境では fail-closed `NotRun`。
- [x] probe が pair candidate 表と driver 署名/権限/cleanup の
  best-effort 記録を出力し、verdict 文言が実装と一致する。
- [x] `fmt-check`/`check`/`lint`/`test`/`lint-audio-real-input`/
  `prek` が Pass。probe/verifier の実機実行結果が
  `evidence/rf629/` に記録される。
- [x] impl ir/exp・discovery 更新・lane 設計・#628 結線 IF が
  docs に記録される。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check ruviform-audio-device`
- [x] `mise run lint ruviform-audio-device`
- [x] `mise run test ruviform-audio-device`
- [x] `mise run lint-audio-real-input`
- [x] `mise exec -- prek run --all-files`
- [x] `mise exec -- cargo run -p ruviform-audio-device --example
  endpoint_probe`（Windows 実機、exit 0）
- [x] `cargo test -p ruviform-audio-device --features real-device
  --test virtual_endpoint -- --ignored --test-threads=1`
  （補助証拠 — 未 provision では `verdict: NotRun`）
