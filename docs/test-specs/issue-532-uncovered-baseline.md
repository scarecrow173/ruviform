---
id: "TSPEC-034ZaaUncoveredBaseline01"
type: "test-spec"
status: "approved"
title: "Issue #532 uncovered-baseline bisect test spec"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-uncovered-baseline.md"
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
    - "docs/specs/issue-532-uncovered-baseline.md"
    - "docs/designs/issue-532-uncovered-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-uncovered-baseline.md"
    - "docs/designs/issue-532-uncovered-baseline.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-uncovered-baseline.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #532 uncovered-baseline bisect test spec

## Purpose

`real-dcomp-uncovered` case と `ospres::Inventory.own_windows` が
spec の受入基準を満たすことを検証する。bisect 結果自体は実走
evidence であり、ここの test は probe の実行・復元・verdict
意味論・catalog 整合を保証する。

## Features

### F1: catalog/entry 整合

- `REAL_CATALOG` が 23 行で `real-dcomp-uncovered` が
  `real-dcomp-present` 直後にある。
- `real_entries`/`blocked_reports`/`blocked_probe_violations` が
  23 要素で同一順。
- `tests/console.rs` の `--list --include-real` が 32 行で
  `real-dcomp-uncovered` を含む。

### F2: own-process inventory

- `ospres::Inventory.own_windows` が自 process の top-level
  window を cap 非依存で列挙し、真の `z_index`（全列挙内の
  index）を持つ。
- 非 Windows stub が `own_windows: []` を返す。

### F3: probe 実行と復元

- attach lifecycle（`check_attached_children` +
  `conclude_editor_session`）が既存 dcomp case と同一に動く。
- 変異は baseline 観測 → raise → foreground → topmost →
  uncovered 判定 → settle 3 段 → capture variants → restore の順。
- `osmut::restore` が topmost flag を baseline 値に戻し、
  `topmost_restored` を検証する。
- `SetForegroundWindow` FALSE は `applied:false`+`win32_error`
  で記録され、成功に偽装しない。
- `uncovered:false` でも観測は続行し、判定を捏造しない。

### F4: verdict 意味論

- case verdict は probe 実行 + 復元 + lifecycle のみ。
- screen green・hit-test・`uncovered`・`has_update` の観測値は
  verdict 入力にしない（note-level の bisect データ）。
- z-order 変異 API の FALSE → `Fail`、`SetForegroundWindow`
  FALSE → `note`（foreground 非対称規則）。
- 復元検証失敗 → `Fail`。
- 非 Windows → `Unsupported`。

### F5: 実走 evidence

- baseline（env 未設定）と env-on
  （`GPUI_DISABLE_DIRECT_COMPOSITION=1`）の interactive lane で
  `real-dcomp-uncovered` が実走し、inventory・各段階の
  mutation/hit-test/screen green・settle 各段・`has_update`・
  capture variants・restore 結果が JSONL に記録される。
- `real-dcomp-pixel` verdict が `Fail` を維持する。

## Rules

- R1: 変異 API は `osmut` のみに置き、`osoccl`/`ospix`/`ospres`
  の query-only 契約を侵さない。
- R2: HWND は `u64`、Win32/GPUI 型を crate 公開境界に出さない。
- R3: 復元は close/detach/converge より前に完了させ、後続 case
  の観測条件を変えない。
- R4: verdict は「probe 実行+復元」に対する honest oracle で
  あり、bisect 結果データを verdict に混入しない。
- R5: settle の待機は `settle_ms`/`settle_kind` として必ず記録
  する（暗黙の等待禁止）。

## Examples

- EX1: `ospres::inventory()` が cap 超過環境でも `own_windows`
  に自 process の window を全件、真の `z_index` 付きで返す。
- EX2: settle ループが `uc_observe_settled_200ms`/`500ms`/`1200ms`
  の hit-test+screen 行と、`settle_ms`/`settle_kind`/per-child
  `uncovered`/`update_pending` を持つ `uc_settle` 行を出す。
- EX3: interactive lane で `uc_apply_*`/`uc_observe_*`/`uc_capture`/
  `uc_restore` row が JSONL に出る。
- EX4: `real-dcomp-foreground` の step 名が `fg_*` のまま変わら
  ない（`ProbeLabels` 注入による非回帰）。
