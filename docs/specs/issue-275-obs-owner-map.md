---
id: "SPEC-034WZzsCw7LraB3uPLZvMl"
type: "spec"
status: "approved"
title: "Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付けと Deferred 範囲の Owner/Trigger 明記"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "docs/specs/issue-275-observability-integration-design.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/designs/issue-275-obs-owner-map-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/plans/issue-275-obs-owner-map-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-275-obs-owner-map-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/impl/exp/issue-275-obs-owner-map.md"
      - type: "file"
        path: "evidence/rf275/obs-owner-map-audit.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-275-observability-integration-design.md"
    - "docs/designs/issue-275-observability-integration-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  refines:
    - "docs/specs/issue-275-observability-integration-design.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-275-observability-integration-design.md"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-275-obs-owner-map.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/279"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/290"
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/292"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  defers: []
  deferred-by: []
---

# Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付けと Deferred 範囲の Owner/Trigger 明記

## What Should Be Built

Issue #275 の残件のうち、headless に可能な監査・文書スライスとして
次を作成・更新する。

1. **設計正本への Owner/API/Feature/CI 対応表の追記**:
   `docs/designs/issue-275-observability-integration-design.md`
   に、OBS-01〜11 それぞれについて (a) 実装 Owner Issue
   （URL 付き）、(b) 公開 API/モジュール、(c) Cargo feature、
   (d) 既存 CI（Native Build/Link・Headless・RT）の該当
   lane/case を Case 別に対応付けた表を追記する。
2. **AL/SV 独立 Headless 受入の引き渡し明記**: AudioLens
   #127・Synvelis #28 の Logging/Privacy/RT/精度の独立
   Headless 受入を RF #281/#294 への引き渡しとして対応表に
   明記する。Closed の履歴 Issue（AL #138/SV #81）は再 Open
   しない。
3. **Deferred 範囲の Owner/Trigger 明記**: 実外部 Provider・
   実 OS GUI・実 Device・長時間 Profiler を現行設計 Close の
   必須実走とせず、利用要件発生時の Deferred 範囲として
   再開 Owner と Activation Trigger を明記する。`NotRun` を
   `Pass`/`Unsupported` に置き換えない。
4. **文書チェーンと監査証拠**: 本 spec を起点に design /
   test-spec / plan / task / impl(ir/exp) の文書チェーンを
   `issue-275-obs-owner-map` 名で作成し、監査の実測根拠を
   `evidence/rf275/obs-owner-map-audit.md` に記録する。

## Why Now

- Issue #275 本文の現行受入方針（2026-11-13 更新）は
  「Windows L1＋使用機能の L2/L3、macOS/Linux L1 のみ。
  実外部 Provider・実 OS GUI・実 Device・長時間 Profiler は
  利用要件発生まで Deferred」であり、残件は OBS-01〜11 と
  実装 Owner/公開 API/Feature Matrix/既存 CI Case の対応
  付け・AL/SV 別 Oracle の引き渡し・Deferred 範囲の
  Owner/Trigger 明記である。
- per-module 実装 Issue（#276〜#280・#289〜#293）は CLOSED
  で、`crates/ruviform-observability` のモジュール・feature・
  公開 API は main 上に存在するが、Issue #275 設計正本には
  OBS 単位の Owner/API/feature/CI Case の対応表がなく、
  検証状態（実走済み/NotRun/Deferred）の所在が分散して
  いる。
- Issue 状態（#281/#294/#219 OPEN、AL #127/SV #28 OPEN）は
  変動するため、`gh issue view` 実測に基づく snapshot と
  して本 slice で固定し、gate 段階の Owner が参照できる
  ようにする。

## Users and Value

- **Ruviform maintainer / Observability Owner**: OBS-01〜11
  の Owner Issue・公開 API・feature・既存 CI lane の対応を
  1 表で確認でき、後続の検証・受入判定の根拠を固定できる。
- **#281（OBS-06 gate）・#219/#294（cross-repo 機構）Owner**:
  どの Case がどの lane で実行され得るか、何が `NotRun` で
  Deferred かを対応表から辿れる。
- **AudioLens #127・Synvelis #28 Owner**: 自 repo の
  Logging/Privacy/RT/精度の独立 Headless 受入が
  #281/#294 経由で要求されること、および履歴 Issue を再
  Open しない境界を確認できる。

## Scope

### In Scope

- `docs/designs/issue-275-observability-integration-design.md`
  への対応表・Deferred 範囲節の追記（既存内容の改変は
  「Workerへの共通契約」節を除き最小限。歴史的 snapshot は
  改変しない）。
- `issue-275-obs-owner-map` 名の文書チェーン
  （spec/design/test-spec/plan/task/impl ir・exp）と
  `evidence/rf275/obs-owner-map-audit.md`。
- Issue 状態の `gh issue view` 照合と crate/feature/CI lane
  の実測棚卸し（`cargo metadata`/`git ls-files`/`cargo tree`
  は補助証拠、canonical = mise）。
