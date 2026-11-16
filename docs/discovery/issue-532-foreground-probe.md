---
id: "DISC-034YxCYwvosUuX47paEbNO"
type: "discovery"
status: "approved"
title: "Issue #532 foreground-probe discovery — 変異 probe の探索結果"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-foreground-probe.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-foreground-probe.md"
    - "docs/discovery/issue-532-dcomp-occlusion.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-foreground-probe.md"
  derived-by:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-foreground-probe.md"
    - "docs/plans/issue-532-foreground-probe.md"
    - "docs/test-specs/issue-532-foreground-probe.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  defers: []
  deferred-by: []
---

# Issue #532 foreground-probe discovery — 変異 probe の探索結果

## 問題の枠組み

Issue `#532`: GPUI 0.2.2 の Windows DirectComposition/flip-model
composition が plugin child-HWND の画素を覆い、`real-dcomp-pixel`
が screen green 0.0 で恒常 `Fail`。前回 slice
（`issue-532-dcomp-occlusion`）の query-only 診断で:

- docked `test_editor` child rect 内の `WindowFromPoint` 5 点が
  全て外部 `Chrome_RenderWidgetHostHWND` を返す（cover-vs-skip
  判別済み — chain 全 visible/alive/enabled）。
- GPUI 内部 z-order（parent/root の direct-child 列）に遮蔽なし。
- real lane の mount 経路に foreground 化呼出なし。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` で root exstyle から
  `WS_EX_NOREDIRECTIONBITMAP` が消えるが screen green は 0.0 の
  まま（window-DC は 1.0 — child backing surface は green）。

→ **遮蔽の第一境界は ruviform 外側の window stacking** と確定
したが、**compositor 内部遮蔽は `unresolved`** — `WindowFromPoint`
は HWND 層の hit-test であり DComp visual/flip-model plane を
見ないため、外部 window を退かせても compositor 内部で覆われて
いる可能性を排除できない。

## 探索結果（本 slice の入力）

### 変異で二分できるか

GPUI root HWND を `SetWindowPos`/`SetForegroundWindow` で
foreground/topmost 化すれば、外部 HWND stacking による遮蔽は
除去できる。その状態で:

- screen green が出る → 外部 stacking が遮蔽の全原因
  （compositor 内部は正常）。
- `WindowFromPoint` が自 window 系（child/container/root）を返す
  のに screen green が出ない → compositor 内部（DComp visual
  または flip-model plane）が依然 child を覆う → compositor
  内部遮蔽が確定。

→ 変異 probe で二分可能。ただし `WindowFromPoint` が自 window
を返しても DComp visual は見えないため、「自 window 系が返る +
green なし」は compositor 内部遮蔽の強い証拠になるが、
`WindowFromPoint` の限界（DComp visual 非可視）は limitation
として残る。

### Win32 API の利用可能性

`windows-sys 0.61.2` の既存 feature で必要 API が全て揃う:

- `Win32_UI_WindowsAndMessaging`: `SetWindowPos`,
  `HWND_TOP`/`HWND_TOPMOST`/`HWND_NOTOPMOST`,
  `SWP_NOMOVE`/`SWP_NOSIZE`/`SWP_NOACTIVATE`,
  `SetForegroundWindow`, `GetForegroundWindow`,
  `BringWindowToTop`, `GetWindow`(`GW_HWNDPREV`/`GW_HWNDNEXT`),
  `GetWindowLongPtrW`(`GWL_EXSTYLE`), `GetAncestor`(`GA_ROOT`).
- `Win32_Foundation`: `HWND`, `BOOL`, `GetLastError`.

`AttachThreadInput` は `Win32_System_Threading`（既存 feature）
内だが、thread input queue 結合の副作用があるため design で
不採用とした。

### query-only 契約の維持

`osoccl`/`ospix` は「creation/destruction/movement/style/
z-order/region/message mutation を行わない」query-only 契約。
変異は別 module `osmut` に隔離し、`osoccl`/`ospix` は不変。
`osmut` は `osoccl::probe`（hit-test 再観測）と `ospix`
（screen capture 再観測）を呼んで各段階を観測する — 変異は
`osmut` のみが行い、観測は既存 query module を再利用する。

### verdict の設計判断

screen green の有無を verdict 入力にすると「green が出た =
Pass」という tautology になる（probe の目的は観測であって
修正ではない）。verdict は「probe が計画どおり実行できたか +
状態を復元できたか + 共有 lifecycle 契約」に限定し、green
出現の有無は note-level の結果データとして記録する。

foreground 操作は非対称: `SetWindowPos`（自 window の
z-order/band）は foreground lock 対象外で実質常に成功する
べき → FALSE は異常として `Fail`。`SetForegroundWindow` は
foreground lock で FALSE が仕様上予期される → `note` 記録で
verdict を下げない（拒否自体が「foreground 経路は試せない」
という二分データ）。

## 残存する不確実性

- `SetForegroundWindow` がこの host で成功するかは foreground
  lock 次第 — 拒否時は foreground branch が `unresolved`。
- `WindowFromPoint` は DComp visual を見ない — 「自 window 系 +
  green なし」は compositor 内部遮蔽の強い証拠だが、DComp
  visual 層の直接観測ではない（`PrintWindow` root は後続）。
- 外部遮蔽物は環境依存（前回は Chrome、今回は別 window の
  可能性）— baseline hit-test が識別情報を記録する。
- z-order/foreground の完全復元は保証不能 — topmost flag の
  復元のみを検証契約とし、他は観測記録。
