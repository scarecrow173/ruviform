---
id: "TSPEC-034ZaaChildInventory01"
type: "test-spec"
status: "approved"
title: "Issue #532 child-inventory probe — test spec"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-child-inventory.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-child-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-child-inventory.md"
    - "docs/designs/issue-532-child-inventory.md"
    - "docs/discovery/issue-532-child-inventory.md"
    - "evidence/rf532/child-inventory-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-child-inventory.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-child-inventory.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 child-inventory probe — test spec

## Purpose

`real-dcomp-childinv` case と `oschild::inventory` が、(1)
catalog/entry 整合、(2) query-only read の honest 記録、(3)
capture-plane の分離記録、(4) env arm の proxy 検証、(5)
layered/reparent retry の全 op 記録、(6) 既存 verdict
意味論、(7) fail-closed headless 挙動、を満たすことを検証する。

## 検証項目

### 静的（headless で担保）

- `real_entries` が 27 要素で `real-dcomp-childinv` が
  `real-dcomp-bandescape` 直後、`REAL_CATALOG` row と
  id/label/summary が一致。
- `tests/console.rs` の case 一覧・総行数（34）と
  `tests/plugin_dock.rs` の blocked 行数（27）が一致。
- `cargo test`（default/`real-dock` feature）、`fmt-check`、
  `lint` が通過 — compile 時に `WindowInventory` 全 field と
  cfg stub の対称が担保される。
- `plugin-dock`/`plugin-dock-console`/`scenario-contract`/
  `driver-boundary`/`native-slot`/`gpui`/blocked-probe の
  headless lane が pass — blocked-probe JSONL は 27 case rows。

### 動的（interactive real lane のみ観測可能）

- `ci_inventory` rows: 全 docked child + container + root の
  `style`/`exstyle`/`class_name`/`class_style`/`thread_id`/
  `process_id`/`parent` が measured fact として存在し、
  decode bits（`cs_parentdc`/`noredirectionbitmap` 等）が
  raw 値と整合。
- `ci_capture` rows: `PrintWindow` 4 変種 + window DC +
  screen が dims/backend/green/threshold/sha256 付きで記録。
- `ci_dcomp_env` row: env presence/value・root noredir bit・
  `inferred_dcomp_path` の対応が記録される。
- `ci_layered` rows: child-level write の `applied`/`win32_error`/
  read-back（`latched_as_child`）、retry 条件、reparent 後
  retry（`latched_as_toplevel`）が全 op 記録される。
- `ci_restore` row: `parent_restored`/`exstyle_restored`/
  `rect_restored`/`ops_ok` が記録され、refusal は
  `applied: false` のまま。
- verdict: 必須 op 拒否・restore 不完全なら `Fail`、全完遂なら
  `Pass`、capability 不足なら `NotRun` — green は verdict 入力
  でない。

## 実走結果（2026-11-21）

- 全静的項目 pass（各 headless lane 済み、blocked-probe 27 rows）。
- interactive lane: `real-dcomp-childinv` = `fail`（honest —
  child-level exstyle write refused + restore `ops_ok=false`）。
  inventory/capture/env/layered/retry/restore の全 step rows
  記録済み。詳細は
  `evidence/rf532/child-inventory-results.md`。
- env variant: `GPUI_DISABLE_DIRECT_COMPOSITION=1` で root
  noredir 消失 → `dcomp_disabled_by_env`、window DC green
  1.0、screen 0.0 を確認（`childinv-dcomp-off.jsonl`）。
