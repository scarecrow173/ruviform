---
id: "DESIGN-Kt4NzBw9QxLr6VfGc1PuDo"
type: "design"
status: "proposed"
title: "Issue #421 — Meter/TimeAxis GPUI 描画と parity fixture の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/421"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #421 — Meter/TimeAxis GPUI 描画と parity fixture の設計

## 概要

Meter/TimeAxis は共通 schema（`ruviform-ui::view`）で既に定義済みで、`ViewHost` の検証も通る。欠けていたのは GPUI 側の描画と、両 driver が同じ期待値で検査される仕組みである。本設計は次の 3 点を決める。

1. GPUI の描画規則を、描画副作用から切り離した純関数（plan）として持つ。
2. parity の期待値を 1 つの JSON fixture に置き、Fake と GPUI の両テストが読む。
3. Consumer 観測 seam の公開範囲を決める。

## D1: 描画 plan を純関数に分離する

`ruviform-ui-gpui/src/render.rs` に次の 2 つの純関数を置く。

- `meter_bar_plan(&MeterSpec) -> Vec<MeterBarPlan>`
- `axis_mark_plan(&TimeAxisSpec) -> Vec<AxisMarkPlan>`

canvas の paint closure は plan を消費するだけにする。描画規則は plan に集約し、テストは plan を直接検査する。

描画規則（renderer が持つ policy。共通層には入れない）:

| 規則 | 内容 |
| --- | --- |
| Meter gap | `count == 0` または非有限 peak の bin は描画しない |
| Meter RMS | RMS が非有限なら peak バーのみ描画する |
| Meter 振幅 | peak/RMS は amplitude として扱い、`[0, 1]` にクランプして帯の高さに写す（contract は dB/linear を規定しないため、over-range は帯内にクリップする） |
| Meter 区間 | 空区間（`end <= start`）・逆順区間・viewport と完全に重ならない bin は描画しない |
| TimeAxis cull | viewport 外（x < 0 または x > 1）の tick は描画しない |
| TimeAxis 強弱 | major は全高、minor は半高 |

viewport 外の bin/tick は payload に保持し、描画だけ省く。headless 側は payload を保持するため、この分担は Fake の payload 保持検査と GPUI の plan 検査で両側から確認できる。

## D2: 共通層は変更しない

Meter/TimeAxis の schema は #387 のまま変更しない。`ruviform-ui` に新しい描画 API を足さない。理由は次のとおり。

- 描画 policy（gap の扱い、クランプ、cull）は driver 固有の表示規則であり、headless 契約に入れると GPUI 型を持たない共通層に描画 policy が漏れる。
- 共通層の変更は Consumer（AudioLens/Synvelis）の既存 ViewSpec 構築に影響しうる。本 Issue の範囲では必要がない。

## D3: parity fixture は 1 つの JSON を両 driver が読む

`crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json` は次の 2 部から成る。

- `view`: `ComponentSpec` の serde 形式の ViewSpec（Toggle・Knob・Meter・TimeAxis を含む Panel）。
- `expect`: 両 driver が検査する期待値（mount されるキー、Knob/Toggle の intent 列、Toggle の committed 値、Meter の宣言 bin 数と drawn bar、TimeAxis の宣言 tick 数と drawn mark）。

検査の割当て:

- Fake（`tests/driver_parity.rs`）: mount と reconcile（stable key）、gesture intent 列、committed 値、宣言 payload の保持（gap bin と viewport 外 tick を含む）。
- GPUI（`render.rs` の gpui-test）: `meter_bar_plan` / `axis_mark_plan` が `expect` の drawn 値と一致、実 pointer 入力による intent 列、RvfView 上での reconcile。

NaN は JSON で表現できないため、fixture では gap を `count == 0` の有限値で表す。NaN を gap として扱う規則は `meter_and_axis_plans_skip_gaps_and_clamp`（Rust 単体テスト）で検査する。

Toggle の atomic set は契約どおり `begin` と `commit` の 2 intent を発行する（`Begin` の atomic flag）。fixture はこの契約を期待値として書く。

## D4: manifest の status は実行結果だけで決める

- `UI-DVR-02` GPUI は `rvf_view_parity_new_kinds_mount_reconcile`（gpui-test）の実行結果で pass とする。
- `UI-DVR-02` / `UI-DVR-03` の Fake は `driver_parity` の 2 テストで pass とする（cargo-test mechanism はファイル・フィルタの実在を検査する）。
- `UI-DVR-03` GPUI は Cancel と遅延・順序逆転 receipt が未接続のため `not_run` のまま、note を実態に合わせる。
- `stories.json` は変更しない（2 scale factor と状態遷移の GPUI 実行が未実施のため、story の status を pass にできない）。

## D5: Consumer 観測 seam は本 PR で公開しない

AudioLens PR #169 は、GPUI の projection 状態と emit された intent を consumer test から観測する seam を求めている。現状の `settle_for_test` / `inbox_len_for_test` は `#[cfg(all(test, feature = "gpui-test"))]` の `pub(crate)` であり、`gpui::TestAppContext` を引数に取る。

本 PR で公開しない理由:

- consumer 向けの seam は、gpui 型を含まない観測 DTO（projection snapshot と intent 列）として設計する必要がある。`TestAppContext` をそのまま公開すると、consumer に gpui の test-support 依存を強制する。
- 公開 feature（例: `test-support`）の追加は、Cargo.toml と boundary guard の双方に影響する設計判断であり、Issue #421 の範囲（描画と parity）を超える。
- boundary guard を通す公開面は設計できるが、本 PR で急いで決めると後戻りの高いAPIになる。

結論: follow-up Issue として観測 DTO と feature 境界を設計する。AudioLens 側は GPUI parity を NotRun のまま維持する。

## 影響範囲

- 公開 API: 追加なし（`render.rs` は `ruviform-ui-gpui` の crate-private module）。
- 永続データ形式: なし。
- Consumer: Meter/TimeAxis を含む ViewSpec が GPUI で描画されるようになる（従来は placeholder）。
- RT: 対象外（非 RT の描画のみ）。

## 検証

| 観点 | 検証 | 実行方法 |
| --- | --- | --- |
| 描画規則 | `meter_and_axis_plans_skip_gaps_and_clamp` | `mise run test-ui-gpui` |
| parity（描画 plan） | `parity_new_kinds_plans_match_shared_fixture` | `mise run test-ui-gpui` |
| parity（入力・reconcile、GPUI） | `rvf_view_parity_new_kinds_gesture_and_render` / `rvf_view_parity_new_kinds_mount_reconcile` | `mise run test-ui-gpui` |
| parity（Fake） | `tests/driver_parity.rs` の 3 テスト | `mise run test-ui-parity` |
| manifest 整合 | `manifests_are_consistent`（mechanism 実在を含む） | `mise run test-ui-driver-contract` |
| 境界 | boundary guard | `mise run check-ui-driver-boundary` |
