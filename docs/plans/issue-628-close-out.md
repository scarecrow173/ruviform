---
id: "PLAN-034Y3gHUCoEkqu0kcIyuzx"
type: "plan"
status: "approved"
title: "Issue #628 close-out — implementation plan"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-628-close-out.md"
    - "docs/designs/issue-628-close-out.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-628-close-out.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-close-out.md"
    - "docs/designs/issue-628-close-out.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-628-close-out.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-628-close-out.md"
    - "docs/test-specs/issue-628-close-out.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-628-close-out.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-628-close-out.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 close-out — implementation plan

## Goal

`ruviform-audio-test` の残存小項目（RunMetadata JSON・cancellation
fault）を実装し、docs 陳腐化修正と Issue AC 棚卸しで close-out する。

## Scope

Single plan — 2 つのコード変更は独立だが同一 crate・同一 slice の
受入単位。docs/evidence の簿記を含めて 1 plan とする。

## Upstream Documents

- `docs/specs/issue-628-close-out.md`

## Design Inputs

- `docs/designs/issue-628-close-out.md`（approved）

## File Map

| File | Change |
| --- | --- |
| `crates/ruviform-audio-test/Cargo.toml` | `serde`/`serde_json` 追加（理由コメント）・description 更新 |
| `crates/ruviform-audio-test/src/clock.rs` | `VirtualClockConfig`/`JitterSpec` に serde derive のみ |
| `crates/ruviform-audio-test/src/metadata.rs` | 4 enum + 2 struct に serde derive、JSON API、stale doc 更新 |
| `crates/ruviform-audio-test/src/backend.rs` | `StreamFault::Cancellation` variant + module doc 更新 |
| `crates/ruviform-audio-test/src/backend/stream.rs` | `collect_faults` に cancel、両 pump の cancel 経路 |
| `crates/ruviform-audio-test/src/lib.rs` | module doc の scope 記述更新（JSON serialization を範囲内へ） |
| `crates/ruviform-audio-test/tests/metadata.rs` | round-trip・wire 語彙 pin・save/load test |
| `crates/ruviform-audio-test/tests/backend_faults.rs` | cancellation 正負テスト |
| `crates/ruviform-audio-test/README.md` | slice 記述・Non-responsibilities・CI 統合判断の更新 |
| `crates/README.md` | `ruviform-audio-test` 行の記述更新 |
| `docs/tasks/issue-628-{deterministic-test-runtime,scripted-backend,pipeline-wiring}.md` | status `done` + checkbox 更新 |
| `evidence/rf628/` | AC 棚卸し表 + close-out 検証記録 |
| `docs/{specs,designs,plans,tasks,test-specs}/issue-628-close-out.md` | 本 chain |
| `docs/impl/{ir,exp}/issue-628-close-out.*` | impl records |
| `Cargo.lock` | serde/serde_json 追加に伴う再生成 |
| `docs/*/README.md`（管理 index） | doc-maintenance/impl-doc の再生成 |

## Implementation Sequence

- [ ] T1: `Cargo.toml` に serde/serde_json 追加 + description 更新
- [ ] T2: `clock.rs` serde derive、`metadata.rs` serde derive + JSON API
      + stale doc 更新
- [ ] T3: `backend.rs` `StreamFault::Cancellation` + `stream.rs` pump
      経路 + `lib.rs` doc 更新
- [ ] T4: `tests/metadata.rs` JSON tests、`tests/backend_faults.rs`
      cancellation tests
- [ ] T5: crate README・`crates/README.md`・`docs/tasks/` 3 件 status
      修正
- [ ] T6: `evidence/rf628/` AC 棚卸し表 + 検証記録
- [ ] T7: 検証実行（fmt-check/check/lint/test/test-rt/prek）
- [ ] T8: impl/ir + impl/exp 記録、review-protocol レビュー
- [ ] T9: docs index 再生成・origin/main merge・commit/push/PR

## Dependencies and Risks

- serde derive が `#[non_exhaustive]` enum で問題なく動くか — fieldless
  enum + kebab-case rename は定型的に動作する。`cargo check` で即検証。
- `serde_json` version は lockfile の 1.0.154（workspace 既存 pin 範囲
  `"1"`）に解決される — 新規 version を引かないことを `Cargo.lock`
  diff で確認。
- cancellation の `PumpReport` 表現 — 既存 field のみで観測
  （`errors_fired`/`invalidated`/`frames=0`）。field 追加は consumer
  互換のため行わない。

## Task Breakdown

`docs/tasks/issue-628-close-out.md` を参照。

## Verification

| Check | Command |
| --- | --- |
| fmt | `mise run fmt-check` |
| check | `mise run check ruviform-audio-test` |
| lint | `mise run lint ruviform-audio-test` |
| test | `mise run test ruviform-audio-test` |
| rt | `mise run test-rt`（既存 `rt_` 非回帰 — 新規 `rt_` test なし） |
| hooks | `mise exec -- prek run --all-files` |
| index | doc-maintenance `index plan/apply` + impl-doc `sync_impl_index` |
| boundary | `git diff --name-only` に製品 crate src が無いこと |

## Open Questions / Gaps

- なし（cancellation 写像・wire format は design で確定済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch 内で完結）。
- Independent work streams: none（T1–T9 は逐次）。
- User approval required before dispatch: no — `review-protocol` Skill
  のレビューで承認を代替（blocker が残っていない状態を承認とする）。

## Review Handoff

- cancellation 写像の解釈（data callback skip + `Backend("cancelled")`
  + stream 終了）が Issue の意図と整合するか。
- serde wire format が Case Schema v1 語彙と一致するか。
- 棚卸し表の verdict が正直か（NotRun/Deferred の過大/過小主張なし）。
