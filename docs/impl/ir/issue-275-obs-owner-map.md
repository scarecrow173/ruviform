---
id: "IMPL-034WZPoBmkr0oYITBjQA67"
type: "impl"
status: "in-progress"
title: "Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付け"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "docs/specs/issue-275-obs-owner-map.md"
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
  implements:
    - "docs/tasks/issue-275-obs-owner-map-task.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-275-obs-owner-map-task.md"
    - "docs/plans/issue-275-obs-owner-map-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-275-obs-design-reconciliation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-275-obs-owner-map-task.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "evidence/rf275/obs-owner-map-audit.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-275-obs-owner-map.md"
    rejected: []
---

# Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付け

## Summary

Issue #275 の残件のうち headless に可能な監査・文書
スライスを実施した。統合設計正本
`docs/designs/issue-275-observability-integration-design.md`
に OBS-01〜11 の Owner Issue・公開 API・Cargo feature・
既存 CI Case の対応表と、Deferred 範囲の Owner/Trigger 表を
追記し、`issue-275-obs-owner-map` 文書チェーンと監査証拠
`evidence/rf275/obs-owner-map-audit.md` を作成した。
コード・CI・manifest の変更はない。

## Context

- 対象 Issue: [#275](https://github.com/scarecrow173/
  ruviform/issues/275)。現行受入（2026-11-13）は
  「Windows L1＋使用機能の L2/L3、macOS/Linux L1 のみ。
  実外部 Provider・実 OS GUI・実 Device・長時間 Profiler
  は利用要件発生まで Deferred」。
- 文書チェーン: `docs/specs/issue-275-obs-owner-map.md` →
  `docs/designs/issue-275-obs-owner-map-design.md` →
  `docs/test-specs/issue-275-obs-owner-map.md` →
  `docs/plans/issue-275-obs-owner-map-plan.md` →
  `docs/tasks/issue-275-obs-owner-map-task.md`。
- 実測根拠: `gh issue view`（Issue 状態 snapshot
  2026-11-14）、`cargo tree --workspace -e features`
  （feature unify の実測、補助証拠）、`Cargo.toml`/
  `src/`/`mise.toml`/`.github/workflows/` 走査。

## Implementation

### Changes

- `docs/designs/issue-275-observability-integration-design.md`:
  `## 証拠項目対応` 節の直後に `## OBS-01〜11 Owner/API/
  Feature/CI Case 対応表`（11 行の対応表 + Consumer 独立
  Oracle 引き渡し小節）と `## Deferred 範囲の
  Owner/Trigger`（4 項目の表 + feature compile matrix の
  区別注記）を新設。front matter `updated` を
  2026-11-14 に更新。「Workerへの共通契約」節・歴史的
  snapshot 節は不変。
- `evidence/rf275/obs-owner-map-audit.md`（新規）: Issue
  状態・crate/feature/公開 API・CI lane・Deferred 棚卸し・
  判定メモ。
- `issue-275-obs-owner-map` 文書チェーン 5 件 + 本 IR +
  exp 記録。
- 生成 index: `docs/{specs,designs,plans,tasks,test-specs}/
  README.md` を `doc-maintenance index apply` で再生成、
  `docs/impl/ir/README.md` を `sync_impl_index.js --kind ir
  --apply` で再生成。`docs/impl/exp/README.md` の管理 index
  は `*.jsonl` のみを対象とし、`.md` の exp 記録は
  issue-306 先例どおり index 非掲載のため変更なし。

### Decisions During Implementation

- **対応表は 1 表 8 列**: Owner Issue・Module/crate・
  Feature・公開 API（代表）・正本 doc・既存 CI Case・
  状態区分。詳細 API は per-issue 正本参照とし drift を
  避ける（既存設計書の索引方針を継承）。
- **状態語彙の分離**: 要件状態（Required/Deferred/
  NotApplicable）と実行結果（Pass/Fail/NotRun/
  Unsupported）を分け、lane 定義の存在から Pass を推論
  しない旨を明記。`ci.yml` が dispatch-only である点も
  明記した。
- **feature 付き module の CI 到達範囲**: `logging`/
  `tracing-export`/`allocation-probe` を有効化する既存
  lane は `msrv-verify`/`test-observability-msrv`
  （`--all-features`・Linux・MSRV 1.89）のみ。Win/macOS
  native の feature 付き build/link/test は `NotRun` と
  記録した。`integration` は `ruviform-audio` dev-dep に
  よる feature unify で `test-full` 系に到達することを
  `cargo tree` で実測確認した。
- **OBS-04 の所在**: `ruviform-plugin-host` 側の実装で
  あり observability crate の feature を持たないことを
  表で区別した。
- **Deferred の Owner/Trigger**: 実外部 Provider・実 OS
  GUI・実 Device・長時間 Profiler の 4 項目に再開 Owner
  と Activation Trigger を記載。feature compile matrix は
  Deferred ではなく #281 G0 の継続 gate 項目（Required・
  NotRun）として区別した。
- **履歴 Issue**: AL #138・SV #81 は CLOSED の導入履歴
  Issue であり再 Open しない。継続受入は AL #127・SV #28
  → #281/#294 経由の独立 Oracle として記載。

## Related Experiments

### Adopted

- [exp: Issue #275 OBS owner map 監査メモ](../exp/
  issue-275-obs-owner-map.md) — Issue 状態・crate/
  feature・CI lane の棚卸し方針と判定メモ。

### Rejected

- なし。

## Validation

- `gh issue view` による Issue 状態照合（2026-11-14）:
  #275/#281/#294/#219・AL #127/#138・SV #28/#81 の状態を
  実測。#276〜#280・#289〜#293 は CLOSED、#281/#294/#219・
  AL #127・SV #28 は OPEN。
- `cargo tree --workspace -e features -i
  ruviform-observability`（補助証拠）: `integration`
  feature は `ruviform-audio` dev-dep 経由でのみ unify
  されることを確認。
- `mise run fmt-check`・`mise exec -- prek run --files
  <変更ファイル>`・`doc-maintenance` index 再生成・
  `audit_docs`/`audit_impl_record`/`audit_experiment_log`
  ・review-protocol — 結果は PR 本文に記録。

## Risks

- Issue 状態・CI lane 構成は変動する。snapshot 日付
  （2026-11-14）を明記済み。将来の状態変化では gate 段階
  で再照合が必要。
- 対応表は「実行し得る lane」の対応付けであり、個別 SHA
  の実測 Pass を意味しない。この誤読リスクは状態語彙節と
  証拠 file の判定メモで緩和した。

## Follow-ups

- #281（OBS-06 gate）: feature compile matrix・実機負荷・
  Consumer Oracle の実施は gate 段階の継続作業。
- #294/#219: candidate tuple gate・cross-repo 検証機構の
  運用は各 Owner が継続。
- AL #127・SV #28: Consumer 側の独立 Headless 受入。
- Deferred 項目の再開は各 Activation Trigger の具体化待ち。
