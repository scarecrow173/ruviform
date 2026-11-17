---
id: "PLAN-034YZYv4eOit5JDno5u0gR"
type: "plan"
status: "approved"
title: "Issue #725 serde 語彙 Phase A — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-725-serde-vocab-phase-a.md"
  implements:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  implemented-by:
    - "docs/tasks/issue-725-serde-vocab-phase-a.md"
  depends-on:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  blocks: []
  supersedes: []
  derives-from:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  verified-by:
    - "docs/test-specs/issue-725-serde-vocab-phase-a.md"
---

# Issue #725 serde 語彙 Phase A — 実装計画

## Goal

`EnvironmentKind::RealAbiTestPlugin` の serde 語彙 gap に対し、
deserialize 側 alias 受容（Phase A）を実装し、consumer 影響評価・
移行方針・issue-629 docs status を記録する。

## Scope

単一 slice。emit flip は第2 slice（rf#631 占有解除後）に分離
されるため本 plan の対象外 — design の「第2 slice の作業範囲」は
記録のみ。

## Upstream Documents

- `docs/specs/issue-725-serde-vocab-phase-a.md`（approved）
- `docs/designs/issue-725-serde-vocab-phase-a.md`（approved）
- `docs/discovery/issue-725-serde-vocab-phase-a.md`
- `evidence/rf725/consumer-impact-assessment.md`（AC1 記録済み）

## Design Inputs

- `docs/designs/issue-725-serde-vocab-phase-a.md` — 移行期間中の
  挙動表・第2 slice 作業範囲・却下案。

## File Map

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-audio-test/src/metadata.rs` | `RealAbiTestPlugin` に `#[serde(alias = "test-plugin")]` 付与・enum doc の gap 記述を Phase A 状態に更新。`as_str()`/emit 不変 |
| `crates/ruviform-audio-test/tests/metadata.rs` | alias 受容 pin テスト追加（enum 単体 + `RunMetadata` JSON）。:190-192 の deferred コメントを Phase A 記録に更新 |
| `crates/ruviform-audio-test/Cargo.toml` | 変更なし見込み（serde derive 既存）。変更した場合は理由を PR 本文に明記 |
| `docs/designs/issue-629-virtual-endpoint-lane.md` | gap 記述へ rf#725 Phase A status を追記（append-only） |
| `docs/designs/issue-629-provisioning-runbook.md` | 同上 |
| `docs/specs/issue-629-virtual-endpoint-lane.md` | 同上 |
| `docs/specs/issue-629-provisioning-runbook.md` | 同上 |
| `docs/test-specs/issue-629-virtual-endpoint-lane.md` | 同上 |
| `evidence/rf725/consumer-impact-assessment.md` | 作成済み（AC1） |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-725-serde-vocab-phase-a.md` + `impl/exp/*-exp.jsonl` | docs chain（本書含む） |

## Implementation Sequence

- [x] consumer 影響評価 → `evidence/rf725/consumer-impact-assessment.md`
- [x] discovery → spec → design（移行方針決定）
- [ ] `metadata.rs` に `#[serde(alias = "test-plugin")]` + enum doc 更新
- [ ] `tests/metadata.rs` に alias 受容 pin 追加・deferred コメント更新
- [ ] issue-629 docs 5 ファイルへ status 追記（append-only）
- [ ] plan → task → test-spec → 実装検証 → impl/ir・exp 記録
- [ ] 正規検証（check/lint/test/fmt-check/test-ci-consumer-verify/prek）
- [ ] review-protocol レビュー → commit/push/PR

## Dependencies and Risks

- **依存**: なし（正式 blockedBy なし・open PR なし）。rf#631 占有は
  emit flip のみをブロックし Phase A をブロックしない。
- **リスク**: alias が serialize 出力を変える誤解 — serde の仕様上
  alias は deserialize 側のみ。pin テストで emit 不変を固定する。
- **リスク**: issue-629 docs の「追記のみ」制約違反 — 既存記述の
  書き換え・削除を行わず、status 追記のみに限定する。

## Task Breakdown

`docs/tasks/issue-725-serde-vocab-phase-a.md` に展開（T1〜T10）。

## Verification

- [ ] `mise run fmt-check` — pass
- [ ] `mise run check ruviform-audio-test` — pass
- [ ] `mise run lint ruviform-audio-test` — clippy `-D warnings` pass
- [ ] `mise run test ruviform-audio-test` — alias pin 含む全 test pass
- [ ] `mise run test-ci-consumer-verify` — schema 側非回帰
- [ ] `mise exec -- prek run --all-files` — 軽量チェック pass
- [ ] `*-full`/`*-msrv`/3OS 横断は daily CI 委譲（NotRun として記録）

## Open Questions / Gaps

- なし。emit flip のタイミングは rf#631 lease 状況に依存する
  Coordinator 調整事項として design/PR に記録済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch 内で完結）
- Independent work streams: none
- User approval required before dispatch: no（Orca worker として
  Spec に従い自律実行）
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- reviewer focus: `#[serde(alias)]` が emit を変えないことの pin・
  issue-629 docs が追記のみであること・「解消済み」誤記がないこと・
  移行期間中の挙動表の正確性。
