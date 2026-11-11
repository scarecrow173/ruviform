---
id: "TSPEC-034UdQbwnBMc4dBhcTwfxI"
type: "test-spec"
status: "implemented"
title: "Issue #243 GPUI 実窓 lifecycle 実機検証 試験仕様"
created: "2026-11-09"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/issue-243-gpui-realwin-windows.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-243-gpui-realwin-windows.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-243-gpui-realwin-windows.md"
    - "../plans/implement-issue-243-gpui-realwin-windows.md"
  derived-by: []
  verifies:
    - "../specs/issue-243-gpui-realwin-windows.md"
  verified-by:
    - "../impl/ir/issue-243-gpui-realwin-windows.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 GPUI 実窓 lifecycle 実機検証 試験仕様

## Purpose

Issue #243 の Windows 部分スコープで、GPUI Driver の Window lifecycle 契約が
実 OS window 上で成立することを検証する試験仕様。headless fixture
（`run_lifecycle`）が保証する契約のうち実窓で観測可能な部分を、
実測値として記録することを目的とする。3 OS 横断の受入や Plugin Dock 受入を
代替するものではない。

## Feature

GPUI Driver 実窓 Window Lifecycle — Windows 実機検証

## Rules

- Rule: 2 窓以上の `create_window` は各々 `Created` → `Ready` → `Destroyed`
  の lifecycle を独立に発行し、window id は destroy 後に再利用されない。
- Rule: `focus(w, true)` は対象 window に `Focused(true)`、非対象に
  `Focused(false)` を発行する（window 別 focus の独立性）。
- Rule: `focus(w, false)` は gpui 0.2.2 では `UiError::Unsupported` を返す
  （黙って no-op しない契約）。未知 window への `focus` は `UiError::Host`。
- Rule: OS close 要求（`WM_CLOSE`）は veto 可能な `CloseRequested` を発行し、
  `Veto`/`Defer` は window を開いたままにし、`Approve`/無応答既定は
  `Destroyed` に至る。`Destroyed` は window ごとに高々 1 回。
- Rule: `request_close` は `CloseRequested` 経路を通り、`destroy_window` は
  `CloseRequested` を bypass して `Destroyed` に至る。
- Rule: `resize` API は実 window の `Resized` イベントを発行する。
- Rule: Dock 2-phase move・abort・detach/reattach・close 時 view relocate は
  `DockWorkspace` モデル上で所有権が契約どおり遷移する
  （`real_window_lifecycle` example の再実行で確認）。
- Rule: degenerate `WindowSpec`（0x0）に対する driver の応答は実測どおり記録し、
  推測で契約を主張しない。

## Examples

- Example: Given `main`/`veto` 両窓 Ready, When `WM_CLOSE` を veto へ 3 回送る
  （Veto→Defer→Approve と応答）, Then veto は 1・2 回目で開いたまま、
  3 回目で `Destroyed` 1 回のみ発行される。
- Example: Given veto `Destroyed` 後, When `create_window` で reborn を生成,
  Then reborn の `WindowId` は veto/main と異なり `Created`/`Ready` を発行する。
- Example: Given main へ `WM_CLOSE`, When handler が `respond_close` しない,
  Then 無応答既定 Approve で `Destroyed` が発行される。
- Example: Given reborn へ `request_close` → `Veto`, When 続けて
  `destroy_window(reborn)`, Then `CloseRequested` を追加発行せず `Destroyed`。
- Example: Given 0x0 `WindowSpec`, When `create_window`, Then driver の実応答
  （受理/`Destroyed` 縮退/panic）を分類して記録する。

## Guarantees

- 実測ログ（stderr 全量）を impl record に引用し、実行コマンド・OS・
  toolchain・exit code を記録する。
- 各項目を Pass/Fail/NotRun/Unsupported に分類し、OS メッセージ経路
  （`WM_CLOSE`）・gpui 内部 API 経路・手動操作を区別する。
- 観測できなかった項目は NotRun/Unsupported とし理由と owner を明記する。

## Non-goals

- macOS/Linux での同等検証、DPI/モニター変動、IME、物理ユーザー入力、
  Native Plugin Dock、consumer 非回帰 — 全て本 spec の対象外（別 owner）。
- headless 契約自体の再検証（`test-ui-gpui` lane が担う）。

## Risk

実 OS window の観測はタイミング依存であり、focus/幾何イベントの到達順は
環境で変わり得る。probe は lifecycle kind と window id の対応を正本とし、
幾何の絶対値ではなく「イベントが発行されたか」を判定する。
