---
id: "DESIGN-034Ue3PUgmQ7PIOzbrGa9K"
type: "design"
status: "approved"
title: "Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回配線の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added: []
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
  depends-on:
    - "docs/specs/issue-451-focus-next-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-451-keyboard-tab-nav.md"
  refines:
    - "docs/designs/issue-451-keyboard-tab-nav.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-451-focus-next-wiring.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回配線の設計

## Context

FU-1 で selection 付き Tabs ヘッダは `tab_stop(true)` + `track_focus` により GPUI の tab stop に登録済みで、矢印キー等の「タブ内移動」は動く。残るのは「タブ stop 間の移動」— GPUI 0.2.2 は `Window::focus_next` / `focus_prev`（`rendered_frame.tab_stops` 巡回・端で循環）を持つが既定の Tab キーバインドを持たない。配線はホスト側の責務とされてきたが、consumer が個別に書くのではなく `ruviform-ui-gpui` がホストシェルとして供給するのが FU-1b の目的。

GPUI のキー dispatch を確認した結果（gpui 0.2.2 `window.rs` / `key_dispatch.rs`）:

- 要素の `on_key_down` / `on_action` は「フォーカス中ノードから root への dispatch path」上でのみ発火する。フォーカスが無いとき path は root ノード 1 件だけで、view 内のどの要素にも届かない。**要素レベルのハンドラでは「無フォーカス時の Tab」を拾えない。**
- `App::observe_keystrokes` / `intercept_keystrokes` はアプリ全 window のキー入力に対し、要素 dispatch の解決後（observer）/ 前（interceptor）に `(&KeystrokeEvent, &mut Window, &mut App)` で発火する。発火した window がそのまま渡るため multi-window でも正しい window を更新できる。
- `observe_keystrokes` は propagation が止められなかったキーにのみ発火するため、将来フォーカス済み要素が Tab を自前処理して `stop_propagation` した場合はドライバ側の巡回が走らず、コンポーネント優先の自然な優先順位になる。

## Scope

### In Scope

- `GpuiDriver` の `Inner` に「フォーカス巡回の配線済み」フラグを足し、`drain`（`run` の app context と `settle_turn` のテスト経路が共有する唯一の適用点）の先頭で一度だけ `cx.observe_keystrokes` を登録する。
- observer は `keystroke.key == "tab"` のみを処理し、修飾なし → `window.focus_next()`、Shift のみ → `window.focus_prev()`、それ以外の修飾（Ctrl+Tab 等）→ 無視。
- `Subscription` は既存の `app_subs` に保持し、アプリ寿命まで有効にする。
- `gpui-test` 限定の観測 seam: `GpuiObserver::press_key`（フォーカスを動かさずキー名を dispatch）と `GpuiObserver::focused_tab`（フォーカス中の tab strip の `ComponentKey`）。`RvfView::focused_tab_key` を `pub(crate)` で追加。
- `tests/consumer_observation.rs` にフォーカス巡回の headless テストを追加。

### Out of Scope

- `ruviform-ui` 契約・`ruviform-ui-test` の driver-neutral port の変更。
- `RvfView::render` ルート div への `on_key_down` 追加（下記「代替案」参照）。
- Tab stop の追加・削除、tab index / tab group の制御（現状 strip ヘッダのみが stop）。
- 実窓での手動証拠（F3）と実 OS 入力経路の自動化（#313）。

## Components and Boundaries

| 責務 | 置き場所 | 理由 |
| --- | --- | --- |
| Tab / Shift-Tab → `focus_next` / `focus_prev` の写像 | `lib.rs`（`drain` から一度だけ登録する `observe_keystrokes`） | キー巡回は window chrome の既定動作であり、view ではなくドライバ（ホストシェル）が所有する。全 window・全 headless 経路（`run` / `settle_turn`）を一箇所でカバーする |
| tab stop の登録と巡回そのもの | GPUI（`rendered_frame.tab_stops`、`Window::focus_next`/`focus_prev`） | 巡回順・循環は GPUI に委ねる。登録側は要素の `.tab_stop(true)` だけでは不十分で、`track_focus` に渡す `FocusHandle` 自体に `.tab_stop(true)` が必要（下記「実装時の訂正」） |
| フォーカス中 strip の読み出し | `RvfView::focused_tab_key`（`pub(crate)`）+ `GpuiObserver::focused_tab` | 観測 seam 経由でのみ公開し、契約層に GPUI 型を出さない |
| 生キー dispatch | `GpuiObserver::press_key` | `press_tab_key` は strip フォーカスを先に動かすため、Tab 自体の検証には使えない。キー名だけを送る経路が必要 |

