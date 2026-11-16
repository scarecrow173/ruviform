---
id: "TSPEC-034YxD7ux8VjsYpPupK624"
type: "test-spec"
status: "approved"
title: "Issue #532 foreground-probe test spec"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-foreground-probe.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/test-specs/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-foreground-probe.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-foreground-probe.md"
  derived-by:
    - "docs/plans/issue-532-foreground-probe.md"
  verifies:
    - "docs/specs/issue-532-foreground-probe.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #532 foreground-probe test spec

## Purpose

`real-dcomp-foreground` case と `osmut` module が spec の受入基準を
満たすことを検証する。二分結果自体は実走 evidence であり、
ここの test は probe の実行・復元・verdict 意味論・catalog 整合を
保証する。

## Features

### F1: catalog/entry 整合

- `REAL_CATALOG` が 14 行で `real-dcomp-foreground` が
  `real-dcomp-pixel` 直後にある。
- `real_entries`/`blocked_reports`/`blocked_probe_violations` が
  14 要素で同一順。
- `tests/console.rs` の `--list --include-real` が 22 行で
  `real-dcomp-foreground` を含む。

### F2: osmut plain-value 境界

- `RootSnapshot`/`MutationOutcome`/`RestoreOutcome` が `u64`/
  bool/整数/`String`/`Option`/`Vec` の plain 値のみを持ち、
  Win32 型を公開しない。
- 非 Windows stub が `supported:false`・全 field None/空を返す。

### F3: probe 実行と復元

- attach lifecycle（`check_attached_children` +
  `conclude_editor_session`）が `real-editor-attach` と同一に
  動く。
- 変異は baseline 観測 → raise → foreground → topmost → restore
  の順で、各段階で hit-test + screen green を再観測する。
- `osmut::restore` が topmost flag を baseline 値に戻し、
  `topmost_restored` を検証する。
- `SetForegroundWindow` FALSE は `applied:false`+`win32_error`
  で記録され、成功に偽装しない。

### F4: verdict 意味論

- case verdict は probe 実行 + 復元 + lifecycle のみ。
- screen green 出現の有無・`WindowFromPoint` 回答は verdict
  入力にしない（note-level の結果データ）。
- z-order 変異 API の FALSE → `Fail`、`SetForegroundWindow`
  FALSE → `note`（foreground 非対称規則）。
- 復元検証失敗 → `Fail`。
- 非 Windows → `Unsupported`。

### F5: 実走 evidence

- baseline（env 未設定）と env-on
  （`GPUI_DISABLE_DIRECT_COMPOSITION=1`）の interactive lane で
  `real-dcomp-foreground` が実走し、各段階の適用結果・hit-test・
  screen green・復元結果が JSONL に記録される。
- `real-dcomp-pixel` verdict が `Fail` を維持する。

## Rules

- R1: 変異 API は `osmut` のみに置き、`osoccl`/`ospix` の
  query-only 契約を侵さない。
- R2: HWND は `u64`、Win32/GPUI 型を crate 公開境界に出さない。
- R3: 復元は close/detach/converge より前に完了させ、後続 case
  の観測条件を変えない。
- R4: verdict は「probe 実行+復元」に対する honest oracle で
  あり、二分結果データを verdict に混入しない。

## Examples

- EX1: `osmut::observe` が root/exstyle/topmost/above_count/
  above_hwnd/below_hwnd/foreground を plain 値で返す。
- EX2: `raise_to_top` が `SetWindowPos(HWND_TOP,
  SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)` を呼び `applied`/
  `win32_error`/適用後 snapshot を返す。
- EX3: `restore` が NOTOPMOST（または baseline topmost に応じ
  TOPMOST）+ above-neighbor reposition + foreground 復帰試行を
  行い、`topmost_restored`/`zorder_before/after`/
  `foreground_before/attempted/final` を記録する。
- EX4: interactive lane で `real-dcomp-foreground` が全段階を
  実行し、`fg_apply_*`/`fg_baseline_*`/`fg_restore*` row が
  JSONL に出る。
