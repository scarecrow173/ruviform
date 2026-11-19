---
id: "TSPEC-034XWO2xGlAsmD12I1K5tl"
type: "test-spec"
status: "approved"
title: "Issue #631 Headless Scenario Console のテスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
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
    - "docs/plans/issue-631-scenario-console.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-scenario-console.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Headless Scenario Console のテスト仕様

## Purpose

headless Scenario Console（`plugin-dock-console` bin）が
「catalog の capability projection を evidence に見せない」
「selector は fail-closed」「部分実行は実際に実行された case の
verdict のみを記録する」「real lane は console で実行しない」
という契約を保証する。本 spec は GUI console slice 到着後も
console 層の契約として残す。

## Feature: `--list` の capability projection

### Rule: `--list` は `HEADLESS_MATRIX.project()` のみを出力し verdict 語彙を含まない

- **Example**: 出力は 8 行の JSON で、各行が
  `{type:"catalog", case, scenario, driver, required, runnable,
  missing}` を持つ。`view-mount` は `driver:"fake"`・
  `runnable:true`・`missing:null`、probe 7 case は
  `driver:"plugin-host-probe"`。
- **Example**: 出力全体に `"verdict"` key も verdict 語彙
  （`pass`/`fail`/`not_run`/`unsupported`/`blocked`）も現れない
  — 一覧は実行証拠ではない。

### Rule: `--include-real` は real lane を正直に扱う

- **Example**: `real-dock` feature build では 12 行出力され、末尾
  4 行が `driver:"real-os"`・`session_required:true`・
  `lane:"real-window-dock"`・`runnable:null`・`missing:null` を
  持ち、実行を主張しない。
- **Example**: feature なし build では exit 2 + real lane が
  compile-in されていない旨のメッセージ（要求を暗黙に棄却しない）。

## Feature: `--run` の selector 実行

### Rule: selector は catalog 順の部分実行で lane と同一 verdict を記録する

- **Example**: `--run`（selector なし）は 8 case 全件を実行し、
  JSONL header が `run_id:"rf631-plugin-dock"`・
  `lane:"plugin-dock-console"`、case verdict が
  `plugin-dock-scenario` lane と同一。
- **Example**: `--run --case view-mount` は 1 case のみを実行し
  `pass`。`--run --driver fake` も同じ 1 cell に一致する
  （同一 flag 内 OR・flag 間 AND）。
- **Example**: `--run --case editor-attach` は artifact 前提 case
  を含まないため `not_run`（「plugin artifact unavailable」）を
  記録し exit 0 — 前提未実行を Pass に見せない。
- **Example**: `--run --case artifact --case editor-attach` では
  catalog 順で `artifact` が先に走り `editor-attach` は
  `unsupported`（fixture に editor 無し）を記録。

### Rule: unknown selector・一致 0 件は exit 2 で何も実行しない

- **Example**: `--run --case bogus`・`--run --driver bogus`・
  `--run --scenario bogus` は exit 2 で stdout に JSONL を出さない。
- **Example**: `--run --driver fake --case artifact`（各値は既知
  だが組合せ一致 0 件）も exit 2。

### Rule: real-lane selector は console で実行せず exit 2 で誘導する

- **Example**: `--run --driver real-os`・`--run --case
  real-window-mount`・`--run --scenario rf631.plugin-dock-real`
  は exit 2 + `plugin-dock-real-scenario` lane を使う旨の
  メッセージ。console に 2 つ目の real executor はない。

### Rule: exit code は verdict 規約に従う

- **Example**: いずれかの verdict が `fail` なら exit 1
  （例: `PLUGIN_PATH_ENV` に壊れたファイルを指し `--run --case
  artifact` → `Vst3Library::load` 失敗 → `fail`）。
- **Example**: `not_run`/`unsupported`/`blocked` のみなら exit 0。

## Feature: catalog metadata の drift 防止

### Rule: `REAL_CATALOG` は `real_entries()` と一致する

- **Example**: feature-gated test が `real_entries()` の
  case/scenario/required と `REAL_CATALOG` 4 行を zip 比較し、
  drift で fail する。
- **Example**: `real_dock` の spec_extras が名乗る `lane` は
  `real_catalog::LANE`（`"real-window-dock"`）と同一 const を共有。

## Guarantees

- console 経路に GPUI 型を漏らさない（`real_catalog` は
  driver-neutral metadata のみ — `check-ui-driver-boundary` green、
  `boundary.json` 不変）。
- `run_all`/既存 bin/JSONL schema の外部 contract は不変
  （`run_all` = `run_selected` の select-all 投影、lane verdict
  非回帰）。
- JSON DSL・独自 executor・二重 runner を作らない
  （`issue-464` design の制約 — catalog helper の再利用のみ）。

## Non-goals

- GPUI 製 Scenario Console / Test Lab GUI の描画・操作。
- real lane の console 経由実行・real executor の統合。
- `mise.toml` lane task・CI workflow（別 Worker / 後続 slice）。
- 実 Native Dock Pass・dock-pixels・child HWND 判定
  （`test_editor` 未到着のため NotRun/Unsupported 維持）。

## Risk

- selector 規則（同一 flag OR・flag 間 AND）の誤解 — doc/spec に
  明記し、test で `--driver fake`/`--case` 組合せを固定。
- `REAL_CATALOG`↔`real_entries()` drift — consistency test で
  検出（feature build の test lane で担保）。
