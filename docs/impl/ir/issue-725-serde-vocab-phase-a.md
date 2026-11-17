---
id: "IMPL-034Z04w3xKvYC8XSP2bAGu"
type: "impl"
status: "completed"
title: "Issue #725 serde 語彙 Phase A — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
    - "evidence/rf725/consumer-impact-assessment.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-725-serde-vocab-phase-a.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-725-serde-vocab-phase-a.md"
  implemented-by:
    - "docs/impl/exp/issue-725-serde-vocab-phase-a.md"
  depends-on:
    - "docs/tasks/issue-725-serde-vocab-phase-a.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
  experiments:
    adopted:
      - "docs/impl/exp/issue-725-serde-vocab-phase-a-exp.jsonl"
    rejected: []
---

# Issue #725 serde 語彙 Phase A — 実装記録

## Summary

`EnvironmentKind::RealAbiTestPlugin` の serde 語彙 gap に対し、
Phase A（deserialize 側 alias 受容）を実装した。consumer 影響評価
（audio-lens・synvelis は `metadata` module 未消費で影響ゼロ）と
serde 移行方針（Phase A = alias、第2 slice = emit flip）を記録し、
issue-629 docs 5 ファイルへ status を追記した。emit flip は
rf#631 占有の `crates/ruviform-plugin-dock-test/` 更新を必須とする
ため第2 slice に分離 — 本 slice では「gap 解消済み」とは記さない。

## Context

`docs/tasks/issue-725-serde-vocab-phase-a.md`（T1-T9）に従う。
上流: `docs/specs/issue-725-serde-vocab-phase-a.md`・
`docs/designs/issue-725-serde-vocab-phase-a.md`（移行期間中の挙動
表・第2 slice 作業範囲・却下案）・`docs/discovery/`・
`docs/test-specs/`・`evidence/rf725/consumer-impact-assessment.md`。

## Implementation

### Changes

- `crates/ruviform-audio-test/src/metadata.rs`:
  - `RealAbiTestPlugin` variant に `#[serde(alias = "test-plugin")]`
    を付与 — deserialize が `real-abi-test-plugin`（canonical 綴り）
    と `test-plugin`（schema 語彙）の両方を受容。
  - enum doc の「Known vocabulary gap」記述を Phase A 状態に更新
    （deserialize 両綴り受容・emit flip は第2 slice）。
  - `as_str()`・serde emit 綴りは不変（`rename_all = "kebab-case"`
    由来の `real-abi-test-plugin` を維持）。
- `crates/ruviform-audio-test/tests/metadata.rs`:
  - `environment_kind_deserialize_accepts_both_vocab_spellings`
    追加 — 両綴り deserialize 受容・emit/`as_str` 不変・未知綴り
    拒否を pin。
  - `run_metadata_from_json_accepts_test_plugin_spelling` 追加 —
    `RunMetadata` JSON 文書の `test-plugin` 綴り受容・再 emit で
    旧綴りに正規化・旧綴り文書の読み戻しを pin。
  - `environment_kind_wire_spellings_match_case_schema_v1` の
    コメントを Phase A 記録に更新（emit 綴り pin 自体は維持）。
- `evidence/rf725/consumer-impact-assessment.md`（新規）:
  consumer 影響評価の記録（AC1）。
- `docs/designs|specs|test-specs/issue-629-*.md` 5 ファイル:
  gap 記述へ rf#725 Phase A status を追記（append-only、計 13 箇所）。
- docs chain: `docs/discovery|specs|designs|plans|tasks|test-specs|
  impl/ir|impl/exp/issue-725-serde-vocab-phase-a.md` + companion
  `impl/exp/*-exp.jsonl`（新規）。
- `crates/ruviform-audio-test/Cargo.toml`: 変更なし（serde derive
  既存で `#[serde(alias)]` に十分 — 追加依存不要）。

### Decisions During Implementation

- `as_str()` は変更しない — serde emit（`rename_all` 由来）と
  `as_str()` は `environment_kind_serde_matches_as_str` で一致が
  強制されており、emit flip は serde `rename` + `as_str()` +
  plugin-dock-test 更新を同一 slice で行う必要があるため第2 slice
  へ分離（design 記載どおり）。
- `#[serde(alias)]` は deserialize 側のみ作用し serialize 出力を
  変えない（serde の仕様）— emit 不変を新規テストで pin。
- issue-629 docs は「追記のみ」制約に従い、既存記述の書き換え・
  削除を行わず status 追記のみに限定。
- ADR は作成しない — 2 段階 serde 移行は `docs/designs/` の設計
  判断として記録（issue-721 前例）。exp jsonl seq 5 に記録。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-725-serde-vocab-phase-a-exp.jsonl` — 本 slice
  の判断（Phase A 範囲・designs/ 採用）と検証結果を記録した
  experiment log。alias 実装・emit 不変 pin・docs 記録の根拠。

### Rejected

- なし（experiment log は 1 本のみ。却下した代替案は design の
  「却下した代替案」節に記録）。

## Validation

実施済み（exit code 記録）:

- `mise run fmt-check` — （実行結果は PR 本文に記録）
- `mise run check ruviform-audio-test` — exit 0
- `mise run lint ruviform-audio-test` — （実行結果は PR 本文に記録）
- `mise run test ruviform-audio-test` — exit 0（新規 2 テスト含む
  全 pass）
- `mise run test-ci-consumer-verify` — （実行結果は PR 本文に記録）
- `mise exec -- prek run --all-files` — （実行結果は PR 本文に記録）

NotRun: `*-full`・`*-msrv`・3OS 横断・GUI/EXT-L3 実機 lane —
daily CI 委譲（POLICIES.md §5・Spec の検証範囲どおり）。

## Risks

- Phase A だけでは emit 側 `real-abi-test-plugin` が schema 語彙
  `test-plugin` と一致しない — 既知 gap として docs/PR に明記。
  consumer-verify の lane record 検証では引き続き未知語彙として
  拒否される。emit flip（第2 slice）で解消。
- 第2 slice は rf#631 lease 解除後または Coordinator 調整待ち —
  長期化リスクは design の Risks 表に記録済み。

## Follow-ups

- 第2 slice（emit flip）: `#[serde(rename = "test-plugin",
  alias = "real-abi-test-plugin")]` + `as_str()` flip +
  `crates/ruviform-plugin-dock-test/` の literal assert・コメント
  更新。再開条件: rf#631 lease 解除後または Coordinator 調整。
  Issue #725 の完全 close には本 slice が必須。
- docs index（`docs/**/README.md`）は Coordinator が統合時に
  再生成 — 本 slice では未更新。
