---
id: "PLAN-Rt5LxV8dNq2MzKc7WbFy4"
type: "plan"
status: "proposed"
title: "Issue #436 — GpuiObserver 描画 plan 公開の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/436"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-436-observer-drawn.md"
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
  implements:
    - "docs/designs/issue-436-observer-drawn.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-436-observer-drawn.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-436-observer-drawn.md"
---

# Issue #436 — GpuiObserver 描画 plan 公開の実装計画

## 対象

設計 `docs/designs/issue-436-observer-drawn.md`（D1〜D6）を実装する。単一 PR・単一 crate の変更であり、分割計画は不要。

## ファイルマップ

| ファイル | 変更内容 | 対応設計 |
| --- | --- | --- |
| `crates/ruviform-ui-gpui/src/render.rs` | `waveform_bin_drawn` の抽出（paint と共有）、`DrawPlan` と `drawn_plans` の追加、`MeterBarPlan` / `AxisMarkPlan` の field を `pub(crate)` 化、unit test 追加 | D1, D3 |
| `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` | 公開 DTO（`ViewDrawn` 他）、`GpuiObserver::drawn`、`DrawPlan` → DTO の写像 | D2, D4 |
| `crates/ruviform-ui-gpui/src/lib.rs` | `observe` 再公開に DTO を追加（export 面の最小変更） | D5 |
| `crates/ruviform-ui-gpui/tests/consumer_observation.rs` | Meter/TimeAxis の fixture 一致、Waveform の bin 範囲、未知 window の `None` | AC-1〜AC-4 |
| `mise.toml` | `test-ui-consumer-observation` の description を drawn 観測に更新（task 名は不変） | D5 |
| `docs/**` | spec / design / plan / task / test-spec | — |

## 依存順序

1. `render.rs`: `waveform_bin_drawn` 抽出（挙動不変）。既存の `test-ui-gpui` が通ることを確認。
2. `render.rs`: `DrawPlan` / `drawn_plans` と unit test（Tabs・gap・clamp）。
3. `observe.rs`: DTO と `drawn`、`lib.rs` の export。
4. `consumer_observation.rs`: 観測テスト（fixture 一致・Waveform・未知 window）。
5. `mise.toml` の description 更新、`docs/development-environment.md` の該当行の確認。
6. 検証（下記）。

## リスク

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `lib.rs` の export 面が RF388 dock PoC と競合 | rebase 時の衝突 | export 行の追加のみ。競合時は main を取り込んで解消 |
| paint の Waveform 判定抽出で挙動が変わる | 描画回帰 | 述語の内容を同一に保つ。既存の render unit test を通す |
| Waveform の JSON で gap bin を `NaN` にできない | consumer test の gap 検証が書けない | gap は `count == 0`（有限の min / max）で表す。NaN の gap は render unit test で検証 |
| `drawn` の DTO が gpui 型を含む | boundary guard 失敗 | DTO は `f64` / `usize` / `String` / `ComponentKey` / `Range<u64>` のみ |

## ロールバック

変更は追加的であり、`drawn` と DTO を削除し `waveform_bin_drawn` を元の inline 判定へ戻せば完全に元へ戻る。永続データ形式・lockfile の変更はない。

## 検証マトリクス

| 段階 | コマンド | 目的 |
| --- | --- | --- |
| 1 | `mise run test-ui-gpui` | paint 抽出後も既存 render test が通る |
| 2 | `mise run test-ui-consumer-observation` | AC-1〜AC-4 |
| 3 | `mise run check-ui-driver-boundary` | AC-5 |
| 4 | `mise run lint-ui-gpui` / `mise run check-ui-gpui` / `mise run fmt-check` | 警告ゼロ・整形 |
| 5 | `mise run check` / `mise run lint` / `mise run test` | ワークスペース全体 |
| 6 | `mise run test-ui-parity` / `mise run test-ui-driver-contract` / `mise run test-ui-headless` / `mise run test-ui-component-view` | 周辺 UI lane の回帰 |
| 7 | `mise run test-ui-test-msrv` | MSRV 検証（実行可否は環境次第。未実行なら理由を明記） |
