---
id: "TSPEC-f2Kd9wLq4Zx7mNv3bRc5Ty"
type: "test-spec"
status: "approved"
title: "Issue #631 AC4 evidence-fields 完成のテスト仕様"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-631-evidence-fields.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-evidence-fields.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 AC4 evidence-fields 完成のテスト仕様

## Purpose

AC4 evidence field 拡充が「実観測に基づく正直な記録」である
ことを保証する test spec。window identity・capture digest・
artifact digest・semantic tree 全量が、捏造・欠落・偽装なく
JSONL evidence に刻まれることを検証する。verdict 判定ロジック
自体は変更しない — 本 spec が守るのは evidence field の
存在・形状・fail-closed 性。

## Feature

`crates/ruviform-plugin-dock-test` の real lane
（`real-dock` feature、interactive Windows desktop 前提）と
headless lane（`lab-gui`/default feature）の scenario runner、
および AC4 evidence field（`environment.window_identities`・
`capture_sha256`・`artifact.{path,sha256}`・
`observed.semantic_tree` 全量）。

## Rules

- Rule: window identity digest は session 開始時に window
  A–D 各々について 1 回だけ計算され、canonical 文字列
  （window id・title・geometry）から `Sha256::digest` で
  導出される。同じ入力は同じ digest、異なる window/geometry
  は異なる digest。
- Rule: geometry 読取に失敗した window は `sha256: null` で
  正直に記録し、identity を捏造しない。`blocked_reports`/
  `blocked_probe` 経路（session 不在）には
  `window_identities` を刻まない。
- Rule: `real-window-dpi` の `window_geometry_sha256`（case
  内再観測 digest）は本 slice で変更しない — session 開始時
  identity とは別 field。
- Rule: `capture_sha256` は実際に capture した RGBA 画素
  バイトの SHA-256 — `real-capture`（`RgbaImage::pixels()`）
  と dcomp 系（`ospix::PixelCapture.rgba`）の両方で、capture
  成功時のみ刻む。
- Rule: PNG artifact を書いた場合の記録は `{path, sha256}`
  object — sha256 は書き込んだ PNG bytes の digest と一致
  する。書込失敗は `artifact_error` で verdict を変えない。
- Rule: `--pixel-artifacts` は opt-in — canonical
  `test-ui-plugin-dock-real` は artifacts dir を渡さず PNG を
  生成しない。`real-capture` も同じ opt-in で PNG を書ける。
- Rule: `observed.semantic_tree` は mount 済み `ViewSpec` の
  全 component key（`validate_spec` の flat 列挙と同じ —
  Panel children 再帰、Tabs は alias key + canonical child
  key の両方）を持ち、各値は `component_snapshot_json`
  object または観測失敗時 `null`。
- Rule: semantic tree walk は verdict ロジックと分離した
  best-effort evidence — 観測失敗で case の verdict を
  変えない。
- Rule: catalog 件数・row・`case`/`scenario`/`required` は
  不変（headless 9 + real 22）。既存の整合 test が green
  を維持する。
- Rule: headless/blocked では fail-closed 形状を維持 — 新
  field も session 不在の行に刻まず、`environment_blocked`/
  `NotRun` を偽装しない。`EnvironmentKind::RealWindow` は
  実走 session 内の report のみが名乗る。
- Rule: evidence 拡充は additive — 既存 field の型・意味を
  変えない（`artifact` の path-only 文字列 → `{path,
  sha256}` object は AC4 要求の shape 変更として
  verification.md に明記）。

## Examples

### Example: window identity が全 real case の environment に出る

Given interactive Windows desktop で real lane を実走
When `plugin-dock-real-scenario.jsonl` が生成される
Then 全 real case row の `environment.window_identities` に
window A–D 各々の `{window_id, title, geometry, sha256}` が
記録され、geometry 読取失敗 window は `sha256: null`。

### Example: capture row に digest が出る

Given `real-capture`/`real-dcomp-pixel`/`real-dcomp-foreground`/
`real-dcomp-present` の capture step が成功
When step row が JSONL に書かれる
Then `capture_sha256`・`backend`・`width`・`height` が記録
され、PNG artifact があれば `artifact: {path, sha256}`
object になる。

### Example: semantic tree が全 key を持つ

Given `sv_fader_dock_fixture`（Panel>Tabs>2×Fader）を mount
した `real-command-receipt`/headless `command-receipt`
When `observed.semantic_tree` が記録される
Then `panel`/`mixer-tabs`/`Ch 1`/`fader-ch1`/`Ch 2`/
`fader-ch2` の全 6 key が存在し、各値は snapshot object
または `null`。

### Example: blocked-probe は新 field を刻まない

Given headless 環境で `--blocked-probe` を実行
When `plugin-dock-real-headless.jsonl` が生成される
Then 全行 `NotRun`+`environment_blocked` で
`window_identities`/`capture_sha256`/`artifact` は出ず、
exit 0 を維持する。

## Verification

- `mise run test ruviform-plugin-dock-test --features real-dock`
  — window identity helper の determinism・semantic tree
  helper の全 key 列挙・`save_pixel_artifact` の `{path,
  sha256}` を unit test で検証。
- `mise run test ruviform-plugin-dock-test` — headless
  `command-receipt` の `semantic_tree` 全量を integration
  test で検証。
- `mise run test-ui-plugin-dock-real-headless` — blocked-
  probe が新 field を刻まず fail-closed を維持。
- `mise run test-ui-plugin-dock-real`（interactive あれば）
  — real JSONL の新 field を目視確認。
- `mise run test-ui-plugin-dock-console` — catalog 整合
  （件数不変）を確認。
