---
id: "TSPEC-034ZaaBandEscapeProbe01"
type: "test-spec"
status: "approved"
title: "Issue #532 band-escape probe test spec"
created: "2026-11-20"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-band-escape.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-532-uncovered-baseline.md"
    - "docs/specs/issue-532-band-escape.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-band-escape.md"
    - "docs/designs/issue-532-band-escape.md"
    - "docs/discovery/issue-532-band-escape.md"
    - "evidence/rf532/band-escape-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-band-escape.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-band-escape.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 band-escape probe test spec

## Purpose

`real-dcomp-bandescape` case と `osmut` container mutation
family が、(1) catalog/entry 整合、(2) staged mutation の
honest 記録（verified-write 含む）、(3) verified restore、
(4) 既存 verdict 意味論、(5) fail-closed の headless 挙動、を
満たすことを検証する。

## Features

### F1: catalog/entry 整合

- `real_entries` は `[ScenarioEntry; 24]`、
  `real-dcomp-bandescape` は `real-dcomp-uncovered` の直後。
- `REAL_CATALOG`（`real_catalog.rs`）に同名 row が存在し、
  required = `native-plugin-dock` + `visual-capture`。
- `plugin-dock-console --list --include-real` は 33 行、
  `real-dcomp-bandescape` を `real-dcomp-uncovered` の次に含む。
- 検証: `tests/console.rs`（33 行 assertion + selector 一覧）、
  `real_dock::tests::real_catalog_matches_entries`。

### F2: container mutation の honest 記録

- `be_apply_*` step は `ContainerMutationOutcome`（api/applied/
  win32_error/after snapshot）を持つ。
- `exstyle_write` は read-back 検証付き: last-error 成功でも
  `GetWindowLongPtrW` read-back で要求値が確認できなければ
  `applied: false` — silent drop を成功と誤記しない。
- `ContainerSnapshot` は `class_style`（`GCL_STYLE` raw +
  `own_dc`/`class_dc` decode）を含む。
- 検証: `cargo test --features real-dock`（unit）、実走 JSONL の
  step 構造。

### F3: probe 実行と verified restore

- probe 順序: baseline 観測 → `be_apply_repaint` →
  `be_apply_layered_{exstyle,frame,lwa}` → settle →
  `be_apply_reparent_{exstyle,frame,parent,rect}` → settle →
  `be_restore` → `be_observe_restore`。
- `be_restore` は `parent_restored`/`exstyle_restored`/
  `rect_restored` を deterministic に検証し、z-order は記録のみ。
- `repaint_container` は `RDW_UPDATENOW` で update region を
  drain し、`be_update_pending` で滞留の有無を記録する。
- 検証: 実走 JSONL（`be_restore.verdict=pass` + 3 boolean）。

### F4: verdict 意味論

- required mutation op（`be_apply_repaint`・`*_frame`・
  `be_apply_layered_repaint`・`be_apply_reparent_*`・`be_restore`
  — いずれも legitimate な拒否経路を持たない）の拒否
  （applied=false）は `fail`（probe 未完走の事実記録）。
- as-child `be_apply_layered_exstyle`/`be_apply_layered_lwa` の
  拒否は確定 class-C 診断（`WS_EX_LAYERED` は `WS_CHILD` に
  latch しない — `docs/discovery/issue-532-dcomp-residual.md`）
  の期待値であり、`verdict: note` + `expected_outcome` 付きの
  診断行として記録され verdict 入力にしない（Issue #917 による
  「必須 mutation 拒否 = fail」契約の意図的改訂 — refusal の
  `applied`/`win32_error`/`after` 記録は維持され Fail→Pass の
  改竄ではない）。as-child layered の latch 成功は確定機序と
  矛盾する OS 依存の新規 data として
  `contradicts_confirmed_mechanism` flag で記録し Pass を妨げない
  — lane が計るのは probe 契約の完遂であって OS 実装詳細の
  不変性ではないため。
- green/hit-test/class_style は note 級 data で verdict に
  影響しない。
- lifecycle（attach/close）の破壊も `fail`。
- 検証: 実走 JSONL の verdict + detail。

### F5: 実走 evidence

- canonical `mise run test-ui-plugin-dock-real`（Windows
  interactive）の JSONL + `--pixel-artifacts` の stage PNG
  （`{baseline,repaint,layered,reparent,restore}-0.png`）を
  `evidence/rf532/` に保存。
- `evidence/rf532/band-escape-results.md` に fix-decision
  table（layered 経路 / reparent 経路 / 次 direction）と
  limitations を記録。

### F6: fail-closed headless

- `--blocked-probe` で `real-dcomp-bandescape` が
  `not_run`/`environment_blocked` を返し、session-scoped
  field を捏造しない。
- 検証: `mise run test-ui-plugin-dock-real-headless` +
  `blocked_probe_rows_carry_no_session_scoped_fields`。

## Rules

- `osmut` 以外（`osoccl`/`ospix`/`ospres`）に mutation を置かない。
- public boundary に Win32 型を出さない（HWND は `u64`、
  exstyle/class_style は `u64` 生値）。
- 非 Windows stub は `applied: false`/`supported: false` で
  成功を捏造しない。
- `mise.toml` の task 定義は変更しない（本 slice は新 lane 不要）。
- `Refs #532` のみ — `Fixes`/`Closes` 等の auto-close keyword
  禁止。

## Examples

- `be_apply_layered_exstyle` — last-error 成功でも
  `after.exstyle.value` に `WS_EX_LAYERED`（`0x80000`）が無ければ
  `applied: false`/`win32_error: null`/`verdict: note` +
  `expected_outcome`（as-child では latch しない確定 class-C
  機序どおりの期待拒否 — Issue #917 改訂前は `verdict: fail`
  だった）。
- `be_apply_reparent_parent` — `SetParent(GetDesktopWindow())`
  成功で `after.parent` が desktop HWND（実走では `65548`）に
  変化、container が GPUI root の直上 z-order に。
- `be_restore` — reparent した container を baseline parent へ
  戻し、`ScreenToClient` 変換した baseline rect を再適用。
  `parent_restored`/`exstyle_restored`/`rect_restored` = true。
