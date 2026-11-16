---
id: "PLAN-lbYjQBxlWJgpBMrj1gEUoI"
type: "plan"
status: "approved"
title: "Issue #629 provisioning runbook plan"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-629-provisioning-runbook.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-provisioning-runbook.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-provisioning-runbook.md"
    - "docs/tasks/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-provisioning-runbook.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 provisioning runbook plan

## Goal

Issue #629 closeout-prep slice: `test-midi-virtual-endpoint`
opt-in lane の `mise.toml` 登録と lane 説明文書化（AC6
残部）、provisioning runbook + capability matrix 確定版 +
AC7 consumer 照合の headless 記録（AC4/AC5/AC7 残部）、
stale 参照同期、3 lane self-run の fail-closed evidence
記録を完了する。

## Scope

単一 implementation stream。lane 登録・docs chain・
runbook/matrix/AC7 記録・evidence は同一 PR で論理コミット
に分けて納める。実 provisioning・lane の CI 統合・実走
Pass・consumer 実受入は後続 slice / 外部 gate へ hand-off
（spec Out of Scope と同一境界）。

## Upstream Documents

- `docs/specs/issue-629-provisioning-runbook.md`
- `docs/designs/issue-629-provisioning-runbook.md`（approved
  — runbook 本体・matrix 確定版・AC7 照合の正本）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
  （候補比較・暫定 matrix・Open Questions・Evidence 節）
- `docs/designs/issue-629-virtual-endpoint-lane.md`（5th
  slice — audio lane 登録の先例）
- `docs/designs/issue-629-wasapi-endpoint.md`（4th slice —
  AC6 lane 設計記録・verifier 契約）
- `docs/plans|tasks|test-specs|docs/impl/*/
  issue-629-midi-loopback.md`（3rd slice — loopback
  verifier）
- `POLICIES.md` / Issue `scarecrow173/ruviform#627`（Case
  schema・`environment_kind`）

## Design Inputs

- `docs/designs/issue-629-provisioning-runbook.md` — lane
  構成（`--test loopback` 全体収集の判断根拠）・lint
  coverage 判断・`test-midi-real-device` description 同期
  方針・runbook/matrix/AC7 の記録内容。

## File Map

- `mise.toml` - `[tasks.test-midi-virtual-endpoint]` 追加
  （`test-midi-real-device` 直後）+ 同 task の
  description/コメント同期。
- `docs/development-environment.md` - lane 一覧表に
  `test-midi-real-device`・`test-midi-virtual-endpoint` 行
  追加 + opt-in lane 注記 bullet 追加。
- `crates/ruviform-midi-device/tests/loopback.rs` - module
  doc・`loopback_round_trip` doc comment の canonical lane
  参照更新（挙動不変）。
- `docs/designs/issue-629-provisioning-runbook.md` - design
  + runbook 本体 + matrix 確定版 + AC7 照合（作成済み）。
- `docs/designs/issue-629-wasapi-endpoint.md`・
  `docs/designs/issue-629-virtual-endpoint-lane.md` -
  Deferred Design Concerns の同期。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` -
  Evidence 節へ 2026-11-17 dated entry 追記。
- `docs/specs|designs|plans|tasks|test-specs|
  issue-629-provisioning-runbook.md` + `docs/impl/ir|exp/
  issue-629-provisioning-runbook.md`（+ `*-exp.jsonl`）-
  doc chain。
- `evidence/rf629/` - 3 lane self-run 記録
  （`midi_virtual_endpoint_slice6_selfrun-windows-local.txt`
  等）。

## Implementation Sequence

- [ ] T1: `mise.toml` lane 登録 + `test-midi-real-device`
  description 同期 + `docs/development-environment.md`
  追記。
- [ ] T2: `loopback.rs` doc 参照更新 + wasapi-endpoint・
  virtual-endpoint-lane 両 design の Deferred 同期 +
  survey doc Evidence 追記。
- [ ] T3: 検証 — fmt/check/lint/test（`ruviform-midi-
  device`）・3 lane self-run・`prek`・`evidence/rf629/`
  記録。
- [ ] T4: docs 仕上げ — impl ir/exp・exp jsonl・
  review-protocol。

## Dependencies and Risks

- `loopback.rs` の変更は doc 参照のみ — `#[ignore]`・
  verdict 文言・テスト本体は不変。`--test loopback` の
  2 テスト収集は design で判断記録済み（name filter の
  fail-open 回避）。
- lane self-run は未 provisioning で `verdict: NotRun` —
  Pass を主張しない。evidence は lane 実行ログ +
  `--nocapture` 補助証拠の 2 部構成（5th slice 先例）。
- survey doc Evidence 節は append-only — 既存 entry を
  編集せず新規 dated entry で記録。
- runbook/matrix は「provisioning 実測待ち」を消さない —
  ❓/⚠️・Open Questions #1-#7 を明記。
- docs index（`docs/**/README.md`）は生成物 — 本 slice
  では再生成せず Coordinator 統合時に委譲（PR 本文に
  明記）。

## Task Breakdown

- T1 → T2 → T3 は独立だが同一 PR。T4 は全検証後。

## Verification

- [ ] `mise run fmt-check` — Pass。
- [ ] `mise run check|lint|test ruviform-midi-device` —
  Pass。
- [ ] `mise run test-midi-real-device`（退行確認）—
  記録。
- [ ] `mise run test-midi-virtual-endpoint`（self-run）—
  exit 0 + `verdict: NotRun` を `evidence/rf629/` に記録。
- [ ] `mise run test-audio-virtual-endpoint`（既存 lane
  挙動不変確認）— 記録。
- [ ] `mise exec -- prek run --all-files` — Pass。
- [ ] MSRV lane — NotRun（daily CI 管轄）。

## Open Questions / Gaps

- 採用 endpoint の確定・実 provisioning・実走 Pass・
  `.github/**` 統合・AC3 typed 結線・consumer 実受入 —
  後続 slice / 外部 gate / consumer 責務。
- capability matrix の ❓/⚠️ 項目 — provisioning 実測待ち
  （runbook §8 の実測待ち項目で収集予定）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology
  are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatch worker が
  直接実装）。
- Independent work streams: none（逐次実施）。
- User approval required before dispatch: yes →
  review-protocol の blocker なしをもって承認とみなす
  （worker 契約）。
- If approved: discover available implementation and
  delegation capabilities in the current environment,
  then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the
  same dependency graph and verification matrix.

## Review Handoff

- reviewer は spec AC の headless 範囲と fail-closed 境界
  （self-run NotRun を Pass にしない・runbook が候補を
  決め打ちしない・matrix の実測待ち明記・AC7 が consumer
  Pass を主張しない）を重点確認。`mise.toml` の task が
  既存 lane を壊さないこと、`--test loopback` 収集範囲の
  判断記録、`test-midi-real-device` が run 不変であること
  も確認対象。
