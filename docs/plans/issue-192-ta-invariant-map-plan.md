---
id: "PLAN-034WYgMWpq7xkREpxpFnGt"
type: "plan"
status: "approved"
title: "Issue #192 TA Invariant Map — 監査・対応表作成計画"
created: "2026-11-13"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-192-ta-invariant-map.md"
      - type: "file"
        path: "docs/specs/issue-192-ta-invariant-map.md"
      - type: "file"
        path: "docs/designs/issue-192-ta-invariant-map-design.md"
      - type: "file"
        path: "docs/plans/issue-192-ta-invariant-map-plan.md"
      - type: "file"
        path: "docs/tasks/issue-192-ta-invariant-map-task.md"
      - type: "file"
        path: "docs/test-specs/issue-192-ta-invariant-map.md"
      - type: "file"
        path: "docs/impl/ir/issue-192-ta-invariant-map.md"
      - type: "file"
        path: "docs/impl/exp/issue-192-ta-invariant-map.md"
    modified:
      - type: "file"
        path: "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/specs/issue-192-ta-invariant-map.md"
  implemented-by:
    - "docs/tasks/issue-192-ta-invariant-map-task.md"
  depends-on:
    - "docs/designs/issue-192-ta-invariant-map-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-192-ta-invariant-map.md"
    - "docs/test-specs/issue-192-ta-invariant-map.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-192-ta-invariant-map.md"
    - "docs/designs/issue-192-ta-invariant-map-design.md"
  derived-by:
    - "docs/tasks/issue-192-ta-invariant-map-task.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-ta-invariant-map.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #192 TA Invariant Map — 監査・対応表作成計画 Implementation Plan

## Goal

umbrella 設計 §7 の TA-001〜TA-012 invariant を現行 API/Issue/L1 テスト/
Headless Gap へ 1 対 1 対応付けた正本監査文書を作成し、`cargo metadata`
の静的読みで依存非循環性を記録し、索引側へ参照を追記する。

## Scope

単一の文書監査ストリーム。実装コードの変更を伴わないため分割しない。
変更対象は `docs/discovery/issue-192-ta-invariant-map.md`（正本）、
`docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
（索引側の参照追記）、および本計画が生成する文書チェーン
（spec/design/plan/task/test-spec/impl-ir/impl-exp）と doc-maintenance が
再生成する各 `docs/<type>/README.md` index に限定する。

## Upstream Documents

- `docs/specs/issue-192-ta-invariant-map.md`（要求）
- `docs/discovery/issue-192-ta-invariant-map.md`（監査の発見元・正本）
- `docs/designs/synvelis-driven-target-architecture-and-migration.md`（umbrella §7 正本）
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`（索引側・追記対象）
- `docs/discovery/issue-194-m0-baseline-audit.md`（M0 先行実測）

## Design Inputs

- `docs/designs/issue-192-ta-invariant-map-design.md`（監査構成・証拠基準・
  記録様式）

## File Map

- `docs/discovery/issue-192-ta-invariant-map.md` - TA-001〜012 対応表正本 +
  依存非循環性監査節 + Area 別 API 対応 + Headless Gap 接続 + Evidence。
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md` -
  対応表への参照 1 行の追記のみ（既存記述は変更しない）。
- `docs/specs|designs|plans|tasks|test-specs|impl/ir|impl/exp/issue-192-ta-invariant-map*.md` -
  文書チェーン。
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/README.md` -
  doc-maintenance による index 再生成（手編集しない）。

## Implementation Sequence

- [x] umbrella §7 の TA-001〜012 invariant を抽出し、tracking canonical の
      snapshot を確認。
- [x] `crates/*` の公開 API・`tests/` のテスト関数名を棚卸しし、各 TA の
      現行 API・既存 OS-L1 テストを特定。
- [x] `mise exec -- cargo metadata`（default/--all-features）で依存 edge・
      循環・target 条件を集計し、umbrella §3 の禁止方向を検査。
- [x] `gh issue view` で関連 Issue の OPEN/CLOSED snapshot を取得。
- [x] `docs/discovery/issue-192-ta-invariant-map.md` に対応表・監査節・
      Gap 接続・Evidence を記録。
- [x] 索引側 tracking 設計へ対応表参照を追記。
- [x] 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）を
      作成し relations を接続。
- [x] doc-maintenance で index を再生成し doc-status 監査を通す。
- [x] `mise run fmt-check` と `prek run --files` を通す。
- [x] review-protocol レビューで blocker を解消する。
- [ ] Conventional Commits でコミットし、worker branch を push して PR 作成。

## Dependencies and Risks

- 依存: umbrella 設計・#193・M0 監査・関連 ADR/Issue 状態。全て読み取り
  済みでブロッカーなし。
- リスク: Issue 状態 snapshot の陳腐化（記録日明記で緩和）、grep 棚卸しの
  網羅性限界（review 責務継続を明記で緩和）、dev-dep 上位参照の循環誤認
  （kind 別集計で緩和）。詳細は design の Risks 節。

## Task Breakdown

- 単一 task `docs/tasks/issue-192-ta-invariant-map-task.md` に集約
  （文書監査の一貫性を保つため分割しない）。

## Verification

- [x] `mise exec -- cargo metadata --format-version 1 --locked`（default）
      exit 0・42 member・98 normal+build edge・26 dev edge・循環 0。
- [x] `mise exec -- cargo metadata --format-version 1 --locked --all-features`
      exit 0・103 normal+build edge・129 全 kind edge・循環 0。
- [x] `mise run fmt-check` exit 0。
- [x] `mise exec -- prek run --files <変更ファイル>` exit 0。
- [x] doc-maintenance の index 再生成と doc-status 監査が整合。
- [x] review-protocol レビューで blocker なし。

## Open Questions / Gaps

- DAG 非循環の CI 常設 gate 化の要否（#294/#219 系と Owner 判断）。
- TA gate の正式 Close 判定と #192/#193 の Close（証拠棚卸し時）。
- Consumer 独立受入の完了時期（SV #28/#106/#109・AL #127・SV #84）。
- 実機/実 Plugin/実 Consumer の受入は各 Open Gate Issue の責務であり
  本計画の完了条件に含めない。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（文書監査のため委譲不要）。
- Independent work streams: none。
- User approval required before dispatch: yes（review-protocol で代替）。
- If approved: そのまま task を実行。
- If declined or unavailable: 同じ依存関係と検証行列で inline 実行。

## Review Handoff

- 監査の網羅性（TA-001〜012 全行・5 軸記録）と依存監査の再現性
  （SHA・コマンド・edge 数・循環 0・違反なし）を重点確認。
- 証拠等級の分離（実測 vs 棚卸し vs 既存テスト記録 vs 未実行）が
  POLICIES の検証レベル規範と整合するか確認。
