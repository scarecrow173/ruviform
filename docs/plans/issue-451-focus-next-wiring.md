---
id: "PLAN-034Ue8cy65F5L1MWmEzj3E"
type: "plan"
status: "approved"
title: "Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回配線の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-451-focus-next-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-451-focus-next-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/plans/issue-451-keyboard-tab-nav.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-451-focus-next-wiring.md"
    - "docs/designs/issue-451-focus-next-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-451-focus-next-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回配線の実装計画

## Goal

`ruviform-ui-gpui` のドライバが Tab / Shift-Tab を `Window::focus_next` / `focus_prev` に配線し、gpui-test でフォーカス巡回を検証する。

## Scope

単一の実装ストリーム。`ruviform-ui-gpui` 内に閉じ、契約層・他 driver・fixture は変更しない。

## Upstream Documents

- `docs/specs/issue-451-focus-next-wiring.md`
- `docs/designs/issue-451-focus-next-wiring.md`
- `docs/test-specs/issue-451-focus-next-wiring.md`

## Design Inputs

- `docs/designs/issue-451-focus-next-wiring.md`（案 D: `observe_keystrokes` をドライバで一度だけ登録）

## File Map

- `crates/ruviform-ui-gpui/src/lib.rs` — `Inner` に `focus_nav_wired: bool` を追加し、`drain` 先頭で一度だけ `cx.observe_keystrokes` を登録する `wire_focus_navigation` を追加。
- `crates/ruviform-ui-gpui/src/render.rs` — `RvfView::focused_tab_key(&self, window) -> Option<ComponentKey>`（`pub(crate)`、読み取りのみ）と、`tab_focus` が作る `FocusHandle` への `.tab_stop(true)` 付与（gpui は `track_focus` handle に要素側 `tab_stop` を伝播しないため必須）。
- `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` — `GpuiObserver::press_key`（キー名のみ dispatch）と `GpuiObserver::focused_tab`（フォーカス中 strip の `ComponentKey`）を `gpui-test` 限定で追加。
- `crates/ruviform-ui-gpui/tests/consumer_observation.rs` — selection 付き Tabs strip を 2 つ持つ view で Tab / Shift-Tab / Ctrl+Tab のフォーカス遷移を検証するテストを追加。
- `docs/**` — 本文書 chain と各 README 索引。

`Cargo.toml` / `Cargo.lock` は変更しない。依存追加なし。

## Implementation Sequence

- [x] S1: `Inner` フラグと `wire_focus_navigation`（`observe_keystrokes` → `focus_next`/`focus_prev`、Shift のみ → prev、その他修飾は無視）を `lib.rs` に実装。
- [x] S2: `RvfView::focused_tab_key` と `GpuiObserver::press_key` / `focused_tab` seam を追加。
- [x] S3: `consumer_observation.rs` にフォーカス巡回テスト（無フォーカス→先頭、Tab→次、循環、Shift-Tab→逆方向、Ctrl+Tab→不変）を追加。
- [x] S4: `fmt` → `lint-ui-gpui` → `test-ui-gpui` → `test-ui` → `check` → `fmt-check` を実行。
- [ ] S5: main 取り込み、docs 索引再生成、review-protocol レビュー、PR 作成。

## Dependencies and Risks

- gpui 0.2.2 の `observe_keystrokes` / `focus_next` / `focus_prev` / `Keystroke::parse` は既存依存で利用可能（ソース確認済み）。
- 無フォーカス時も observer は発火する（dispatch path = root ノード）が、要素が propagation を止めた場合は発火しない — これは意図した優先順位。
- driver と App が 1:1 でない経路は現行に存在しない（設計リスク表参照）。

## Task Breakdown

- T1（S1+S2）: 配線と seam の実装。
- T2（S3）: headless テスト追加。
- T3（S4+S5）: 検証・レビュー・PR。

## Verification

- [x] `mise run test-ui-gpui` — 新規フォーカス巡回テストを含め pass。
- [x] `mise run test-ui` — 共有 fixture / parity が無回帰。
- [x] `mise run check` / `mise run lint` / `mise run fmt-check` — workspace 健全性。
- [x] `mise run check-ui-driver-boundary`（存在すれば）— 契約層に gpui 型を出さない。
- [ ] 実窓手動証拠（F3）— NotRun（実 OS 入力経路の自動化は #313 の責務。PR に理由を明記）

## Open Questions / Gaps

- 実 OS の Shift+Tab が gpui プラットフォーム層で `key="tab", modifiers.shift` に正規化されるかは実機依存。headless は `Keystroke::parse("shift-tab")` の同形で検証し、実機は F3 の後続に委ねる。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（Orca worker が直接実装）。
- Independent work streams: none。
- User approval required before dispatch: review-protocol による blocker 無し確認で代替。

## Review Handoff

- observer 登録点が `drain` 先頭で全経路をカバーすること、`app_subs` で寿命管理されること、修飾判定が「なし→next / shift のみ→prev / その他→無視」であることを重点確認。