## Data and Control Flow

```
OS / test key event ("tab" / "shift-tab")
  -> Window::dispatch_event(KeyDown)
  -> dispatch_key_event: element listeners (focused node -> root)
       - tab header の on_key_down は "tab" を処理しない（tab_key_target が None、stop_propagation しない）
  -> finish_dispatch_key_event -> dispatch_keystroke_observers
  -> driver の observe_keystrokes callback          [lib.rs]
       - key != "tab"            -> 無視
       - modifiers == none       -> window.focus_next()
       - modifiers == shift のみ -> window.focus_prev()
       - その他の修飾            -> 無視
  -> Window::focus(&handle)      [rendered_frame.tab_stops 巡回・循環は GPUI]
```

フォーカス無しのときも `focus_node_id_in_rendered_frame` は root ノードを返し、observer は必ず発火するため `focus_next` は先頭の tab stop に着く。

### 実装時の訂正（gpui 0.2.2 の `track_focus` + `tab_stop`）

当初「FU-1 で strip ヘッダは既に tab stop 登録済み」と想定したが、gpui 0.2.2 の `InteractiveElement::tab_stop` は **自動生成される focus handle にのみ適用**される（`div.rs` の prepaint で `tracked_focus_handle.is_none()` の場合に限り `.tab_stop(self.tab_stop)` を handle へ写す）。`track_focus(&handle)` で明示 handle を渡す経路では要素側フラグが handle に伝播せず、`TabStopMap::insert` が読む `FocusHandle::tab_stop`（既定 `false`）のままになるため `focus_next`/`focus_prev` が strip に到達しない。headless テストでこれを実測確認し、`RvfView::tab_focus` が作る handle を `cx.focus_handle().tab_stop(true)` に修正した。handle の `tab_index` は全 strip で 0 のままとし、巡回順は挿入順（= 描画順）に委ねる。

## Non-Functional Constraints

- 登録は `Inner` フラグで一度だけ。`drain` は tick ごとに走るため、フラグ判定は borrow 1 回の bool 読みで済ませる。
- `#[gpui::test]` の `cx.add_window_view` 経路（render.rs の unit test）はドライバを通らないため今回の配線の対象外 — 検証は `GpuiObserver` seam 経由の統合テストに置く。
- `unsafe` 不使用、公開 enum 追加なし、依存追加なし（`observe_keystrokes` / `focus_next` / `focus_prev` は gpui 0.2.2 既存 API）。

## Risks and Trade-offs

| 案 | 内容 | 判定 |
| --- | --- | --- |
| A | `RvfView::render` ルート div に `on_key_down` | 却下。フォーカスが無いと dispatch path が root ノードのみになり、view 内の要素ハンドラに Tab が届かない。「ウィンドウを開いてすぐ Tab」の期待動作を満たせない |
| B | `cx.bind_keys` + 独自 Action + 要素 `on_action` | 却下。action dispatch も同じ dispatch path 制約を受け、無フォーカス時に届かない。keymap・action 型の追加はこの用途に過剰 |
| C | `intercept_keystrokes` で先取り + `stop_propagation` | 却下。要素・将来のコンポーネントより先に介入し、Tab の自前処理を奪う。observer（事後発火）なら propagation が生きている限り既定動作として機能する |
| D（採用） | `observe_keystrokes` をドライバで一度だけ登録 | 採用。全 window・全経路をカバーし、要素側が処理を止めた場合は自然に譲る |

| リスク | 対応 |
| --- | --- |
| driver と App の対応が 1:1 でない経路で配線が抜ける | 現行の全経路（`run` の `Application`、`GpuiObserver` の `TestAppContext`）は driver 1 つにつき App 1 つ。フラグは `Inner`（driver）単位で、App をまたぐ再利用は存在しない |
| 複数 GpuiDriver が同一 App を共有した場合の多重発火 | `TestAppContext::single()` は呼び出しごとに独立した App を作り、`run` も `Application::new()` を都度作るため発生しない |
| Tab stop が無い window での Tab | `tab_stops.next/prev` が `None` を返し `focus` は呼ばれない — no-op で安全 |
| 実 OS の Shift+Tab が `key="tab", modifiers.shift` として届くか | gpui のプラットフォーム層がキー名を正規化する前提。headless は `Keystroke::parse("shift-tab")` で同形を検証する |

## Verification Notes

- [x] gpui 0.2.2 の `Window::focus_next`/`focus_prev`/`observe_keystrokes`/`Keystroke` API をソースで確認済み。
- [ ] `mise run test-ui-gpui`（新規テスト含む）、`test-ui`、`check`、`lint`、`fmt-check`。
