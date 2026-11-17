---
id: "EXP-725-serde-vocab-phase-b"
type: "experiment"
status: "completed"
title: "Issue #725 serde 語彙 Phase B — 実装実験記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-725-serde-vocab-phase-b.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/impl/exp/issue-725-serde-vocab-phase-b-exp.jsonl"
  implements:
    - "docs/tasks/issue-725-serde-vocab-phase-b.md"
  implemented-by:
    - "docs/impl/ir/issue-725-serde-vocab-phase-b.md"
  depends-on:
    - "docs/tasks/issue-725-serde-vocab-phase-b.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
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
  events: "docs/impl/exp/issue-725-serde-vocab-phase-b-exp.jsonl"
---

# Issue #725 serde 語彙 Phase B — 実装実験記録

## Summary

Issue #725 Phase B slice（emit flip: serde canonical 綴りを
`test-plugin` に統一）の実装過程で行った判断と検証の記録。
companion JSONL: `docs/impl/exp/issue-725-serde-vocab-phase-b-exp.jsonl`。

## 実験イベント

### seq 1: start

Phase B slice の開始記録。

### seq 2: observation — emit flip の atomic 実装

- `crates/ruviform-audio-test/src/metadata.rs`:116 の
  `#[serde(alias = "test-plugin")]` を
  `#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]`
  へ切替、:145 `as_str()` を `"test-plugin"` に flip。
  enum doc・variant doc を Phase B 完了状態に更新。
- `tests/metadata.rs`: emit pin 3 テストを flip —
  `environment_kind_wire_spellings_match_case_schema_v1` に
  `RealAbiTestPlugin` → `test-plugin` pin 追加、
  `environment_kind_deserialize_accepts_both_vocab_spellings` と
  `run_metadata_from_json_accepts_test_plugin_spelling` の emit
  期待値を `"test-plugin"` に更新（deserialize 両綴り受容 assert
  は維持・旧綴り文書の re-emit 正規化 pin を追加）。
- `plugin_dock.rs` :168,:211 を `"test-plugin"` に更新、
  `Cargo.toml` :77-79 コメントを更新。
- `scenario.rs`:280 `json!(kind.as_str())` は `as_str()` flip に
  自動追従 — 編集不要を確認（lease 外・read-only 参照）。

### seq 3: observation — 実効カバレッジの確認

- `plugin_dock.rs` の fail-closed degrade 設計（:137-149
  early-return）に対し、`ruviform_plugin_test_vst3.dll` が
  `target/debug/deps/` に存在し `artifact::resolve_artifact`
  の candidate 2（exe の `deps/` 直下）で解決されることを確認。
  `fixture_run_reports_honest_verdicts` は degrade 経路ではなく
  full assert 経路を実行し、更新後の `"test-plugin"` assert
  （:168,:211）を実効的に検証した。

### seq 4: validation — 正規検証の結果

- `mise run check ruviform-audio-test` — exit 0
- `mise run lint ruviform-audio-test` — exit 0
- `mise run test ruviform-audio-test` — exit 0（metadata 13 tests
  全 pass — flip pin・両綴り受容・未知語彙拒否・round-trip）
- `mise run check ruviform-plugin-dock-test` — exit 0
- `mise run lint ruviform-plugin-dock-test` — exit 0
- `mise run test ruviform-plugin-dock-test` — exit 0
  （`fixture_run_reports_honest_verdicts` 含む 4/4 pass — full
  assert 経路で `test-plugin` 綴りを実効検証）
- `mise run test-ci-consumer-verify` — exit 0（84/84 pass —
  schema 側非回帰）
- `mise run fmt-check` — pass（初回 2 箇所の rustfmt line-join
  指摘を修正後に pass）
- `mise exec -- prek run --all-files` — exit 0
- NotRun: `*-full`/`*-msrv`/3OS 横断・opt-in lane
  （`test-ui-plugin-dock*`）実走 — daily CI 委譲・運用判断

### seq 5: summary

Phase B emit flip 完了。serde emit・`as_str()`・
plugin-dock-test assert が全て `test-plugin` に統一され、
deserialize は新旧両綴りを恒久的に受容。残作業: issue-629 docs
5 ファイルへの「解消済み」status 追記（append-only）・
`evidence/rf725/` 検証記録・review-protocol・commit/push/PR。

## 判断メモ

- serde `rename`+`alias` の向き: canonical = `test-plugin`（schema
  正本に収束）、`real-abi-test-plugin` は deserialize alias で
  恒久受容 — Phase A design :81-87 の右列の実現。
- `environment_kind_serde_matches_as_str` は変更せず不変条件
  として維持 — flip の atomic 性を検証する側。
- `plugin_dock.rs` の assert 更新は綴りのみで判定構造
  （fail-closed degrade）不変 — 最小の整合的変更。
- fmt 初回失敗は rustfmt の line-join 正規化のみ（意味変更なし）
  — 修正後に fmt-check pass・test 再実行で確認。
