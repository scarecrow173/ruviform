---
id: "PLAN-Qz8Ld3WbTm6XyKr2HnVc4E"
type: "plan"
status: "approved"
title: "Issue #448 — 行選択と command 起動の実装計画"
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
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-448-table-selection-command.md"
  implemented-by:
    - "docs/tasks/issue-448-table-selection-command.md"
  verified-by:
    - "docs/test-specs/issue-448-table-selection-command.md"
  depends-on:
    - "docs/specs/issue-448-table-selection-command.md"
    - "docs/designs/issue-448-table-selection-command.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-435-label-table-kinds.md"
---

# Issue #448 — 実装計画

## 1. ファイル対応表

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui/src/view.rs` | `TableSelectionSpec` / `TableSpec.selection` / `TABLE_SELECT_ACTION`、`ButtonSpec` / `ComponentSpec::Button`、`A11yRole::Button`、`ComponentSnapshot.selected`、`ViewError::InvalidInput`、`ViewHost::{select_row, invoke}`、receipt 連動の選択 revert、`check_command`、unit test 8 件 |
| `crates/ruviform-ui/src/behavior.rs` | `CommandIntent::Action.params`（`serde(default, skip_serializing_if)`） |
| `crates/ruviform-ui-gpui/src/render.rs` | `button_element`、`table_element` の行押下と選択強調、`RvfView::{select_row, invoke}`、`DrawPlan::Button`、`collect_drawn` の arm |
| `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` | `DrawnPlan::Button`、`GpuiObserver::{select_row, invoke}` |
| `crates/ruviform-ui-gpui/tests/consumer_observation.rs` | 選択と Button の intent・snapshot・drawn の seam テスト 1 件 |
| `crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json` | 新規。ViewSpec と expect（intents・phase・a11yTree・drawn） |
| `crates/ruviform-ui-test/tests/driver_parity.rs` | fake 側テスト 2 件（mount・intents と snapshot） |
| `crates/ruviform-ui-test/fixtures/driver-lab/cases.json` | UI-DVR-02 に fake・gpui、UI-DVR-03 に gpui の binding を追加 |
| `docs/{specs,designs,plans,tasks,test-specs}/issue-448-*.md` | 文書 chain |

`Cargo.toml` / `Cargo.lock` は変更しない（新規依存なし）。

## 2. 順序付き手順

1. 設計（契約の型・intent 語彙・receipt 連動の規則）を spec / design に記録する。
2. `ruviform-ui` の契約を実装する。`Action.params` を追加し、Button と選択可能 Table、`select_row` / `invoke`、receipt 連動の revert を入れる。unit test を先に書いて失敗を確認してから実装する。
3. `ruviform-ui` の unit test と `mise run test-ui-component-view` を通す。
4. parity fixture `selection-command.json` を作り、Fake 側テストを追加する（`test-ui-parity`）。
5. GPUI 側を実装する（Button 描画・押下、Table 行押下・強調、`DrawPlan::Button`）。`test-ui-gpui` の既存テストを壊さないことを確認する。
6. GPUI の seam テストを追加し、`test-ui-consumer-observation` を通す。
7. `cases.json` の binding を追加し、`test-ui-driver-contract` で manifest 整合を確認する。
8. `fmt` / `lint` / `lint-ui-gpui` / `check-ui-driver-boundary` を通す。
9. 文書 chain（test-spec・task）を完成させ、docs index を再生成する。
10. `origin/main` を取り込み、最新 HEAD で §4 の全件を再実行する。

## 3. 依存関係

- 手順 2 は手順 1 の契約（型名・action 名・receipt 規則）に依存する。
- 手順 4・5 は手順 2 の公開 API に依存する。手順 4 と 5 は独立に進められる。
- 手順 6 は手順 5 の seam に依存する。
- 手順 10 は RF#450（`render.rs` の Seek/Fader canvas）との競合を含むため、最後に行う。

## 4. 検証マトリクス

| 層 | コマンド | 目的 |
| --- | --- | --- |
| 整形 | `mise run fmt-check` | rustfmt 準拠 |
| lint | `mise run lint` | `clippy --workspace --all-targets -- -D warnings` |
| lint（gpui） | `mise run lint-ui-gpui` | gpui feature の lint |
| lint（gpui-test） | `mise exec -- cargo clippy -p ruviform-ui-gpui --all-targets --features gpui-test -- -D warnings`（補助証拠。mise に対応タスクはない） | gpui-test 対象の lint |
| 境界 | `mise run check-ui-driver-boundary` | 共通層に GPUI 型が漏れていないこと |
| 契約 | `mise run test-ui-component-view` | contract（既存の component fixture を含む） |
| parity（fake） | `mise run test-ui-parity` | selection-command の fake 側 |
| driver lab | `mise run test-ui-driver-contract` | cases.json の manifest 整合 |
| GPUI | `mise run test-ui-gpui` | gpui-test（描画・既存 plan） |
| 観測 | `mise run test-ui-consumer-observation` | seam の intent・snapshot・drawn |
| 全体 | `mise run test` | workspace の既存テストが変わらないこと |
| webview | 対応なし | `ruviform-ui-webview` に ViewSpec 描画経路がないため、NotRun として記録 |

## 5. リスク登録

| リスク | 影響 | 対策 |
| --- | --- | --- |
| 楽観的な選択表示が consumer の拒否と食い違う | 表示と domain state がずれる | Rejected/Unsupported で直前の選択へ戻す。Accepted/Applied は phase で進捗を示す |
| 行 index が spec 更新で別の行を指す | 誤った行を consumer が処理する | 文書化（非スコープの row identity として明示）。consumer は自身のデータと突き合わせる |
| `CommandIntent::Action` への `params` 追加が既存 consumer を壊す | 下流ビルド失敗 | Action はこれまでどこからも発行されていない。`skip_serializing_if` で既存 JSON を変えない |
| RF#450 が `render.rs` を同時変更し競合する | merge 時の衝突 | 変更は table/label 周辺・`element_for`・`collect_drawn` に限定する。Seek/Fader canvas には触れない |
| `ruviform-ui-webview` に描画経路がなく、webview parity が書けない | A3 の webview 部分が未達 | 契約を driver-neutral に定義し、webview は NotRun として PR 本文と cases に明記する |
| undo 記録が receipt の来ない command で溜まる | メモリが増える | 削除・detach・unmount で解放。consumer は Action に terminal receipt を返す前提（expect_receipt） |

## 6. ロールバック

- 契約追加はすべて additive（新 kind・新 variant・serde default）。`git revert` で本 PR の commit 群を戻せば既存の挙動に戻る。
- cases.json の binding を戻すと driver-contract は従来どおり pass する。
