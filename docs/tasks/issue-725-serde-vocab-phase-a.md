---
id: "TASK-034YZZNkTftEQV1BWT8vGc"
type: "task"
status: "in-progress"
title: "Issue #725 serde 語彙 Phase A — task"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-725-serde-vocab-phase-a.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-725-serde-vocab-phase-a.md"
  implements:
    - "docs/plans/issue-725-serde-vocab-phase-a.md"
  implemented-by:
    - "docs/impl/ir/issue-725-serde-vocab-phase-a.md"
  depends-on:
    - "docs/plans/issue-725-serde-vocab-phase-a.md"
    - "docs/discovery/issue-725-serde-vocab-phase-a.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-725-serde-vocab-phase-a.md"
---

# Issue #725 serde 語彙 Phase A — task

## Work

`EnvironmentKind::RealAbiTestPlugin` の serde 語彙 gap に対する
Phase A slice: deserialize 側 alias 受容 + consumer 影響評価記録 +
移行方針の設計文書化 + issue-629 docs の status 追記。

- [x] T1: consumer 影響評価 → `evidence/rf725/consumer-impact-
  assessment.md`（audio-lens・synvelis read-only 照合）
- [x] T2: discovery → spec → design（移行方針決定・記録）
- [x] T3: `metadata.rs` に `#[serde(alias = "test-plugin")]` 付与
  + enum doc の gap 記述を Phase A 状態に更新
- [x] T4: `tests/metadata.rs` に alias 受容 pin 追加
  （`EnvironmentKind` 単体 + `RunMetadata` JSON）・
  :190-192 の deferred コメントを Phase A 記録に更新
- [x] T5: issue-629 docs 5 ファイルへ status 追記（append-only）
- [x] T6: plan → task → test-spec → impl/ir・exp 記録
- [ ] T7: 正規検証 — fmt-check / check / lint / test /
  test-ci-consumer-verify / prek run --all-files
- [ ] T8: review-protocol レビュー・blocker 解消
- [ ] T9: Conventional Commits でコミット・push・PR 作成

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-725-serde-vocab-phase-a.md`
- `docs/specs/issue-725-serde-vocab-phase-a.md`
- `docs/designs/issue-725-serde-vocab-phase-a.md`
- `docs/test-specs/issue-725-serde-vocab-phase-a.md`
- `docs/discovery/issue-725-serde-vocab-phase-a.md`

## Dependencies

- なし（正式 blockedBy なし）。rf#631 占有は emit flip（第2 slice）
  のみをブロックし本 slice をブロックしない。

## Affected Files

- `crates/ruviform-audio-test/src/metadata.rs`
- `crates/ruviform-audio-test/tests/metadata.rs`
- `crates/ruviform-audio-test/Cargo.toml`（条件付き — 変更した場合は
  PR 本文に理由明記）
- `docs/designs|specs|test-specs/issue-629-*.md`（追記のみ）
- `evidence/rf725/consumer-impact-assessment.md`（新規）
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,
  impl/exp}/issue-725-serde-vocab-phase-a.md` + `impl/exp/*-exp.jsonl`
  （新規）

## Done When

- [ ] alias 受容 pin テストが pass し、emit 綴り不変が維持される
- [ ] consumer 影響評価・移行方針・issue-629 status 追記が記録済み
- [ ] 正規検証（check/lint/test/fmt-check/test-ci-consumer-verify/
  prek）が全て pass
- [ ] 「gap 解消済み」と誤記しない（emit flip は第2 slice と明記）

## Verification

- [ ] `mise run fmt-check` — pass
- [ ] `mise run check ruviform-audio-test` — pass
- [ ] `mise run lint ruviform-audio-test` — pass
- [ ] `mise run test ruviform-audio-test` — pass
- [ ] `mise run test-ci-consumer-verify` — pass（schema 非回帰）
- [ ] `mise exec -- prek run --all-files` — pass