- 触った dir の生成 index の `doc-maintenance` による再生成。

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/workflows/` の変更（コード・CI 変更なし）。
- per-module の詳細 API・内部実装・test 規則の再定義
  （#276〜#293 の設計・test-spec が正本。本 slice は
  対応表と索引のみを所有する）。
- AudioLens/Synvelis repo の変更・実測（cross-repo gate・
  consumer-verify・各 Consumer Issue が Owner）。
- 実外部 Provider/実 OS GUI/実 Device/長時間 Profiler の
  実施（Deferred として Owner/Trigger のみ明記）。
- 新規 CI Workflow・証拠 YAML/JSON スキーマの作成。
- GitHub の正式依存（blockedBy）の登録・削除。
- Closed 履歴 Issue（AL #138/SV #81）の再 Open。

## Requirements

- OBS-01〜11 の全項目について、実装 Owner Issue（URL 付き）・
  公開 API/モジュール・Cargo feature・既存 CI lane/case
  の対応を、実在する正本（Issue・`Cargo.toml`・`src/`・
  workflow・mise タスク）に基づいて記述する。
- `default = []` と独立 optional feature（`logging`/
  `tracing-export`/`allocation-probe`/`integration`）の
  実態を反映し、仮想的な feature 名・module 名を記載しない。
- AL #127・SV #28 の Logging/Privacy/RT/精度の独立
  Headless 受入を #281/#294 への引き渡しとして明記し、
  一方の Consumer の Pass を他方へ転用しない。
- 実外部 Provider・実 OS GUI・実 Device・長時間 Profiler
  の Deferred 範囲に、再開 Owner と Activation Trigger
  （利用要件の具体化）を明記する。未使用 device/plugin
  format/特殊 filesystem 等を global blocker にしない。
- Windows は OS-L1 必須・GUI-L2/EXT-L3 は使用機能のみ
  Required、macOS/Linux は OS-L1 必須・GUI/EXT は利用要件
  発生まで Deferred という現行方針（POLICIES）を反映する。
- `NotRun`/`Fail` の必須 case が残る能力は未受入のまま
  記録し、job 定義や skipped job から Pass を推論しない。
- 「設計承認・実装完了・OS-L1 証拠・Windows 使用機能
  L2/L3 証拠・Consumer 受入・cross-repo candidate-SHA
  gate 完了」の各段階を分離して記述する。
- 「Workerへの共通契約」節を含む既存設計書の保護対象を
  変更しない。

## Acceptance Criteria

- [ ] 設計正本 `docs/designs/issue-275-observability-
      integration-design.md` に OBS-01〜11 の Owner Issue
      （URL 付き）・公開 API/型・feature・既存 CI
      （Native Build/Link/Headless/RT）Case 別の対応表が
      追記されている。
- [ ] AL #127・SV #28 の Logging/Privacy/RT/精度の独立
      Headless 受入が #281/#294 への引き渡しとして対応表
      に明記され、Closed 履歴 Issue（AL #138/SV #81）を
      再 Open する記述がない。
- [ ] 実外部 Provider・実 OS GUI・実 Device・長時間
      Profiler が Deferred 範囲として Owner/Trigger 付きで
      明記され、`NotRun` が `Pass` に置き換えられていない。
- [ ] 変更が `docs/` と `evidence/rf275/` のみであり、
      `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
      `.github/` に差分がない。「Workerへの共通契約」節に
      差分がない。
- [ ] Issue 状態は `gh issue view` 実測の snapshot として
      記録され、Closed/Deferred/NotRun を実装 Blocked by
      として誤記していない。

## Deferred Design Concerns

- Feature compile matrix（`--no-default-features`・各
  feature 単独・full consumer 組合せの compile 検証）|
  既存 lane は `msrv-verify` の `--all-features` のみで
  per-feature 分離は未実施 | RF #281 G0 の gate 証拠項目
  として継続 | feature 分離の compile 破壊が検出されない
  まま残るリスク。
- 実 OS GUI（実 Window 上の Observability 購読/表示）|
  GUI-L2 は使用機能のみ Required であり現行は Deferred |
  RF #293（API 境界）+ 利用機能の Owner Issue、Trigger は
  GUI 診断表示の実提供決定 | GUI 経路の実機不具合が
  headless 検証をすり抜けるリスク。
- 実 Device・長時間 Profiler・実外部 Provider | 利用要件
  未発生のため Deferred | RF #281 + 新規 Issue（必要時）、
  Trigger は各利用要件の具体化 | 実負荷・実機条件下の
  RT/精度問題が gate 前に顕在化するリスク。

## Sources and Relations

- 正本 Issue: [#275](https://github.com/scarecrow173/
  ruviform/issues/275)（残件: OBS-01〜11 Owner/API/
  Feature/CI 対応・AL/SV Oracle 引き渡し・Deferred
  Owner/Trigger）。
- 親設計: `docs/specs/issue-275-observability-integration-
  design.md` / `docs/designs/issue-275-observability-
  integration-design.md`（本 slice の更新対象正本）。
- Gate/Consumer Issue: RF #281（OBS-06）・#294・#219、
  AL #127（継続追従）・AL #138（履歴 CLOSED）、SV #28
  （継続追従）・SV #81（履歴 CLOSED）。
- 実測証拠: `evidence/rf275/obs-owner-map-audit.md`
  （Issue 状態・crate/feature・CI lane の棚卸し）。
