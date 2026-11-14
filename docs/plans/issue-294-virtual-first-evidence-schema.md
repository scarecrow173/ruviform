---
id: "PLAN-034XC0f0Wxlvtq0vaQJb2t"
type: "plan"
status: "approved"
title: "Issue #294 first slice — consumer-verify Case schema v2 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-294-virtual-first-evidence-schema.md"
      - type: "doc"
        path: "docs/tasks/issue-294-virtual-first-evidence-schema.md"
      - type: "doc"
        path: "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
      - type: "doc"
        path: "docs/impl/ir/issue-294-virtual-first-evidence-schema.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  implemented-by:
    - "docs/tasks/issue-294-virtual-first-evidence-schema.md"
    - "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
  depends-on:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-294-virtual-first-evidence-schema.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-294-virtual-first-evidence-schema.md"
  derived-by:
    - "docs/tasks/issue-294-virtual-first-evidence-schema.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
  references:
    - "ci/consumer-verify/evidence.mjs"
    - ".github/workflows/consumer-verify.yml"
  defers: []
  deferred-by: []
---

# Issue #294 first slice — consumer-verify Case schema v2 実装計画 Implementation Plan

## Goal

`ci/consumer-verify/` の lane/case record を virtual-first Case schema（`case_id`/`level`/`environment_kind`/`requirement`/optional Oracle・fixture・resolved SHA）へ additive に拡張し、evidence JSON を `schemaVersion: 2`（`contract: consumer-verify-tuple/v1` 不変）へ上げる。workflow の全 lane 記録行が新項目を正直に宣言し、`readLanes` の fail-closed 検証と contract test で非破壊を pin する。

## Scope

単一の実装 stream — `evidence.mjs` schema 拡張 → workflow lane 行更新 → contract test 拡張 → 検証 → doc chain 仕上げ、の順序依存が強いため分割しない。編集範囲は `ci/consumer-verify/`・`.github/workflows/consumer-verify.yml`・本 slug の doc chain・`evidence/rf294/` に限定。

## Upstream Documents

- `docs/specs/issue-294-virtual-first-evidence-schema.md`
- `docs/specs/issue-294-cross-consumer-candidate-gate.md`（先行 spec — 証拠契約の拡張元）

## Design Inputs

- `docs/designs/issue-294-virtual-first-evidence-schema.md`
- `docs/designs/issue-627-virtual-first-verification-contract-design.md` §2 Case schema（正本）
- `docs/designs/issue-627-policies-revision-proposal.md` ゲート移管台帳 RF #294/#281 行

## File Map

- `ci/consumer-verify/evidence.mjs` - lane record schema v2・`SCHEMA_VERSION` 2・CLI flag・`readLanes` fail-closed 語彙検証
- `ci/consumer-verify/tests/evidence.test.mjs` - schema v2 の fixture test 拡張
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` - workflow case 宣言構造の contract test 拡張
- `.github/workflows/consumer-verify.yml` - `rec()` helper の case spec 化・全 lane 行・extra task lane・fallback `schemaVersion: 2`
- `evidence/rf294/` - 検証記録・判断メモ（任意）
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-294-virtual-first-evidence-schema.md` - doc chain

## Implementation Sequence

- [ ] T1: `evidence.mjs` — `LEVELS`/`ENVIRONMENT_KINDS`/`REQUIREMENTS` export、`SCHEMA_VERSION` 2、`makeLaneRecord` 拡張、`lane` CLI flag、`readLanes` fail-closed 検証
- [ ] T2: `.github/workflows/consumer-verify.yml` — 両 job の `rec()` を `"<case_id>|<level>|<environment_kind>|<requirement>"` spec 化、全 lane 行に case_id/分類を付与、extra task lane 更新、fallback `schemaVersion: 2`
- [ ] T3: `ci/consumer-verify/tests/` — evidence.test.mjs（必須性・語彙・直交・optional 型・旧 artifact 意味不変・readLanes 拒否）と yml contract test（rec spec 形状・非既定分類 pin・fallback 版）を拡張
- [ ] T4: 検証 — `mise run test-ci-consumer-verify`、`mise exec -- actionlint .github/workflows/consumer-verify.yml`、補助として `node --test ci/`（tracker consumer の非破壊確認）、`mise exec -- prek run --all-files`
- [ ] T5: doc chain 仕上げ — impl/ir・impl/exp（+ companion jsonl）・管理 index 再生成（`doc_maintenance.js index apply`・`sync_impl_index.js`）・`evidence/rf294/` 記録

## Dependencies and Risks

- `ci/tracker` contract test が `deriveStatus`/`OUTCOMES` を import — signature・語彙の非破壊が必須（test で確認）。
- `ci/daily/decide.mjs` は `schemaVersion`/`lanes` を読まない — `contract` v1 固定で回復経路は不変（test + コード確認済み）。
- workflow の記録経路は3系統（`rec()`・extra task 直接 `lane`・fallback `printf`）— 全て同時更新しないと finalize が fail する。yml contract test で構造 pin。

## Task Breakdown

- T1-T3 は同一 commit 群で実装（schema と producer/consumer を同時に整合させる）。T4 は各変更後に反復。T5 は実装確定後。

## Verification

- [ ] `mise run test-ci-consumer-verify` pass
- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml` pass
- [ ] `node --test ci/`（補助・canonical = mise）で tracker/daily の consumer 側 contract test 非破壊を確認
- [ ] `mise exec -- prek run --all-files` pass
- [ ] `docs/*/README.md` index が再生成済みで `doc-status` lint が新規 error を出さない

## Open Questions / Gaps

- `rec_cand`（resolved_ruviform_sha 付き helper）か spec 拡張か — design で「実装時に簡潔な方」とした。実装で決定し IR に記録。
- Issue レベル AC（tuple binding・Oracle・fixture version・実走）は本 slice 対象外 — spec/design の deferred 表に記録済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker、単一 stream）
- Independent work streams: none
- User approval required before dispatch: yes → review-protocol のレビューで blocker なしを承認として扱う
- If approved: 現在の worktree で task-by-task に実行。

## Review Handoff

- reviewer focus: `contract` v1 固定の維持、`requirement`×`outcome` 直交の schema 強制、workflow 全記録経路の case 宣言網羅性、非既定分類（GUI-L2/EXT-L3）の正直さ。
