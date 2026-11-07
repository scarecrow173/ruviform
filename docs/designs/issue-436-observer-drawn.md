---
id: "DESIGN-Hv8ZsN3QcKr6YwTe2LmBd1"
type: "design"
status: "proposed"
title: "Issue #436 — GpuiObserver 描画 plan（drawn）公開の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/436"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-436-observer-drawn.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-436-observer-drawn.md"
  depends-on:
    - "docs/designs/issue-429-consumer-observation-seam.md"
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
  related:
    - "docs/specs/issue-436-observer-drawn.md"
---

# Issue #436 — GpuiObserver 描画 plan（drawn）公開の設計

## 概要

#429 の `GpuiObserver` に、描画 plan を返す `drawn(window)` を追加する。描画 plan は #421 で paint 経路が使う純粋関数（`meter_bar_plan` / `axis_mark_plan`）の出力であり、observer はそれを読み取って driver-neutral な DTO へ写す。paint と observer が同じ関数を通るため、観測される値は描画される値と定義上一致する。

## D1: 描画 plan の所有者は render.rs のまま

- plan 計算（`meter_bar_plan` / `axis_mark_plan` / Waveform の bin 判定）は `render.rs` に残す。observer 側で同じ計算を再実装しない。
- 「どの component が何を描くか」の走査（tree 走査）も `render.rs` に 1 関数（`drawn_plans`）として置く。走査規則は `element_for` と同じで、Panel は全子、Tabs は active の子のみ、Meter / TimeAxis / Waveform のみを plan として出す。
- `drawn_plans` は crate 内部型 `DrawPlan` を返す。`observe.rs` は `DrawPlan` を公開 DTO へ写すだけで、paint の規則を知らない。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| observer が `ViewSpec` を自前で走査し、plan 関数を呼ぶ | Tabs の active 規則が二重になる。paint と観測がずれる原因になる |
| 公開 DTO を `render.rs` に直接置き、observe から re-export する | renderer が観測 seam の型に依存する向きになる。DTO の公開面を render 内部の変更に縛る |
| plan 型を `pub` にして公開 API に直接出す | `f32` の内部表現と内部 field 名が公開契約になる。DTO は `f64` で独立させる |

## D2: 公開 DTO は observe.rs に置く（契約型と std 型のみ）

```text
ViewDrawn { window: WindowId, components: Vec<ComponentDrawn> }   // #[non_exhaustive]
ComponentDrawn { key: ComponentKey, plan: DrawnPlan }             // #[non_exhaustive]
DrawnPlan                                                          // #[non_exhaustive] enum
  Meter { bars: Vec<DrawnBar> }
  TimeAxis { marks: Vec<DrawnMark> }
  Waveform { channels: Vec<DrawnWaveformChannel> }
DrawnBar { channel: usize, x0: f64, x1: f64, peak: f64, rms: Option<f64> }   // #[non_exhaustive]
DrawnMark { x: f64, major: bool, label: String }                             // #[non_exhaustive]
DrawnWaveformChannel { channel: usize, bins: Vec<Range<u64>> }              // #[non_exhaustive]
```

- 数値は `f64`。内部 plan の `f32` からの拡張は無損失（`f64::from`）。
- `Range<u64>` は std 型で gpui に依存しない。`bins` は描画される bin の `start..end`（frame）。
- `components` は paint 順（tree 順）。`ViewObservation.components`（key 順）と順序規則が異なる点を doc に明記する。plan は paint 順の意味を持つため。

## D3: Waveform の gap 判定を paint と共有する

現在の paint closure は `count == 0` または min / max の非有限を読み飛ばす。この判定を `waveform_bin_drawn(&WaveformBin) -> bool` に抽出し、paint と `drawn_plans` の双方が使う。paint の挙動（描画される bin の集合）は変わらない。

Waveform の geometry（x 座標・帯の高さ）は paint にだけ残す。plan は「どの bin が描画されるか」を返すのみで、座標は返さない。座標を返すと paint の計算を plan へ移す大きな変更になり、本 Issue の範囲（観測 seam の公開）を超える。

## D4: 取得 API の形

- `GpuiObserver::drawn(&self, window: WindowId) -> Option<ViewDrawn>`。
- 内部では `self.cx.update` の中で `view.read(app).host().spec()` を読み、`render::drawn_plans(&spec.root)` を呼ぶ。`host.spec()` は mount 済み spec であり、settle を要しない。
- 既存の `observe` / `intents` / `settle` / `dispatch_input` は変えない。

## D5: 公開面と export

- `lib.rs` の `observe` 再公開に `ViewDrawn` / `ComponentDrawn` / `DrawnPlan` / `DrawnBar` / `DrawnMark` / `DrawnWaveformChannel` を追加する。
- 追加の feature は作らない（`gpui-test` のまま）。
- `ruviform-ui` の契約は変更しない。

## D6: 境界と非 RT

- DTO は `ruviform-ui` の契約型（`WindowId`・`ComponentKey`）と std 型だけで構成するため、`gui-boundary-guard` の signature 検査で gpui 型は検出されない。
- 非 RT（UI thread）。allocation は観測時のみで、paint 経路には新しい allocation を足さない（paint は従来どおり plan を事前計算する）。

## 影響範囲

- 公開 API: 追加のみ（`GpuiObserver::drawn` と上記 DTO）。既存 API の変更なし。
- paint 経路: Waveform の gap 判定を `waveform_bin_drawn` へ抽出（挙動不変）。Meter / TimeAxis の plan 関数は変更なし。
- 既存の内部 hook（`settle_for_test` / `inbox_len_for_test` / `view_for_test`）は変更なし。

## 限界

- 観測できるのは Meter / TimeAxis / Waveform のみ。Knob / Toggle / Fader 等の描画 primitive は本 Issue の対象外。
- Waveform は bin 範囲のみ。振幅（min / max / rms）の観測は含めない（必要なら follow-up）。
- `drawn` は window inbox を drain しない（`observe` と同じ）。`post_event` が inbox に滞留している間の `drawn` は、次の paint で投影される値と一時的に異なりうる。観測は `settle` の後に行う前提とする。
- 描画 plan は spec に基づく純粋計算であり、実際の pixel 出力（GPU 描画）を観測するものではない。実機の描画は manual verification。

## 検証

| 観点 | 検証 | 実行方法 |
| --- | --- | --- |
| Meter drawnBars の一致 | `observes_drawn_meter_and_axis_plans_through_seam` | `mise run test-ui-consumer-observation` |
| TimeAxis drawnMarks の一致 | 同上 | 同上 |
| Waveform bin 範囲（gap 除外） | `observes_drawn_waveform_bins_through_seam` | 同上 |
| 未知 window の `None` | `unknown_window_has_no_view_to_observe_or_drive` | 同上 |
| Tabs の active 規則と gap / clamp | `drawn_plans_follow_paint_tree`（render.rs） | `mise run test-ui-gpui` |
| 公開面の gpui 非露出 | `gui-boundary-guard` | `mise run check-ui-driver-boundary` |
