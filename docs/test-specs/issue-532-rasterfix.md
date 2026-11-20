---
id: "TSPEC-034ZaaRasterfix00001"
type: "test-spec"
status: "approved"
title: "Issue #532 raster-fix oracle — test spec"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-rasterfix.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-rasterfix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-rasterfix.md"
    - "docs/designs/issue-532-rasterfix.md"
    - "docs/discovery/issue-532-rasterfix.md"
    - "evidence/rf532/rasterfix-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-rasterfix.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-rasterfix.md"
  verified-by: []
metadata:
  issue: 532
---

# Issue #532 raster-fix — test spec

## 検証対象

`docs/specs/issue-532-rasterfix.md` の受入条件 — private
container class、bounded paint drain、
`real-dcomp-rasterfix` の fix oracle（screen green
`>= 0.5`）、verified restore、catalog/count 同期、
fail-closed headless 動作。

## 検証 lane と実施結果

| lane | 実行 | 結果 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 実施 | pass |
| `mise run check-ui-gpui` / `check ruviform-plugin-dock-test --features real-dock --all-targets` | 実施 | pass |
| `mise run lint-ui-gpui` / `lint ruviform-plugin-dock-test --features real-dock --all-targets` | 実施 | pass（`-D warnings`） |
| `cargo test -p ruviform-ui-gpui --features "gpui-test uitest"` | 実施 | 全 pass（`container_uses_private_class_without_parentdc` を含む 78+ tests） |
| `cargo test -p ruviform-plugin-dock-test --features real-dock` | 実施 | 17 lib + 14 console + 5 plugin_dock 全 pass（`real_catalog_matches_entries` で 28 行同期を検証） |
| `mise run check-ui-driver-boundary` | 実施 | pass（dep-graph/signature/allowlist） |
| `mise run test-ui-native-slot` | 実施 | 13 tests pass |
| `mise run test-ui-plugin-dock` | 実施 | lane exit 0（`evidence/rf631/plugin-dock-scenario.jsonl`） |
| `mise run test-ui-plugin-dock-console` | 実施 | `--list` + `--run` exit 0 |
| `mise run test-ui-plugin-dock-real-headless` | 実施 | 28 行全て `NotRun` + `environment_blocked` の fail-closed mapping、exit 0 |
| `mise run test-ui-plugin-dock-real` | 実施 | **`real-dcomp-rasterfix` = Pass（`fix_arm = "reparent"`）**、`real-dcomp-pixel`/`bandescape`/`childinv` は既知未解決の honest Fail、exit 1 は設計どおり |
| DComp-off aux run（`GPUI_DISABLE_DIRECT_COMPOSITION=1`） | 実施 | `real-dcomp-rasterfix` = Pass（`fix_arm = "reparent"`）、`evidence/rf532/plugin-dock-real-scenario-dcomp-off.jsonl` |
| `--pixel-artifacts` aux run | 実施 | Pass + `evidence/rf532/rasterfix-pixels/` PNG 証跡、`evidence/rf532/plugin-dock-real-scenario-artifacts.jsonl` |

## verdict の誠実性チェック

- `real-dcomp-rasterfix` の Pass は (a) real
  `NativePluginDock` lease 経由の実 editor attach、
  (b) attach→close→detach→converge の lifecycle 完遂、
  (c) probe evaluable、(d) reparent arm の全 op（baseline
  exstyle/frame/`SetParent`/rect seat/`HWND_TOPMOST`）
  実行、(e) restore の parent/exstyle/rect 検証 +
  `HWND_NOTOPMOST` band 復元を含む全 op applied、
  (f) `reparent` arm の screen green 1.000 の全てが
  揃って初めて出る。mutation 成功だけでは Pass しない。
- arm が全て走り restore が検証済みでも green が出な
  ければ `Fail`（仮説棄却 — pump arm は実際に 0.000 を
  記録した）。
- baseline unreadable → arm skip 時は `executed=false` で
  「全 arm が走った」扱いにしない。
- `win32_error` は refusal を記録したまま `applied`
  （read-back 一致判定）を変えない — 証跡は失われない。
- headless では session を開始せず blocked-probe の
  fail-closed mapping を自己検証（`environment_blocked`
  と `environment_kind=real-window` 非claim を assert）。

## catalog 同期

`real_entries`（28 要素）↔ `REAL_CATALOG`（28 行）↔
`tests/console.rs` `list_include_real_appends_real_lane_rows`
↔ `tests/plugin_dock.rs` ↔ `lib.rs` module doc ↔ `README.md`
case 表 — `real_catalog_matches_entries` unit test で
entry↔metadata の一致を常時検証。
