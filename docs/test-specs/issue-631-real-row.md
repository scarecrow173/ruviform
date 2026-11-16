---
id: "TSPEC-034YqGZs8qcuHItS5Ep1JW"
type: "test-spec"
status: "approved"
title: "Issue #631 real-row 拡充 テスト仕様"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-631-real-row.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-real-row.md"
    - "docs/specs/issue-631-real-row.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-631-real-row.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-631-real-row.md"
    - "docs/specs/issue-631-real-row.md"
    - "docs/designs/issue-631-real-row.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-real-row.md"
    - "docs/designs/issue-631-real-row.md"
  verified-by: []
  references:
    - "evidence/rf631/verification.md"
  defers: []
  deferred-by: []
---

# Issue #631 real-row 拡充 テスト仕様

## Feature: catalog 同期（Rule: drift 検出）

- `real_catalog_matches_entries`: `REAL_CATALOG` と `real_entries()`
  が 12 行で case/scenario/required が pairwise 一致。
- `blocked_reports_fail_closed`: session 未開始の blocked path が
  全 12 行に対し `NotRun` + `observed.environment_blocked` +
  `environment_kind != real-window` を生成し、required caps が
  entry と一致する。

## Feature: headless fail-closed probe（Rule: 自己検査）

- `--blocked-probe` は session を開始せず blocked 写像と同一の
  report を生成し、全行 `NotRun`/`environment_blocked`/非
  `real-window` を検査する。違反時 exit 1、正常時 exit 0 で
  JSONL を `--out` に書く。
- probe の detail は session start 未試行を明示し、観測した
  headless run を装わない。

## Feature: AC1 実行可能行（Rule: 観測された効果のみ Pass）

- `real-multiwindow-focus`: `windows()` ≥ 2、`request_focus` →
  `settle_until(window_focused)` で focus 移譲を観測。release の
  driver 応答は記録のみで verdict にしない。
- `real-os-input`: `os_input_preflight` effective が前提。
  `EnvironmentBlocked`→`NotRun`（`observed.environment_blocked`）、
  `Unsupported`/`NotRun` outcome→同 verdict、全注入 `Injected` +
  cursor read-back のみ `Pass`。pointer aim は `plugin.dock` が
  canvas を paint せず bounds を持たないため、実 native slot を
  bind した `plugin.editor` の painted bounds を使う。focus して
  いない window への注入は adapter が `NotRun` を返す規約に従う。

## Feature: AC1 非提供能力行（Rule: 正直な Unsupported）

- `real-window-dpi`/`real-window-close`: 公開 API 非存在を body が
  `Unsupported` で記録（実 session 内でのみ到達）。
- `real-window-ime`: `Ime` 未 claim のため preflight が
  `Unsupported`（body 未到達）。

## Feature: AC3 非画素 editor op（Rule: 画素を oracle にしない）

- `real-editor-resize`: open `Ok` → 子 HWND 契約 →
  `session.set_size` → 定義済み `Unsupported` 拒否 → session が
  `Attached` に留まる → close 収束 `[Attached{Docked}, Closed]`。
  verdict `Unsupported`。受理（将来）時は geometry oracle 未整備
  として `NotRun`（偽 Pass しない）。
- `real-editor-redock`: open `Ok` → 子 HWND 契約 →
  `session.reattach(同一 lease)` → 定義済み拒否 → session
  `Failed` → close 収束、event 系列 `[Attached{Docked}, Detached,
  Failed{..}, Closed]`。verdict `Unsupported`。受理時も oracle
  未整備として `NotRun`。
- 両 case とも pixel capture を呼ばず、Pass oracle は session
  event・HWND 契約・session 状態のみ。

## Feature: 非回帰

- headless lane（`plugin-dock-scenario`/`plugin-dock-console`）の
  verdict 表・real selector 拒否・`--list` 出力契約は不変。
- `check-ui-driver-boundary` green（GPUI 型非漏洩）。
