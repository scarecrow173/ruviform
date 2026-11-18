---
id: "ADR-w8Mn4vXc2Qp6rTs9yHb5Js"
type: "adr"
status: "accepted"
title: "issue-631-evidence-fields opt-in artifacts・semantic tree helper 配置"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-631-evidence-fields.md"
    - "../designs/issue-631-evidence-fields.md"
    - "../adr/issue-631-headless-command-receipt.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-evidence-fields.md"
    - "docs/designs/issue-631-evidence-fields.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# issue-631-evidence-fields opt-in artifacts・semantic tree helper 配置

## Status

accepted（2026-11-18）

## Context

AC4 evidence field 完成にあたり 2 つの設計判断が必要だった:

1. **pixel artifact の canonical 配線** — `test-ui-plugin-
   dock-real` に `--pixel-artifacts` を常時配線して PNG を
   毎回 `evidence/rf631/` に生成・コミットする案と、opt-in
   artifacts + canonical digest（`capture_sha256`）の案。
2. **semantic tree walk の配置** — real lane（`real_dock.rs`）
   と headless lane（`scenario.rs`）の両方で mount 済み
   `ViewSpec` の全 component key を observe する必要がある。
   `component_snapshot_json` の lane 間複製は
   `docs/adr/issue-631-headless-command-receipt.md` で既知・
   記録済み。

## Decision

1. **opt-in artifacts + canonical digest を採用する** —
   `capture_sha256` があれば画素を保存せず run 間 diff が
   可能であり、opt-in で書いた PNG には `{path, sha256}`
   が付いて検証可能性も保たれる。`*.png` は `.gitattributes`
   で `binary`（LFS 非対象）のため、canonical lane が実走
   ごとに PNG をコミットすると Git 履歴に純バイナリ churn
   が積み上がる。canonical `test-ui-plugin-dock-real` の
   引数は変更しない（`mise.toml` description の stale 文言
   修正のみ）。
2. **semantic tree helper は `scenario.rs` に
   `SemanticGuiTest` generic として実装する** — real lane
   （`real_dock.rs`）と headless lane（`scenario.rs`）の両方
   から呼べる共有 helper を `scenario.rs` に置く。walk は
   `ViewHost::validate_spec`（`view.rs:1185`）と同じ
   （Panel children 再帰 + Tabs alias/canonical 両 key）。
   `component_snapshot_json` の lane 間複製は本 slice でも
   維持する（helper 自体は `scenario.rs` 側に置き、real
   lane は `#[cfg(feature = "real-dock")]` で参照 —
   `component_snapshot_json` のみ lane ごとに既存のまま）。
   新規 driver API・production crate 変更は追加しない。

## Consequences

- `capture_sha256`/`{path, sha256}`/`window_identities`/
  全量 `semantic_tree` が全 real case 行・headless
  `command-receipt` に additive に記録される。
- canonical lane は PNG を生成しない — Git 履歴にバイナリ
  churn が積み上がらない。opt-in `--pixel-artifacts` で
  PNG を書いた場合のみ `{path, sha256}` が残る。
- semantic tree walk は verdict ロジックと分離した
  best-effort evidence — 観測失敗 key は `null` で verdict
  を変えない。
- `component_snapshot_json` の lane 間複製は継続 — 将来の
  共有化は別 slice の判断事項。
- `artifact` の記録 shape が path-only 文字列から
  `{path, sha256}` object に変わる — AC4 要求の shape 変更
  として verification.md/PR に明記。
