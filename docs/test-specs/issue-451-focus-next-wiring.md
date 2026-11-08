---
id: "TSPEC-034Ue3Xudk0zUKfypVisTg"
type: "test-spec"
status: "approved"
title: "Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回のテスト仕様"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-451-focus-next-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-451-keyboard-tab-nav.md"
  refines:
    - "docs/test-specs/issue-451-keyboard-tab-nav.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-451-focus-next-wiring.md"
    - "docs/designs/issue-451-focus-next-wiring.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回のテスト仕様

## Purpose

FU-1b の「Tab / Shift-Tab が window の tab stop を巡回する」というドライバ既定動作を、headless（gpui-test）で再現可能に検証する。実 OS のキー入力経路は対象外（#313）であり、本 spec は `Window::dispatch_keystroke` 経由のキー dispatch が同じ `focus_next` / `focus_prev` 経路を通ることを保証する。

## Feature

Tab / Shift-Tab によるウィンドウ内フォーカス巡回（driver-level keyboard focus navigation）。

## Rules

- **R1**: 修飾なしの `tab` キーは `window.focus_next()` を呼び、フォーカスを `rendered_frame.tab_stops` の次の stop へ移す。末尾では先頭へ循環する。
- **R2**: Shift のみを伴う `tab` キー（`shift-tab`）は `window.focus_prev()` を呼び、逆方向へ循環する。
- **R3**: `tab` 以外のキー、および Shift 以外の修飾を伴う `tab`（例: `ctrl-tab`）はフォーカスを動かさない。
- **R4**: フォーカスがどの tab stop にもない状態で `tab` を押すと、先頭の tab stop にフォーカスが移る。
- **R5**: フォーカスが tab stop 上にあるとき `tab` を押すと、次の tab stop（2 つの selection 付き Tabs strip ではもう一方のヘッダ）へ移る。
- **R6**: 巡回は GPUI の tab stop 順（挿入順）に従い、アプリ側で独自の順序を持たない。

## Examples

- **E1（R4+R5+R1）**: selection 付き Tabs strip を 2 つ持つ view を mount する。無フォーカスで `press_key(window, "tab")` → 先頭 strip がフォーカスされる。もう一度 `"tab"` → 2 番目の strip がフォーカスされる。さらに `"tab"` → 循環して先頭に戻る。
- **E2（R2）**: 2 番目の strip がフォーカスされた状態で `press_key(window, "shift-tab")` → 先頭の strip に戻る。
- **E3（R3）**: 任意の状態で `press_key(window, "ctrl-tab")` → フォーカスは変わらない。
- **E4（観測）**: `focused_tab(window)` がフォーカス中 strip の `ComponentKey` を返し、巡回の各段階で期待する strip を示す。

## Guarantees

- Tab / Shift-Tab のフォーカス遷移が headless テストで決定的に検証できる。
- 修飾キー付き Tab が誤って巡回を起こさないことが検証できる。
- フォーカス状態の観測が `GpuiObserver` の公開 seam 経由で可能（gpui 型を consumer に露出しない）。

## Non-goals

- 実 OS ウィンドウでのキー入力（OS→gpui のキー名正規化）は検証しない。
- タブヘッダ内の矢印キー移動（FU-1 で検証済み）は再検証しない。
- Tab stop の追加・削除・並び替えは検証しない（現状 strip ヘッダのみ）。
- webview / fake driver での同等動作は検証しない。

## Risk

- 配線が observer ではなく要素ハンドラに退行すると、無フォーカス時の Tab（R4）が無言で失われる — E1 の最初の `tab` がこれを検出する。
- 修飾判定を誤ると Ctrl+Tab 等が巡回を起こす — E3 がこれを検出する。
- `press_key` / `focused_tab` seam が `gpui-test` 限定でないと本番 API 面が汚れる — feature gate で担保する。
