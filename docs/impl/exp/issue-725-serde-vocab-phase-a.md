---
id: "EXP-725-serde-vocab-phase-a"
type: "experiment"
status: "draft"
title: "Issue #725 serde 語彙 Phase A — 実装実験記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-725-serde-vocab-phase-a.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-725-serde-vocab-phase-a.md"
      - type: "file"
        path: "docs/impl/exp/issue-725-serde-vocab-phase-a-exp.jsonl"
  implements:
    - "docs/tasks/issue-725-serde-vocab-phase-a.md"
  implemented-by:
    - "docs/impl/ir/issue-725-serde-vocab-phase-a.md"
  depends-on:
    - "docs/tasks/issue-725-serde-vocab-phase-a.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  events: "docs/impl/exp/issue-725-serde-vocab-phase-a-exp.jsonl"
---

# Issue #725 serde 語彙 Phase A — 実装実験記録

## Summary

Issue #725 Phase A slice の実装過程で行った判断と検証の記録。
companion JSONL: `docs/impl/exp/issue-725-serde-vocab-phase-a-exp.jsonl`。

## 実験イベント

### seq 1-3: 調査・観測（start / observation）

- consumer 影響評価: audio-lens・synvelis は `metadata` module・
  `EnvironmentKind`・`RunMetadata`・`environment_kind` を未消費
  （`oracle`/`pcm`/`backend`/`clock` のみ）。影響ゼロを
  `evidence/rf725/consumer-impact-assessment.md` に記録。
- schema 正本 `ci/consumer-verify/evidence.mjs` `ENVIRONMENT_KINDS`
  は `test-plugin`。emit 経路は `ruviform-plugin-dock-test` の
  `as_str()` → evidence JSONL で、literal assert が rf#631 占有の
  ため emit flip は本 slice 不可と確認。

### seq 4-5: 判断（decision）

- **Phase A = deserialize alias のみ**: `#[serde(alias =
  "test-plugin")]` は deserialize 側の追加受容名で serialize 出力を
  変えない。`environment_kind_serde_matches_as_str` が serde emit ↔
  `as_str()` 一致を強制するため、emit flip（serde `rename` +
  `as_str()`）は同一 slice で不可分 — 第2 slice に分離。
- **designs/ を採用（adr/ ではない）**: issue-721 前例に倣い
  設計判断は `docs/designs/` に記録。2 段階 serde 移行計画は
  MADR 級の恒久的アーキテクチャ決定ではないため ADR は作成しない
  （編集範囲に `docs/adr/` も許可されていたが不要と判断）。

### seq 6-7: 検証・総括（validation / summary）

- `mise run check ruviform-audio-test` → exit 0。
- `mise run test ruviform-audio-test` → exit 0。新規 pin
  `environment_kind_deserialize_accepts_both_vocab_spellings`・
  `run_metadata_from_json_accepts_test_plugin_spelling` pass。
  emit 綴り pin は不変で維持。

## 採用・却下

- adopted: 本 slice の全変更（alias・doc 更新・test pin・docs chain・
  evidence・issue-629 status 追記）。
- rejected: emit flip 一括実施（rf#631 占有で不可）・custom serde
  impl・`as_str()` 単独変更・`#[serde(other)]` catch-all
  （design の「却下した代替案」参照）。
