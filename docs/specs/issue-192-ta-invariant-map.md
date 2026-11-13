---
id: "SPEC-034WYfwcdnbMdpGzQQzRKN"
type: "spec"
status: "approved"
title: "Issue #192 TA Invariant ↔ 現行API/Issue/L1テスト対応表 + 依存非循環性監査"
created: "2026-11-13"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
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
  implements: []
  implemented-by:
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-192-ta-invariant-map.md"
    - "docs/test-specs/issue-192-ta-invariant-map.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-192-ta-invariant-map.md"
  derived-by:
    - "docs/designs/issue-192-ta-invariant-map-design.md"
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-ta-invariant-map.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
---

# Issue #192 TA Invariant ↔ 現行API/Issue/L1テスト対応表 + 依存非循環性監査

## What Should Be Built

Issue #192（Synvelis-driven target architecture migration）の umbrella 設計
`docs/designs/synvelis-driven-target-architecture-and-migration.md` §7 が
定義する TA-001〜TA-012 の各 invariant を、現行 Ruviform の公開 API・
crate 構成・設計判断（ADR/正本）・実装 Issue・既存 OS-L1 テスト・未達
Headless Gap へ 1 対 1 で対応付けた正本の監査文書
`docs/discovery/issue-192-ta-invariant-map.md`。あわせて `cargo metadata`
の静的読みで、Ruviform generic 層が AudioLens 精度/CLI・Synvelis
DAW/RT/Project 側へ依存しない境界と、Cargo Feature 別の依存非循環性を
確認した記録を含む。索引側
`docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
には対応表への参照を追記する。

## Why Now

umbrella 設計 §7 の TA-001〜012 は正式受入を Issue #193 の Gate として
管理するが、「各 invariant と現行 API/実装 Issue/既存 L1 テストの 1 対
1 対応表」と「feature 別依存の非循環性の静的確認」は未整備だった。M0
監査（#194・28 crate 時点）は先行実測だが、その後 workspace は 42 member
へ拡大し feature 構成も変化しているため、受入管理に使える現行値への更新が
必要。headless に可能な監査スライスとして先に固定し、実機・実 Consumer
受入は各 Open Gate Issue の責務として切り分ける。

## Users and Value

- Coordinator / Issue #192・#193 の Owner: TA gate の受入判定時に
  「どの invariant がどの API・Issue・テストで裏付けられているか」を
  追跡できる。
- Ruviform の後続実装者: 各 TA の Gap がどの Open Gate Issue に接続される
  かを把握し、重複実装や境界違反を避けられる。
- Consumer（Synvelis / AudioLens）: Ruviform generic 層の依存境界が
  静的に確認された記録を共有できる。

## Scope

### In Scope

- `docs/discovery/issue-192-ta-invariant-map.md` の作成（正本対応表 +
  依存非循環性監査記録）。
- TA-001〜TA-012 各項目の「現行 API / crate・設計判断・実装 Issue・
  既存 OS-L1 テスト・Headless Gap → Open Gate」の対応付け。
- `cargo metadata`（default / all-features）の静的読みによる依存 edge・
  循環・target 条件・禁止方向の監査記録（検査方法・観測・違反有無）。
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  への対応表参照の追記（索引側の更新のみ）。
- 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）の作成。

### Out of Scope

- 実 Device / OS Native Dock / 両 Consumer の実測・受入判定（#251/#253/
  #243/#262/#313/#388/#389/#465/#219/#281/#294/#306 等の責務）。
- TA gate の正式 Close 判定、Issue #192/#193 自体の Close。
- CI 実走・Run URL の証拠収集。
- `crates/` 配下を含む実装コード・Cargo manifest・lockfile・CI 設定の
  変更。
- `cargo metadata` 以外の build/test 実行（文書監査のため）。

## Requirements

- TA-001〜TA-012 の 12 項目全てに対応行を持つこと。
- 各行に現行 API（crate::シンボル）・設計判断・実装 Issue・既存 OS-L1
  テスト名・Gap と接続先 Open Issue を記録すること。
- 依存監査は manifest 宣言と resolver 活性を区別し、default と
  all-features の両方で edge 数・循環・禁止方向を記録すること。
- 観測された事実と推測を区別し、未実行の検証を実行済みと記載しないこと。
- 「merge 済み（headless/CI/PoC）」と「Consumer 受入・実機検証」を別状態
  として記録すること（POLICIES §3）。

## Acceptance Criteria

- [ ] `docs/discovery/issue-192-ta-invariant-map.md` に TA-001〜TA-012 の
      対応表と依存非循環性監査結果が記録されている。
- [ ] 依存監査が `mise exec -- cargo metadata`（default/--all-features）の
      実測値と方法を含み、違反の有無が明記されている。
- [ ] `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
      に対応表への参照が追記されている。
- [ ] 文書チェーンの front matter relations が解決し、doc-status の
      監査・index 再生成が通る。
- [ ] `mise run fmt-check` と `prek run --files <変更ファイル>` が通る。
- [ ] review-protocol によるレビューで blocker が残っていない。

## Deferred Design Concerns

- DAG 非循環の CI 常設 gate 化 | repo 全体の `cargo metadata` 循環検査を
  CI lane にするかは #294/#219 系の CI 設計と Owner 判断に委ねる |
  #294 consumer-verify tuple gate の設計確定時 | gate 不在のまま依存構造が
  悪化しても検出が遅れる。
- TA-002 型混入の CI 静的検査 | grep 棚卸し＋review 責務の現行運用を
  継続するか機械化するかは設計変更を伴う | 型境界の機械的強制を求める
  要件が出た時 | 公開型への DAW 概念混入を review で見逃す可能性。
- Consumer 独立受入の完了 | SV #28/#106/#109・AL #127・SV #84 等の
  consumer repo 側 Issue | 各 consumer の実装・検証完了時 | TA-012 の
  正式受入が遅延する。

## Sources and Relations

- umbrella 設計（権威）:
  `docs/designs/synvelis-driven-target-architecture-and-migration.md` §7。
- tracking canonical:
  `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`。
- 発見元監査: `docs/discovery/issue-192-ta-invariant-map.md`（本 spec の
  `derives-from`）。
- M0 baseline: `docs/discovery/issue-194-m0-baseline-audit.md`。
- 関連 ADR: `synvelis-driven-generic-execution-boundaries`・
  `realtime-contract-and-verification-policy`・`plugin-rt-ownership-split`・
  `audio-engine-snapshot-swap`・
  `configurable-ppqn-resolution-independent-musical-time`・
  `media-io-backend-abstraction`（詳細は discovery 文書の Sources 参照）。
