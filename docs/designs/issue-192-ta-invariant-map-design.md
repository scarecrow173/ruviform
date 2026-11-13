---
id: "DESIGN-034WYfShwch3ZurjTQp3Sp"
type: "design"
status: "approved"
title: "Issue #192 TA Invariant Map — 監査設計"
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
  implemented-by: []
  depends-on:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-192-ta-invariant-map.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-192-ta-invariant-map.md"
    - "docs/discovery/issue-192-ta-invariant-map.md"
  derived-by:
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
    - "docs/test-specs/issue-192-ta-invariant-map.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-ta-invariant-map.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #192 TA Invariant Map — 監査設計

## Context

Issue #192 の umbrella 設計 §7 は TA-001〜TA-012 を正式受入 gate として
定義し、Issue #193 が実行計画を管理する。各 TA の実装は M1〜M7・GUI・
IO・OBS・CrossConsumer の capability Issue 群（#181〜#198 等）で merge
済みだが、「invariant ↔ 現行 API/実装 Issue/既存 L1 テスト」の 1 対 1
対応表と feature 別依存非循環性の静的確認は文書化されていなかった。
M0 監査（#194）は 28 crate 時点の先行実測であり、現在の 42 member・
feature 構成と差分がある。本設計は監査の構成・証拠基準・記録様式を定め、
実装は行わない（文書監査タスク）。

## Scope

### In Scope

- 対応表の列構成と記録粒度（API/crate・設計判断・実装 Issue・OS-L1
  テスト・Gap→Open Gate の 5 軸）。
- 依存監査の方法（`cargo metadata` の 2 feature 集合・dep_kinds 集計・
  DFS 循環検出・禁止方向の境界検査表）。
- 証拠の等級付け（実測値 vs 棚卸し vs 既存テスト記録 vs 未実行）。
- 索引側 tracking 設計への参照追記の位置と文言方針。
- 文書チェーンの構成と relations。

### Out of Scope

- TA gate の正式 Close 判定・受入基準の再定義（umbrella §7 が権威）。
- 実機・実 Plugin・実 Consumer の検証設計（各 Open Gate Issue）。
- CI gate の新設・変更（Deferred Design Concern として記録のみ）。
- 実装コード・manifest・lockfile の変更。

## Components and Boundaries

- **正本対応表**: `docs/discovery/issue-192-ta-invariant-map.md`。
  umbrella §7 の TA-001〜012 を行とし、各行に 5 軸の対応を記録。
  「現行 API」は crate::代表シンボルに限定し、実装詳細の列挙はしない。
- **依存非循環性監査節**: 同文書内の独立節。`cargo metadata`
  （default/--all-features）→ workspace member 間 path edge 抽出 →
  kind/target 別集計 → DFS 循環検出 → umbrella §3 禁止方向の境界検査表。
- **索引側参照**: `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  の TA 対応表近傍に「現行 API/Issue/テスト対応表は discovery 正本を参照」
  の 1 行を追記。既存の設計判断・capability map・Owner 記述は変更しない。
- **文書チェーン**: spec（要求）→ 本 design（監査構成）→ plan → task →
  test-spec（検証規約）→ impl-ir/impl-exp（実施記録）。discovery が
  spec の `derives-from`、test-spec が spec/design を `verifies`/
  `derives-from` で接続。

## Data and Control Flow

監査の情報フロー:

1. **Invariant 抽出**: umbrella §7 の TA-001〜012「Required evidence」を
   正本として採用（tracking canonical の snapshot 表は更新対象）。
2. **API/テスト棚卸し**: `crates/*/src` の公開 export・pub 宣言・
   `tests/` 関数名を grep で列挙し、test-spec の Rule/Example ID
   （PPQN-01〜12・TT-008/025〜030・OF-01〜06・EX-001〜006・G2-xx）と
   対応付け。
3. **依存監査**: `mise exec -- cargo metadata --format-version 1 --locked`
   （default/--all-features）→ `resolve.nodes[].deps[].dep_kinds` を
   workspace member 間 path edge に限定（`kind: null` = normal）→
   集計・循環検出・境界検査。
4. **Issue 状態**: `gh issue view` で OPEN/CLOSED snapshot を取得し、
   Gap を Open Gate Issue へ接続。
5. **記録**: discovery 正本に統合し、索引側へ参照を追記。

## Non-Functional Constraints

- **再現性**: 監査対象 SHA・実行コマンド・exit code・環境を Evidence 節に
  記録し、同一 SHA で再算出可能にする。
- **証拠等級の分離**: cargo metadata 実測・grep 棚卸し・既存テスト記録・
  未実行検証を混同しない（POLICIES の Required/Deferred/NotApplicable・
  Pass/Fail/NotRun/Unsupported 語彙に整合）。
- **範囲制約**: `crates/`・manifest・CI・他 docs ファイルは変更しない。
  docs index は doc-maintenance の再生成のみ。
- **正本の非上書き**: umbrella 設計・#193 を supersede しない。対応表は
  受入管理の索引であり、invariant の再定義ではない。

## Risks and Trade-offs

| Risk | Mitigation |
| --- | --- |
| grep 棚卸しが公開型の網羅を保証しない（TA-002） | 棚卸し結果を「検出なし＋review 責務継続」と正直に記録し、機械化を Deferred Design Concern に残す |
| cargo metadata の解決 edge が manifest 宣言と乖離する | default/all-features の 2 集合で報告し、feature ゲート optional edge を個別列挙。M0 の宣言↔解決区別を踏襲 |
| Issue 状態 snapshot が陳腐化する | 記録日（2026-11-14）と取得方法を明記し、状態は「時点の観測」として扱う |
| 対応表が実装詳細に踏み込み保守負荷が増す | API は crate::代表シンボルに限定し、詳細は各正本・test-spec への参照に委ねる |
| dev-dep の上位参照を循環と誤認する | umbrella §3 の「既存テスト用 dev-dependencies の上位参照を同じ意味の循環と扱わない」を適用し、kind 別に集計 |

## Deferred Design Concerns

- DAG 非循環の CI 常設 gate 化 | #294/#219 系 CI 設計と Owner 判断に委ねる
  | consumer-verify tuple gate 確定時 | 依存構造の悪化検出が遅れる。
- TA-002 型混入の機械的検査 | grep＋review の現行運用を継続するかは設計変更
  を伴う | 型境界の機械的強制要件が出た時 | review 見逃しの可能性。
- 実機/実 Consumer 受入 | 各 Open Gate Issue（#251/#253/#243/#262/#313/
  #388/#389/#465/#219/#281/#294/#306）の責務 | 各 Issue の完了時 | TA gate
  の正式受入が遅延。

## Verification Notes

- [x] discovery 正本に TA-001〜012 全行と依存監査節が存在する。
- [x] 依存監査が `mise exec -- cargo metadata`（default/--all-features）の
      実測値・方法・違反有無を含む。
- [x] 索引側 tracking 設計に対応表への参照が追記されている。
- [x] 文書チェーンの relations が解決し doc-status 監査が通る。
- [x] `mise run fmt-check` と `prek run --files` が通る。
- [x] review-protocol レビューで blocker が残っていない。
