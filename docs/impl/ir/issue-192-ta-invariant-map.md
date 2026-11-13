---
id: "IMPL-034WYgZmp0Qv3NLjgtu1jn"
type: "impl"
status: "completed"
title: "Issue #192 TA Invariant Map — 監査対応表・非循環性静的確認の実装記録"
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
    - "docs/tasks/issue-192-ta-invariant-map-task.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-192-ta-invariant-map.md"
    - "docs/specs/issue-192-ta-invariant-map.md"
    - "docs/designs/issue-192-ta-invariant-map-design.md"
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
    - "docs/tasks/issue-192-ta-invariant-map-task.md"
    - "docs/test-specs/issue-192-ta-invariant-map.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-192-ta-invariant-map-task.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-ta-invariant-map.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #192 TA Invariant Map — 監査対応表・非循環性静的確認の実装記録

## Summary

Issue #192 の headless 監査スライスを文書として実装した。umbrella 設計 §7
の TA-001〜TA-012 各 invariant を現行 API・crate・設計判断・実装 Issue・
既存 OS-L1 テスト・未達 Headless Gap へ 1 対 1 対応付けた正本
`docs/discovery/issue-192-ta-invariant-map.md` を作成し、`cargo metadata`
（default/--all-features）の静的読みで依存非循環性を記録し、索引側
tracking 設計へ参照を追記した。実装コード・manifest・CI の変更はない。

## Context

`docs/tasks/issue-192-ta-invariant-map-task.md`（task）→
`docs/plans/issue-192-ta-invariant-map-plan.md`（plan）→
`docs/specs/issue-192-ta-invariant-map.md`（spec）→
`docs/designs/issue-192-ta-invariant-map-design.md`（design）→
`docs/discovery/issue-192-ta-invariant-map.md`（正本）のチェーンに従う。
umbrella 設計
`docs/designs/synvelis-driven-target-architecture-and-migration.md` §7 と
tracking canonical
`docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
が権威。M0 監査 `docs/discovery/issue-194-m0-baseline-audit.md` が先行
実測。

## Implementation

### Changes

- `docs/discovery/issue-192-ta-invariant-map.md`（新規）: TA-001〜012 の
  対応表（5 軸）、Area 別現行 API 対応、依存非循環性監査節（方法・観測値・
  境界検査・違反なし）、Headless Gap→Open Gate 接続一覧、Key Issues、
  Alternatives、Tentative Conclusions、Open Questions、Promotion
  Candidates、Sources、Evidence。
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  （追記）: 対応表への参照 1 行。
- 文書チェーン: spec/design/plan/task/test-spec/impl-ir/impl-exp を新規
  作成し relations を接続。
- docs index（`docs/{discovery,specs,designs,plans,tasks,test-specs,impl/*}/README.md`）
  は doc-maintenance で再生成。

### Decisions During Implementation

- 対応表の「現行 API」は crate::代表シンボルに限定し、実装詳細は各正本・
  test-spec への参照に委ねた（design の記録粒度方針どおり）。
- 依存監査は `kind: null` を normal として扱い、dev/build/target を
  dep_kinds で区別。dev-dep の上位参照（plugin-host→audio 等）は umbrella
  §3 の規定どおり循環と扱わなかった。
- Issue 状態は 2026-11-14 の `gh issue view` snapshot として記録し、
  umbrella 正本の 2026-11-09 snapshot との差分（#284/#285/#286/#288/#305
  CLOSED 化）を明記した。
- `cargo metadata` の集計は補助 Python スクリプトで実施し、canonical 検証
  入口は mise タスクと明記した（文書監査のため build/test は未実行）。

## Related Experiments

### Adopted

- なし（本タスクは文書監査であり、実験的選択を伴わない）。

### Rejected

- なし。

## Validation

- `mise exec -- cargo metadata --format-version 1 --locked`（default）
  exit 0・42 member・98 normal+build edge・26 dev edge・循環 0。
- `mise exec -- cargo metadata --format-version 1 --locked --all-features`
  exit 0・103 normal+build edge・129 全 kind edge・循環 0。
- `gh issue view` で関連 Issue の OPEN/CLOSED snapshot を取得。
- grep 棚卸しで公開 API・テスト関数名を列挙。
- `mise run fmt-check` exit 0（`cargo fmt --all -- --check`）。
- `mise exec -- prek run --files <変更ファイル>` exit 0（trim trailing
  whitespace・end of files・mixed line ending・utf-8 BOM・private key
  全て Passed）。
- doc-maintenance `index apply` で `docs/{discovery,specs,designs,plans,
  tasks,test-specs,impl/ir}` の README を再生成、`docs/impl/exp/README.md`
  は reports 節へ手動追記。doc-status lint で本チェーンの blocking
  finding なし（`design.implements→spec` の reciprocal warning 1 件は
  既存 convention と同型の非 blocking）。
- review-protocol: cooperative/adversarial の 2 pass で要件網羅性と
  事実確認（テスト名・依存 kind・Issue 状態・lane 存在）を実施し
  blocker なし（ACCEPT）。

## Risks

- Issue 状態 snapshot は 2026-11-14 時点であり、以後の変動は追跡しない
  （記録日・取得方法を明記済み）。
- TA-002 の型境界は grep 棚卸し＋review 責務の補完であり、機械的な網羅
  保証ではない（Deferred Design Concern に記録）。
- DAG 非循環の CI 常設 gate は存在せず、監視 gap として記録（違反では
  ない）。

## Follow-ups

- DAG 非循環の CI 常設 gate 化の要否 → #294/#219 系と Owner 判断。
- TA gate の正式 Close 判定 → #193/Owner。
- Consumer 独立受入 → SV #28/#106/#109・AL #127・SV #84。
- 実機/実 Plugin/実 Consumer 受入 → #251/#253/#243/#262/#313/#388/#389/
  #465/#219/#281/#294/#306。
