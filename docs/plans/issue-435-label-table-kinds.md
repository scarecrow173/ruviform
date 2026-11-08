---
id: "PLAN-Dz8GuKm3QpYv7RtWb4LoCs"
type: "plan"
status: "completed"
title: "Issue #435 — Label/Table ViewSpec kind の実装計画"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/435"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-435-label-table-kinds.md"
    - "docs/designs/issue-435-label-table-kinds.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  verified-by:
    - "docs/test-specs/issue-435-label-table-kinds.md"
---

# Issue #435 — 実装計画

## 1. ファイル対応表

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui/src/view.rs` | `LabelSpec` / `TableSpec` / `CellValue` / `A11yNode`、`ComponentSpec` の 2 variant、`A11yRole` の 5 variant、`check_node` の Table 検証、`ViewHost::a11y_node`、unit test 4 件 |
| `crates/ruviform-ui-gpui/src/render.rs` | `label_element` / `table_element` / `table_plan`、`element_for` の arm、gpui-test 3 件・plan unit 1 件 |
| `crates/ruviform-ui-test/fixtures/driver-lab/parity/label-table.json` | 新規。ViewSpec と expect（drawn・a11yTree） |
| `crates/ruviform-ui-test/tests/driver_parity.rs` | Label/Table の fake 側テスト 3 件を追記 |
| `crates/ruviform-ui-test/fixtures/driver-lab/cases.json` | UI-DVR-02 に Label/Table の fake・gpui binding を追加 |
| `docs/{specs,designs,plans,tasks,test-specs}/issue-435-*.md` | 文書 chain |

変更しない: `crates/ruviform-ui-theme/**`、`crates/ruviform-plugin-*/**`、`crates/ruviform-ui-webview/**`、`mise.toml`（既存タスクの対象に変更なし）、`Cargo.toml` / `Cargo.lock`（依存追加なし）。

## 2. 順序付き手順

1. #435 本文・既存 `ComponentSpec`/`A11ySpec`・#421 parity fixture・GPUI の plan 経路を読み、gap を整理する。完了。
2. 契約を追加する（型・variant・検証・a11y 木）。contract unit test を通す。完了。
3. Fake 側の `ViewHost::a11y_node` と parity fixture `label-table.json` を作り、`driver_parity.rs` で fixture と一致させる。完了。
4. GPUI の `table_plan` と element を追加し、gpui-test で fixture 一致と drawn grid = a11y grid を検査する。完了。
5. `cases.json` の binding を、実行結果に合わせて追加する。完了。
6. `cargo fmt`（`mise run fmt`）と検証タスク（§5）を実行する。完了（merge 前 HEAD）。
7. 文書 chain（spec・design・plan・task・test-spec）を作る。本ドキュメント群。
8. commit、branch push、PR 作成（日本語）。
9. merge 後 HEAD で検証マトリクスを再実行し、結果を PR 本文へ記録する。
10. review-protocol（独立サブエージェントによる diff レビュー）を実施し、blocker を解消する。

## 3. 依存関係

- 手順 2 は手順 1 に依存する。
- 手順 3・4 は手順 2 の型に依存する。手順 4 の gpui-test は手順 3 の fixture に依存する。
- 手順 5 は手順 3・4 の実行結果に依存する（先に status を書かない）。
- 手順 9・10 は全手順に依存する。

## 4. リスク登録

| リスク | 影響 | 対策 |
| --- | --- | --- |
| GPUI の platform a11y API が 0.2.2 で素直に使えない | A4 の a11y 一致が「木の一致」止まりになる | 非スコープとして明示。drawn grid と a11y 木の一致を検査し、OS 接続は別 Gate に分離 |
| 他 PR（RF388 dock PoC）が `lib.rs`/`render.rs` を変更し競合する | merge 時の衝突 | 変更は `render.rs` 末尾付近の追加に限定。merge 後 HEAD で再検証する |
| `#[non_exhaustive]` enum への variant 追加が consumer の exhaustive match を壊す | 下流ビルド失敗 | `A11yRole`・`ComponentSpec` は既に non_exhaustive。下流は `_` arm が必要な前提で、AudioLens 側の変更は別 PR で扱う |
| 数値の表示が renderer 間で揃わない | parity の drawn 比較が崩れる | `CellValue::Display` を唯一の表現にし、両 renderer が同じ関数を使う |
| 行数・列数の上限が実用に小さい | 大きい表が mount できない | 上限は定数で公開。超過時は明示的な `InvalidSpec`。拡大は別 Issue |

## 5. 検証マトリクス

| 層 | コマンド | 目的 |
| --- | --- | --- |
| 整形 | `mise run fmt-check` | rustfmt 準拠 |
| 型 | `mise run check` | workspace 全 target の型検査（exhaustive match の漏れ検出） |
| lint | `mise run lint` | `clippy --workspace --all-targets -- -D warnings` |
| lint（gpui） | `mise run lint-ui-gpui` | gpui feature の lint |
| 境界 | `mise run check-ui-driver-boundary` | GPUI 型の共通層への漏れがないこと |
| 契約 | `mise run test-ui-component-view` | contract unit と component fixture |
| parity（fake） | `mise run test-ui-parity` | Label/Table の fake 側 parity |
| driver lab | `mise run test-ui-driver-contract` | cases.json の manifest 整合（mechanism 存在） |
| GPUI | `mise run test-ui-gpui` | gpui-test（Label/Table の描画・plan） |
| 観測 | `mise run test-ui-consumer-observation` | 既存の観測 seam が影響を受けないこと |
| 全体 | `mise run test` | workspace の既存テストが変わらないこと |
| MSRV | `mise run test-ui-test-msrv` | ruviform-ui-test が MSRV 1.89 で通ること（merge 後 HEAD で実施） |

headless 系（`check-ui-headless` / `test-ui-headless`）は TypeScript の `ui/core` を対象とし、`ComponentSpec` を参照しないため本変更の影響範囲外。

## 6. ロールバック

- 変更は追加のみ（variant・型・関数・fixture・テスト）。既存 kind の挙動は変えない。
- 問題があれば PR を revert する。fixture とテストは対応する契約と同時に戻る。
- 永続データ形式の変更はない（serde の `type` 値の追加のみ）。
