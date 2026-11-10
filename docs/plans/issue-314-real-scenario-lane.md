---
id: "PLAN-034VgbX0QiuS6CYlPVeh59"
type: "plan"
status: "approved"
title: "Issue #314 slice 4 follow-up — real-scenario lane + matrix 実装 plan"
created: "2026-11-10"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "docs/specs/issue-314-real-scenario-lane.md"
    - "docs/designs/issue-314-real-scenario-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-314-real-scenario-lane.md"
  implements:
    - "docs/specs/issue-314-real-scenario-lane.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-314-real-scenario-lane.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 4 follow-up — real-scenario lane + matrix 実装 plan

## Goal

`real-scenario` の canonical mise lane を追加し、real-os story
matrix の残 cell（scale 2.0・第 2 theme）を実 desktop session で
実走して証拠と `stories.json` を実績どおりに更新する。

## Scope

単一実装ストリーム — lane 追加・test matrix 化・証拠・文書は
相互依存が強く分割しない。

## Upstream Documents

- `docs/specs/issue-314-real-scenario-lane.md`

## Design Inputs

- `docs/designs/issue-314-real-scenario-lane.md`

## File Map

- `mise.toml` - `[tasks.test-ui-real-scenario]` 追加（env で
  evidence 出力先を固定、既存 task 不変）
- `crates/ruviform-ui-gpui/tests/real_window.rs` - 2 theme ×
  2 scale matrix 化、`UiError::Host`/`BackendUnavailable` →
  `environment_blocked` evidence + panic、PNG dir の
  workspace-root 相対解決、cell 別 PNG/meta file 名
- `crates/ruviform-ui-test/fixtures/driver-lab/stories.json` -
  `drivers.real-os` の note/mechanism（`mise-task`/
  `test-ui-real-scenario`）を実績どおり更新
- `evidence/issue-314-real-scenario/` - lane 実行の driver-lab /
  golden JSONL + PNG/meta
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  issue-314-real-scenario-lane.md` - 文書チェーン + IR

## Implementation Sequence

- [x] T1 文書チェーン（spec → design → plan → task/test-spec）作成
- [ ] T2 `tests/real_window.rs` matrix 化 + env-blocked 区分 +
  PNG dir 解決
- [ ] T3 `mise.toml` に `test-ui-real-scenario` 追加
- [ ] T4 `mise run test-ui-real-scenario` を対話 desktop session
  で実行 → `evidence/issue-314-real-scenario/` 生成
- [ ] T5 `stories.json` `drivers.real-os` 更新（matrix 実績）
- [ ] T6 既存検証 lane 実行（fmt/check/lint/test/test-ui-* 系）
- [ ] T7 merge origin/main・文書 index 再生成・IR・review・PR

## Dependencies and Risks

- 対話 desktop session が必要 — 非対話環境では lane は
  environment_blocked として記録される（fail ではなく区分）。
- `cargo test` は exit-code 区分を持てないため environment_blocked
  は JSONL `observed` + panic tag で表現（design 決定 2）。
- 実 window 8 session（2 story × 4 cell）は数十秒オーダー。

## Task Breakdown

- T2–T3 は独立だが同一コミット単位が自然。T4 は T2/T3 完了後。
  T5 は T4 の実測後。T6–T7 は全変更確定後。

## Verification

- [ ] `mise run fmt-check` / `check` / `lint` / `test`
- [ ] `mise run test-ui-golden` / `test-ui-scenario-contract` /
  `test-ui-parity` / `test-ui-component-view` / `test-ui-gpui` /
  `check-ui-driver-boundary`
- [ ] `mise run test-ui-real-scenario`（対話 desktop session）
- [ ] `evidence/issue-314-real-scenario/` に JSONL + PNG/meta が
  出力され verdict が pass/baseline_missing→pass を示す

## Open Questions / Gaps

- なし（matrix 軸・env 区分・lane 命名は design で確定済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no（Orca worker として
  coordinator 指示のもと実行）

## Review Handoff

- `environment_blocked` 区分の妥当性（`UiError::Host`/
  `BackendUnavailable` のみを環境扱いにする境界）と、
  `stories.json` mechanism を `mise-task` に変える影響範囲
  （`verify_story_mechanisms` が `[tasks.test-ui-real-scenario]`
  の実在を要求する点）を reviewer に確認してもらう。
