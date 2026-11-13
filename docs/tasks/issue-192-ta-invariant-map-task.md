---
id: "TASK-034WYgYyoC0JWYkg6gXlBK"
type: "task"
status: "done"
title: "Issue #192 TA Invariant Map — 監査対応表の作成と非循環性の静的確認"
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
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
  implemented-by:
    - "docs/impl/ir/issue-192-ta-invariant-map.md"
  depends-on:
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-192-ta-invariant-map.md"
    - "docs/specs/issue-192-ta-invariant-map.md"
    - "docs/designs/issue-192-ta-invariant-map-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
  derived-by:
    - "docs/impl/ir/issue-192-ta-invariant-map.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-ta-invariant-map.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #192 TA Invariant Map — 監査対応表の作成と非循環性の静的確認

## Work

Issue #192 の headless 監査スライス。umbrella 設計 §7 の TA-001〜TA-012
各 invariant を現行 API・crate・設計判断・実装 Issue・既存 OS-L1
テスト・未達 Headless Gap へ 1 対 1 対応付けた正本
`docs/discovery/issue-192-ta-invariant-map.md` を作成し、
`mise exec -- cargo metadata`（default/--all-features）の静的読みで
依存非循環性を記録し、索引側 tracking 設計へ参照を追記する。
実機・実 Plugin・実 Consumer・CI 実走・TA gate Close は対象外。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-192-ta-invariant-map-plan.md`
- `docs/specs/issue-192-ta-invariant-map.md`
- `docs/designs/issue-192-ta-invariant-map-design.md`
- `docs/discovery/issue-192-ta-invariant-map.md`
- `docs/designs/synvelis-driven-target-architecture-and-migration.md`（umbrella §7）
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`（索引側・追記対象）

## Dependencies

- umbrella 設計・M0 監査・関連 ADR/Issue 状態の読み取り（完了済み）。
- `cargo metadata` の実行（mise 経由・exit 0 確認済み）。

## Affected Files

- `docs/discovery/issue-192-ta-invariant-map.md`（新規・正本）
- `docs/specs/issue-192-ta-invariant-map.md`（新規）
- `docs/designs/issue-192-ta-invariant-map-design.md`（新規）
- `docs/plans/issue-192-ta-invariant-map-plan.md`（新規）
- `docs/tasks/issue-192-ta-invariant-map-task.md`（本書）
- `docs/test-specs/issue-192-ta-invariant-map.md`（新規）
- `docs/impl/ir/issue-192-ta-invariant-map.md`（新規）
- `docs/impl/exp/issue-192-ta-invariant-map.md`（新規）
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`（参照追記のみ）
- `docs/{discovery,specs,designs,plans,tasks,test-specs}/README.md`（doc-maintenance 再生成）

## Done When

- [x] discovery 正本に TA-001〜TA-012 全 12 行の対応表と依存非循環性
      監査節が記録されている。
- [x] 依存監査が `mise exec -- cargo metadata`（default/--all-features）の
      実測値・方法・違反有無を含む。
- [x] 索引側 tracking 設計に対応表への参照が追記されている。
- [x] 文書チェーンの relations が解決し doc-status 監査が通る。
- [x] `mise run fmt-check` と `prek run --files` が通る。
- [x] review-protocol レビューで blocker が残っていない。

## Verification

- [x] `mise exec -- cargo metadata --format-version 1 --locked`（default）
      exit 0・42 member・98 normal+build edge・26 dev edge・循環 0。
- [x] `mise exec -- cargo metadata --format-version 1 --locked --all-features`
      exit 0・103 normal+build edge・129 全 kind edge・循環 0。
- [x] `mise run fmt-check` exit 0。
- [x] `mise exec -- prek run --files <変更ファイル>` exit 0。
- [x] doc-maintenance index 再生成・doc-status 監査の整合。
- [x] review-protocol レビューで blocker なし。
