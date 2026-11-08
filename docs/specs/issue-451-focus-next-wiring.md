---
id: "SPEC-034Ue3khxUUmbPdFCx4Mbh"
type: "spec"
status: "approved"
title: "Issue #451 FU-1b — Tab/Shift-Tab によるフォーカス巡回の配線"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-451-focus-next-wiring.md"
      - type: "file"
        path: "docs/plans/issue-451-focus-next-wiring.md"
      - type: "file"
        path: "docs/tasks/issue-451-focus-next-wiring.md"
      - type: "file"
        path: "docs/test-specs/issue-451-focus-next-wiring.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  refines:
    - "docs/specs/issue-451-keyboard-tab-nav.md"
  depends-on:
    - "docs/specs/issue-451-keyboard-tab-nav.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-451-tabs-pointer-descriptor.md"
---

# Issue #451 FU-1b — Tab/Shift-Tab によるフォーカス巡回の配線

## 背景

FU-1（`docs/specs/issue-451-keyboard-tab-nav.md`）で、selection を持つ Tabs ヘッダが `tab_stop(true)` + `track_focus` により GPUI の tab stop として登録され、Left/Right/Home/End のキー操作が実装された。しかし GPUI 0.2.2 は `Window::focus_next` / `focus_prev` を提供するものの既定の Tab キーバインドを持たず、タブヘッダ間を Tab キーで巡回する配線は未実装のまま残っている（FU-1 の設計 7・タスク follow-up FU-1b）。

## 目的

- `ruviform-ui-gpui` が開く全ウィンドウで、Tab キーが `window.focus_next()`、Shift-Tab が `window.focus_prev()` を呼び、`rendered_frame.tab_stops` の巡回でフォーカスが次 / 前の tab stop へ移る。
- フォーカスがどの要素にもない状態でも、Tab で先頭の tab stop にフォーカスが移る（実 OS の Tab 操作と同じ期待動作）。
- 配線はドライバ（ホストシェル相当）に置き、consumer が個別にキー配線を書かなくても `component_showcase` 等でそのまま動く。

## 受入条件

| # | 受入 | 扱い |
| --- | --- | --- |
| F1 | Tab キーで `window.focus_next()`、Shift-Tab で `window.focus_prev()` が呼ばれる配線が `ruviform-ui-gpui` 内にある。循環は GPUI の `rendered_frame.tab_stops` 巡回に委ねる | 本 PR で実装 |
| F2 | headless（gpui-test）で、Tab が次の tab stop へ、Shift-Tab が逆方向へフォーカスを移すことを検証するテストがある | 本 PR で追加 |
| F3 | 実窓での手動証拠（FU-1a: component_showcase での実キー操作）は実施できれば残す。実 OS 入力経路の自動化は #313 の責務 | NotRun 可。理由を PR に明記 |
| F4 | 該当する mise lane（`test-ui-gpui` 等）が pass する | 本 PR で検証 |

## スコープ（本 PR）

- `ruviform-ui-gpui` のドライバ層: アプリ全体で一度だけ登録する keystroke observer が Tab / Shift-Tab を `focus_next` / `focus_prev` に写像する。
- テスト seam: キー名だけを window に dispatch する `press_key` と、フォーカス中の tab strip を返す `focused_tab`（いずれも `gpui-test` 限定の観測 seam）。
- headless テスト: 2 つの selection 付き Tabs strip を持つ view で、Tab / Shift-Tab / 修飾付き Tab のフォーカス遷移を検証する。
- 文書 chain（spec → design → test-spec → plan → task）と docs index。

## 非スコープ

- **`ruviform-ui` の ViewHost 契約。** tab-select intent・`ViewHost::update` 等は確定済みで変更しない。
- **driver-neutral port（`ruviform-ui-test` の trait）への新 seam。** 今回は GPUI 側の観測 seam に限定する（YAGNI）。
- **Seek / Fader / Knob のキーボード操作。** FU-1c として別途扱う。
- **実 OS 入力経路の自動化。** #313 の責務。
- **webview driver での同等配線。** 別 driver の課題。

## 不変条件

- 共通層（`ruviform-ui`）に GPUI 型とキー名を出さない。`check-ui-driver-boundary` が pass すること。
- Tab / Shift-Tab 以外のキー、修飾キーが Shift のみでない Tab（Ctrl+Tab 等）はフォーカス巡回を起こさない。
- フォーカス済み要素が Tab を処理して propagation を止めた場合は、ドライバ側の巡回を行わない（observer は dispatch 解決後にのみ発火する）。
- 公開 API の追加は `gpui-test` 限定の観測 seam に限る。

## 成功指標

- `mise run test-ui-gpui` が、新しいフォーカス巡回テストを含めて pass する。
- `mise run test-ui`、`mise run check`、`mise run lint`、`mise run fmt-check` が pass する。

## Sources and Relations

- Issue #451（Tabs のキーボード操作フォローアップ FU-1b）。
- `docs/specs/issue-451-keyboard-tab-nav.md`（FU-1: タブヘッダのキー操作と tab stop 登録）。
- `docs/designs/issue-451-keyboard-tab-nav.md` 設計 7（GPUI の Tab 循環はホストの `focus_next` バインドに依存する既知の未解決事項）。
