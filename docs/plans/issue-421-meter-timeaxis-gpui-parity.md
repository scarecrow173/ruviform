---
id: "PLAN-Hs2MvCe8ZyWp5QnXb7TkRa"
type: "plan"
status: "in-progress"
title: "Issue #421 — Meter/TimeAxis GPUI 描画と parity の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/421"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
---

# Issue #421 — 実装計画

## 1. ファイル対応表

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/src/render.rs` | Meter/TimeAxis の element 描画、`meter_bar_plan` / `axis_mark_plan`、gpui-test 4 件 |
| `crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json` | 新規。ViewSpec と期待値 |
| `crates/ruviform-ui-test/tests/driver_parity.rs` | 新規。Fake 側 3 テスト |
| `crates/ruviform-ui-test/fixtures/driver-lab/cases.json` | UI-DVR-02 GPUI pass、UI-DVR-02/03 Fake parity binding 追加、UI-DVR-03 GPUI note 更新 |
| `mise.toml` | `test-ui-parity` 追加 |
| `docs/development-environment.md` | `test-ui-parity` の表行 |
| `docs/{specs,designs,plans,tasks,test-specs}/issue-421-*.md` | 文書 chain |

共通層（`crates/ruviform-ui`）、`ruviform-ui-theme`、plugin、DSP/RT は変更しない。

## 2. 順序付き手順

1. 既存 schema（`ComponentSpec::Meter` / `TimeAxis`）と GPUI の placeholder 経路を確認する。完了。
2. parity fixture を作る（fake 側の `ViewHost` 検証が通ることを先に確認）。完了。
3. Fake 側テスト `driver_parity.rs` を書き、Knob/Toggle の intent 期待値（atomic set は begin+commit）を契約どおりに確定する。完了。
4. GPUI の plan 純関数と失敗する gpui-test を追加し、element 描画（`meter_element` / `time_axis_element`）を実装する。完了。
5. manifest の status を実行結果に合わせる。完了。
6. mise タスクと開発環境ドキュメントを追加する。完了。
7. 文書 chain を作る。本ドキュメント群。
8. 最終検証（merge 後 HEAD）、review-protocol（独立レビュー）、PR 作成。

## 3. 依存関係

- 手順 2 は手順 1 に依存する。
- 手順 4 の gpui-test は手順 2 の fixture に依存する。
- 手順 5 の manifest 更新は手順 3・4 の実行結果に依存する（先に status を書かない）。
- 手順 8 は全手順に依存する。

## 4. リスク登録

| リスク | 影響 | 対策 |
| --- | --- | --- |
| GPUI 0.2.2 の `relative()` / `absolute()` の API 形状が想定と違う | TimeAxis のラベル配置がずれる | コンパイル・clippy で検出。`left(relative(x))` は gpui の `Styled` に従う |
| gpui-test の pointer 座標が layout に依存する | 入力テストが不安定 | 座標は prepaint で記録された knob bounds から導く。Toggle は先頭子で原点に置く |
| pedantic clippy（float_cmp、doc_markdown、cast 系） | lint ゲートで落ちる | 浮動小数は許容誤差で比較、cast は既存と同じ `allow` を局所付与 |
| manifest の status 過大申告 | driver-lab の意味が崩れる | pass は実行されたテストに限定。UI-DVR-03 GPUI は not_run のまま |

## 5. ロールバック

- 変更はすべて本ブランチの commit 群に閉じる。問題があれば該当 commit を revert する（履歴の書き換えはしない）。
- `render.rs` の element 変更は `element_for` の 2 arm と 2 メソッドのみで、revert すれば従来の placeholder に戻る。

## 6. 検証マトリクス

| 手順 | 検証 | コマンド |
| --- | --- | --- |
| 3 | Fake parity | `mise run test-ui-parity` |
| 4 | GPUI 描画・parity・入力 | `mise run test-ui-gpui` |
| 4 | Lint（pedantic・-D warnings） | `mise run lint-ui-gpui` |
| 4 | 型検査 | `mise run check-ui-gpui` |
| 5 | manifest 整合・mechanism 実在 | `mise run test-ui-driver-contract` |
| 全体 | 境界（GPUI 型の共通層露出なし） | `mise run check-ui-driver-boundary` |
| 全体 | 整形 | `mise run fmt-check` |
| 全体 | 全体 lint・型・テスト | `mise run lint` / `mise run check` / `mise run test` |
| 全体 | MSRV 1.89 test-target | `mise run test-ui-test-msrv` |
