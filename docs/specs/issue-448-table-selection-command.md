---
id: "SPEC-Vd7Qk2LmRt9WxYc4NbHa3F"
type: "spec"
status: "approved"
title: "Issue #448 — 読み取り専用 Table の行選択と command 起動の ViewSpec 契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/448"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  refines:
    - "docs/specs/issue-435-label-table-kinds.md"
  depends-on:
    - "docs/specs/issue-435-label-table-kinds.md"
    - "docs/specs/issue-387-component-view-specs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-435-label-table-kinds.md"
---

# Issue #448 — 読み取り専用 Table の行選択と command 起動の ViewSpec 契約

## 背景

AudioLens の Jobs / Run / Dataset 画面は Table で一覧を表示するが、行を選べず、Run を起動する操作も表現できない（scarecrow173/audio-lens#166）。現行の `TableSpec`（#435）は読み取り専用で、行選択の intent も選択状態も持たない。現行 `ComponentSpec` の 13 kind のうち操作系は Fader / Seek / Knob / Toggle だけで、いずれも連続値または二値の入力である。「一覧から 1 項目を選ぶ」「値を伴わない命令を起動する」という離散 action は表現できず、consumer が独自規約で詰めると契約の迂回になる。

`CommandIntent::Action`（discrete action、receipt 期待）は #257 以前から intent 語彙に存在するが、どの入力経路からも発行されていない。本 Issue はこの語彙を入力経路と component 契約に接続する。

## 目的

- `Table` に行選択の契約を追加する。選択は `ViewEvent::Intent` として発行し、選択状態を `ComponentSnapshot` で観測できるようにする。
- 値を伴わない操作の component `Button` を追加する。intent は target・action・params を運び、accepted / applied の区別は既存の receipt 契約（`CommandBinding`）に従う。
- いずれも driver-neutral な headless 契約として定義する。同一 spec に対し Fake（`ViewHost`）と GPUI の intent・snapshot が一致することを parity fixture で検査する。

## 受入条件（Issue #448 本文に対応）

| # | Issue #448 受入 | 本 PR での扱い |
| --- | --- | --- |
| A1 | Table（または List）の行を選択可能にする契約。選択 intent を `ViewEvent` として発行し、選択状態を component snapshot で観測できる | 達成。`TableSpec.selection`（任意）を追加し、`ViewHost::select_row` が `ViewEvent::Intent`（`action: "select"`、`params: {row}`）を発行する。`ComponentSnapshot.selected` で観測できる |
| A2 | 値を伴わない操作の component と、その intent（target・params）の契約。accepted / applied は既存 receipt 契約に従う | 達成。`ComponentSpec::Button` と `ViewHost::invoke` を追加し、`CommandIntent::Action`（`target`・`action`・`params`）を発行する。phase は `CommandBinding` の既存遷移に従う |
| A3 | driver-neutral な headless 契約として定義し、GPUI と webview の両 driver で同じ intent を発行する | 部分達成。契約と GPUI は達成。webview は `ruviform-ui-webview` に `ViewSpec` の描画経路が存在しないため未対応（NotRun/Unsupported として記録。下記非スコープ） |
| V1 | headless: 行選択・command 起動の intent が `ViewHost` から発行され、snapshot に選択状態が反映される | 達成。`driver_parity.rs` と `view::tests` で検査 |
| V2 | GPUI parity: 同一 spec で Fake 層と GPUI 層の intent・snapshot が一致する | 達成。`selection-command.json` を Fake と GPUI が同一に検査する |

## スコープ（本 PR）

- `ruviform-ui`: `TableSelectionSpec`、`ButtonSpec`、`ComponentSpec::Button`、`A11yRole::Button`、`ViewHost::select_row` / `ViewHost::invoke`、`ComponentSnapshot.selected`、`ViewError::InvalidInput`、`CommandIntent::Action` の `params` 追加。
- `ruviform-ui-gpui`: Button の描画と押下、Table 行の押下による選択、選択行の強調、`RvfView` の入力経路、`GpuiObserver::invoke` / `select_row`（テスト用 seam）。
- `ruviform-ui-test`: parity fixture `driver-lab/parity/selection-command.json`、`driver_parity.rs` の追加テスト。
- `ruviform-ui-gpui/tests/consumer_observation.rs`: GPUI 側の seam テスト。
- 文書 chain（spec → design → plan → task → test-spec）と docs index の再生成。

## 非スコープ（未達として明示）

- webview driver での `ViewSpec` 描画と intent 発行。`ruviform-ui-webview` は `ViewSpec` / `ViewHost` を参照しない（wry ホストと `rvf://` 配信のみ）。webview の component 描画は別 Issue とし、本 PR では webview parity を NotRun として扱う。
- 行の同一性（row identity）。選択は spec 上の行 index で表し、consumer は自身のデータと突き合わせる。行 key・複数選択・キーボード操作は含めない（YAGNI）。
- 選択の永続化、ソート、列幅・配置、仮想化。
- platform a11y API への接続（#435 の非スコープを継承）。
- AudioLens 側の ViewSpec 化と Run 起動の接続（consumer 側作業）。

## 不変条件

- 共通層（`ruviform-ui`）に GPUI 型を出さない。`check-ui-driver-boundary` が pass すること。
- 公開 enum は `#[non_exhaustive]` を維持する（`A11yRole`・`ViewError`・`ComponentSpec`）。新規 struct の追加フィールドは `serde(default)` で既存 JSON と後方互換にする。
- 選択は表の行を指す。範囲外の行は `ViewError::InvalidInput` で拒否し、intent を発行しない。
- 選択表示は楽観的に更新するが、receipt が `rejected` / `unsupported` で終端したら直前の選択に戻す。accepted / applied は表示を変えず、phase で進捗を示す。
- Button と選択可能 Table の binding id は component key とする（control の `control_id` とは独立）。
- 既存 kind（Fader / Seek / Knob / Toggle / Label / Table の読み取り挙動）の payload・intent・snapshot は変更しない。
