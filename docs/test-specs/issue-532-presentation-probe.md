---
id: "TSPEC-5QuUji5Eu9kpUAQbq5rtDI"
type: "test-spec"
status: "approved"
title: "Issue #532 presentation-probe test spec"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-presentation-probe.md"
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
    - "docs/specs/issue-532-presentation-probe.md"
    - "docs/designs/issue-532-presentation-probe.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-presentation-probe.md"
    - "docs/designs/issue-532-presentation-probe.md"
  derived-by:
    - "docs/plans/issue-532-presentation-probe.md"
    - "docs/tasks/issue-532-presentation-probe.md"
  verifies:
    - "docs/specs/issue-532-presentation-probe.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #532 presentation-probe test spec

## Purpose

`real-dcomp-present` case と `ospres` module が、spec の機能
要件 1-8 と verdict 意味論を満たすことを保証する。観測値
（cover inventory・attributes・capture variant・fingerprint）は
test の合否入力ではなく evidence として記録されることを保証する。

## Features

### F1: catalog/entry 整合

- `REAL_CATALOG`・`real_entries`・`blocked_reports`・
  `blocked_probe_violations`・console `--include-real` 期待値が
  15 行で一致する。
- `real-dcomp-present` は `real-dcomp-foreground` の直後、
  `required = [NativePluginDock, VisualCapture]`、
  `spec_extras = editor_attach_extras`。

### F2: osres/ospix 境界

- `ospres` は `supported:false` の non-Windows stub を持ち、
  Windows 実装は plain 値のみ返す（HWND は `u64`）。
- `ospix::capture_print_window_flags` は flags を `nflags` に
  そのまま渡し、DIB サイズを `PW_CLIENTONLY` 有無で
  client/window rect に切り替える。既存 `capture_print_window`
  は `PRINT_WINDOW_CLIENT_FULL` wrapper で挙動不変。

### F3: probe 実行と観測記録

- attach lifecycle（`check_attached_children` +
  `conclude_editor_session`）を foreground probe と同じく通る。
- probe 全体で `pres_inventory` を 1 度記録し、各 child に対し
  `pres_attributes`/`pres_cover`/`pres_capture`/`pres_screen`/
  `pres_fingerprint` の step row が記録される。
- 個別 query 失敗は `error`/`skipped`/`None` に縮退し、
  捏造値を出さない。

### F4: verdict 意味論

- `lifecycle_ok && evaluated && completed` → `Pass`。
- probe 前提不成立（container/presented/children/root）→
  `NotRun`（reason 伝播）。
- family-level 早期 return（`supported:false` 等）→ `Fail`。
- 非 Windows/`observation.supported == false` → `Unsupported`。
- screen green・hit-test・fingerprint の値は verdict に影響
  しない。

### F5: 既存 case 非干渉

- `real-dcomp-pixel`/`real-dcomp-foreground` の verdict・
  観測順序が変わらない（`real-dcomp-pixel` は `Fail` 維持）。
- `osmut`/`osoccl`/`oschild` に変更が無い。

### F6: 実走 evidence

- baseline + env-on の interactive lane で case が実走し、
  cover inventory・attributes・capture variant・fingerprint が
  JSONL に記録される。
- `--blocked-probe` で 15 case 全て `not_run` +
  `environment_blocked`。

## Rules

- R1: verdict は probe 実行 + lifecycle のみ — 観測値はデータ。
- R2: probe は read-only — window の作成/破棄/移動/style 変更/
  message 送信を行わない（`PrintWindow` の内部 `WM_PRINT` は
  `ospix` 確立済み read-path の範囲）。
- R3: 全 capture/attribute は backend/path・dimensions・error
  を記録し、失敗を成功に見せない。
- R4: bounded — inventory cap・suspect cap・variant 行列は
  固定、`truncated`/`skipped` を記録する。
- R5: 既存 module の query-only 契約を侵さない — `osmut` は
  変更しない。

## Examples

- Example（F4）: attach 成功・全 family 完了・lifecycle OK →
  `Pass`。screen green=0 でも fingerprint が何を示しても
  verdict は変わらない。
- Example（F4）: container token 無し → `NotRun`（reason
  "no container token"）。
- Example（F3）: `PrintWindow` が foreign suspect で refuse →
  `error` フィールドに win32 error、probe は継続、verdict に
  影響しない。
- Example（F6）: env-on run で root の
  `WS_EX_NOREDIRECTIONBITMAP` が消え、`PW_RENDERFULLCONTENT`
  variant の raster 内容が baseline と異なることが記録される。
