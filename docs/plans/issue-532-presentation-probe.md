---
id: "PLAN-8a496Ntecdl8SX97zqENF0"
type: "plan"
status: "approved"
title: "Issue #532 presentation-probe 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-532-presentation-probe.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/specs/issue-532-presentation-probe.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-presentation-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-532-presentation-probe.md"
    - "docs/specs/issue-532-presentation-probe.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-presentation-probe.md"
  derived-by:
    - "docs/tasks/issue-532-presentation-probe.md"
  verifies:
    - "docs/test-specs/issue-532-presentation-probe.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #532 presentation-probe 実装計画 Implementation Plan

## Goal

`real-dcomp-present` case と `ospres` module を追加し、
foreground-probe が残した 3 候補（hit-test-transparent cover /
child presentation / capture 盲点）を判別する観測データを
baseline + env-on の interactive lane で採取する。修正・方式
選定・Issue close は行わない。

## Scope

- `crates/ruviform-plugin-dock-test/` 内の実装変更 +
  doc chain + `evidence/rf532/`・`evidence/rf631/` 記録。
- consumer crate・adapter・GPUI vendor・production compositor・
  `mise.toml`・docs index は変更しない。

## Upstream Documents

- spec: `docs/specs/issue-532-presentation-probe.md`（approved）
- design: `docs/designs/issue-532-presentation-probe.md`（approved）
- test-spec: `docs/test-specs/issue-532-presentation-probe.md`
- task: `docs/tasks/issue-532-presentation-probe.md`

## Design Inputs

- `evidence/rf532/foreground-probe-bisect.md` — 残余 3 候補と
  「hit-test が自 HWND を返しても green 0」の確定事実。
- `docs/specs/issue-532-foreground-probe.md` — verdict 意味論
  （probe 実行が verdict、観測値はデータ）の先例。

## File Map

| file | 変更 |
| --- | --- |
| `src/ospres.rs` | 新規 — top-level inventory + presentation attributes（query-only、Windows impl + non-Windows stub） |
| `src/ospix.rs` | `capture_print_window_flags` 追加、`capture_print_window` を wrapper 化 |
| `src/real_dock.rs` | `real-dcomp-present` case + probe/fingerprint helper + `real_entries` 15 要素化 + unit test 件数更新 |
| `src/real_catalog.rs` | `REAL_CATALOG` に `real-dcomp-present` 行追加（15 行） |
| `src/lib.rs` | module doc の case 一覧に `real-dcomp-present` 追記 |
| `src/Cargo.toml` | windows-sys feature comment に presentation probe の API 面を追記（依存変更なし） |
| `tests/console.rs` | `--include-real` 期待値 14→15 行・case 一覧更新 |
| `README.md` | real-lane case 表に `real-dcomp-present` 行追加 |
| `evidence/rf532/presentation-probe.md` | 判別結果の正本文書（新規） |
| `evidence/rf631/verification.md` | slice 検証記録の追記 |
| lane JSONL 3 本 | baseline / dcomp-disabled / headless の再生成 |

## Implementation Sequence

1. `ospix::capture_print_window_flags` + 既存 wrapper 維持。
2. `ospres` module（inventory + attributes、stub 含む）。
3. `real_dock` probe 本体 + catalog/entries/console/docs 整合。
4. fmt/check/lint/test（default + real-dock）→ lane 群 →
   interactive 実走 2 本 + headless probe。
5. evidence/impl docs → review → merge `origin/main` → commit/
   push/PR。

## Dependencies and Risks

- `windows-sys` 既存 feature で必要 API が全て到達可能か実装時に
  確認（`EnumWindows`/`GetWindowRgn`/`GetRgnBox`/`CreateRectRgn`/
  `GetUpdateRect`/`GetLayeredWindowAttributes`/`GetCurrentProcessId`）。
- `dwmapi` local extern の link 名・`DWMWA_CLOAKED=14` の ABI
  確認。
- `PrintWindow` flag 別の raster 座標系（client vs window rect）
  の取り扱い — design の sizing 規約どおり実装し、実走で
  `size_mismatch` が出た場合は記録のまま残す。

## Task Breakdown

`docs/tasks/issue-532-presentation-probe.md` 参照（単一 task、
実装→検証→証拠→review→統合の順序付き steps）。

## Verification

spec の受入基準 + test-spec の F1-F6 に従う。interactive lane は
baseline（env なし）と `GPUI_DISABLE_DIRECT_COMPOSITION=1`
env-on の 2 本。headless は `--blocked-probe` で fail-closed
mapping を検証。

## Open Questions / Gaps

- `PW_RENDERFULLCONTENT` が GPUI root の DComp content を実際に
  捕捉するかは実装依存 — 実走データが答え。
- foreign top-level の子孫 HWND（hit-test が返す
  `Chrome_RenderWidgetHostHWND` 等）は inventory に現れない —
  fingerprint suspect に hit-test HWND を直接含める設計で
  カバー。網羅的な子孫列挙は evidence 次第で後続判断。

## Self-Review

- spec 要件 1-8 と file map が対応している。
- 変異なしの観測専用構成は foreground slice の「変異しても
  green が戻らない」結果と整合する。
- verdict は probe 実行+lifecycle のみ — 観測値を verdict に
  しない先例どおり。

## Implementation Handoff

`docs/tasks/issue-532-presentation-probe.md` の steps に従って
実装する。

## Review Handoff

完了後 `review-protocol` で self-review し、blocker を解消して
から merge/PR へ進む。
