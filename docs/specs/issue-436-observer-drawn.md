---
id: "SPEC-Qm4Wd7Lz1RbNc8Xk3TgHs5"
type: "spec"
status: "proposed"
title: "Issue #436 — GpuiObserver の描画 plan（drawn）公開"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/436"
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/429"
    - "https://github.com/scarecrow173/audio-lens/pull/166"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-436-observer-drawn.md"
      - type: "file"
        path: "docs/designs/issue-436-observer-drawn.md"
      - type: "file"
        path: "docs/plans/issue-436-observer-drawn.md"
      - type: "file"
        path: "docs/tasks/issue-436-observer-drawn.md"
      - type: "file"
        path: "docs/test-specs/issue-436-observer-drawn.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-429-consumer-observation-seam.md"
    - "docs/designs/issue-429-consumer-observation-seam.md"
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #436 — GpuiObserver の描画 plan（drawn）公開

## 背景

#429（PR #432）で `GpuiObserver` が追加され、consumer test は mount 済み view の component snapshot と intent を観測できる。一方、描画結果（Meter の bar、TimeAxis の mark、Waveform の描画 bin）は観測できない。

#421 で追加された Fake/GPUI parity fixture `parity/new-kinds.json` は `expect.meter.drawnBars` と `expect.timeAxis.drawnMarks` を持つが、この検査は `ruviform-ui-gpui` の crate 内テスト（`render.rs`）でのみ行われている。AudioLens（audio-lens#166）の GPUI parity test は描画 parity を consumer 側から検証できず、NotRun のまま残っている。

## 問題

- `GpuiObserver` の公開 API（`observe` / `intents` / `dispatch_input` / `settle` / `inbox_len`）に描画結果を取得する手段がない。
- 描画 plan を計算する関数（`meter_bar_plan` / `axis_mark_plan`）は `render.rs` の非公開関数であり、paint 経路と crate 内テストからのみ使われる。
- Waveform は描画 plan の関数を持たず、paint の closure の中で gap bin を読み飛ばしている。

## 目的

- `GpuiObserver` に、mount 済み view の component ごとの描画 plan を返す driver-neutral な取得 API を追加する。
- consumer test から `parity/new-kinds.json` の `drawnBars` / `drawnMarks` を観測し、fixture の期待値と一致することを確認する。
- 公開 API に `gpui` 型を現れさせない。

## 要件

- R1: `GpuiObserver::drawn(window)` は window の view の描画 plan を返す。view が無い window では `None`。
- R2: 描画 plan は component ごとに、paint 順（tree 順）で返す。描画 plan を持つのは Meter・TimeAxis・Waveform のみとする。
- R3: Meter は bar（channel / x0 / x1 / peak / rms）を返す。peak・rms は `[0, 1]` に clamp 済みの値。
- R4: TimeAxis は mark（x / major / label）を返す。viewport 外の tick は含めない。
- R5: Waveform は channel ごとに、実際に描画される bin の frame 範囲（`start..end`）を返す。gap bin（`count == 0` または min / max が非有限）は含めない。
- R6: Tabs は描画される active tab の子だけを対象にする（paint と同じ）。
- R7: DTO は `ruviform-ui-gpui` の `observe` モジュールに置き、`#[non_exhaustive]` とし、`gpui` 型・GPUI 由来の型を含まない（数値は `f64`、`ComponentKey` / `WindowId` / `std::ops::Range<u64>` のみ）。
- R8: 既存の描画経路（paint の見た目・順序・計算）と、`settle_for_test` など既存の内部 hook の挙動を変えない。描画 plan の計算は paint と同じ関数を使い、plan と paint の二重実装を作らない。

## 受入条件

| ID | 条件 | 検証 |
| --- | --- | --- |
| AC-1 | `parity/new-kinds.json` の `expect.meter.drawnBars` と consumer 側の観測が一致する（件数・順序・各値、許容誤差 `1e-3`） | `observes_drawn_meter_and_axis_plans_through_seam` |
| AC-2 | `expect.timeAxis.drawnMarks` と consumer 側の観測が一致する（件数・x・major・label） | 同上 |
| AC-3 | Waveform の描画 bin 範囲（gap を除く）を consumer 側で観測できる | `observes_drawn_waveform_bins_through_seam` |
| AC-4 | 未知 window では `drawn` が `None` を返す | `unknown_window_has_no_view_to_observe_or_drive`（拡張） |
| AC-5 | 公開 signature に `gpui` 型が現れない | `mise run check-ui-driver-boundary` |
| AC-6 | Tabs の非 active 子は描画 plan に現れない | `render.rs` の unit test（gpui-test） |

## 範囲外

- Fake 層（`ruviform-ui-test`）側の drawn 検査の追加。
- AudioLens 側の test 実装（consumer 側の作業）。
- 実 OS window での描画確認（manual verification）。
- Waveform の parity fixture への追加（既存の fixture は Issue #421 の契約を維持する）。
- `ruviform-ui` の契約変更（`crates/ruviform-ui/**` は read-only）。

## 制約

- 非 RT（UI thread のみ）。
- feature は既存の `gpui-test` を用いる。新 feature は追加しない。
- MSRV 1.89、`workspace.lints` 準拠、public enum は `#[non_exhaustive]`。
- RF388 dock PoC と `crates/ruviform-ui-gpui/src/lib.rs` の export 面で競合しうるため、export の変更は最小にする。
