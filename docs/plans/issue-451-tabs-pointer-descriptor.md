---
id: "PLAN-Wp3Nd7TzKb5QxHv2RcLf9J"
type: "plan"
status: "approved"
title: "Issue #451 — Tabs pointer 切替と descriptor 更新の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-451-tabs-pointer-descriptor.md"
      - type: "file"
        path: "docs/test-specs/issue-451-tabs-pointer-descriptor.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/component_showcase.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-451-tabs-pointer-descriptor.md"
  implemented-by: []
  refines:
    - "docs/plans/issue-448-table-selection-command.md"
  depends-on:
    - "docs/specs/issue-451-tabs-pointer-descriptor.md"
    - "docs/designs/issue-451-tabs-pointer-descriptor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-448-table-selection-command.md"
---

# Issue #451 — Tabs pointer 切替と descriptor 更新の実装計画

## 1. 目的と範囲

設計 `DESIGN-Hk9Qz4WsNt6XrLc1VbYe8M` の決定を、実装スライスに分けて進める。範囲は spec の受入条件 A1（pointer 部分）・A2・V1〜V3 と、発見した receipt 欠落の修正。キーボード操作と webview は含めない。

## 2. スライス

| # | スライス | 内容 | 完了条件 |
| --- | --- | --- | --- |
| S1 | descriptor 更新で receipt routing を保つ | `unregister_control` を `detach_control`（gesture のみ）と `release` 用に分離し、contract change は `detach_control` を使う。失敗するテストを先に置く | `descriptor_update_keeps_in_flight_receipt_routing` が pass。`descriptor_update_widens_seek_range_without_remount` が pass |
| S2 | Tabs の選択モデル | `TabsSpec.selection`、`TabsSelectionSpec`、`TAB_SELECT_ACTION`、`ViewHost::select_tab`、`emit_selection` の共有化、`Mounted.selected` の初期化、`carried_selection`、`clamp_selection` の Tabs 対応、`action_binding_id` の Tabs 対応、`check_command` の target 検査 | `view::tests::tab_*` と `rejected_tab_selection_restores_the_previous_tab` が pass。既存の Table テストが pass |
| S3 | GPUI の表示と pointer | タブヘッダの `on_mouse_down`、`RvfView::select_tab`、`displayed_tab`、`drawn_plans(host)`、`debug_selector`、`GpuiObserver::select_tab` | `tab_header_press_selects_the_tab_through_the_pointer` と `consumer_observation` の tab 検査が pass |
| S4 | 共有 fixture と Fake 側 | `selection-command.json` へのタブ追加、`driver_parity` の tab 検査、既存 struct literal の `selection: None` 追従 | `test-ui-parity` と `test-ui-component-view` が pass |
| S5 | 手動 GPUI 実窓の証拠 | `component_showcase` のタブを選択可能にし、OS 実ポインタでの切替を確認 | スクリーンショットで `Ch 1 → Ch 2 → Ch 1` を確認（PR に記載） |
| S6 | 文書 chain と index | spec・design・plan・task・test-spec と各 `README.md` 索引の追加 | `doc-status` 相当の整合（front matter・索引）が取れている |
| S7 | 検証と PR | `fmt-check`・`lint`・`lint-ui-gpui`・`test`・UI タスク群、main 取り込み、PR 作成 | 下記の検証マトリクスが全て pass または NotRun と明記されている |

スライスは S1 → S2 → S3 → S4 の順に依存する。S5 は S3 の後。S6・S7 は最後。

## 3. ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui/src/view.rs` | `TabsSpec.selection`、`TabsSelectionSpec`、`TAB_SELECT_ACTION`、`select_tab`、`emit_selection`、`initial_selection`、`carried_selection`、`clamp_selection`、`detach_control`、テスト |
| `crates/ruviform-ui-gpui/src/render.rs` | タブヘッダの入力、`select_tab`、`displayed_tab`、`drawn_plans(host)`、`debug_selector`、テスト |
| `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` | `select_tab` の seam、`drawn` の引数変更 |
| `crates/ruviform-ui-gpui/examples/component_showcase.rs` | タブを選択可能にする（手動検証用） |
| `crates/ruviform-ui-gpui/tests/consumer_observation.rs` | tab の intent・snapshot・drawn の検査 |
| `crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json` | Tabs と `expect.tabs` の追加 |
| `crates/ruviform-ui-test/tests/driver_parity.rs` | tab の intent・snapshot の検査 |
| `crates/ruviform-ui-test/tests/component_view.rs` | `TabsSpec` literal の追従 |
| `crates/ruviform-poc-native-dock/src/poc/dock.rs` | `TabsSpec` literal の追従 |
| `docs/**` | 文書 chain と index |

## 4. 依存と順序の注意

- `TabsSpec` に追加するフィールドは、workspace 内の全 struct literal を同時に更新しないとビルドが通らない。S2 と S4 は同じコミット群で扱う。
- `drawn_plans` の引数変更は `gpui-test` feature の crate 内だけで閉じる。consumer 側に影響しない。
- `Cargo.toml` と `Cargo.lock` は変更しない（依存追加なし）。

## 5. リスク登録簿

| リスク | 影響 | 緩和 |
| --- | --- | --- |
| descriptor 更新での receipt 維持が、removal の stale receipt 保護を弱める | 古い receipt が新しい epoch に届く | `update_reregister_keeps_binding_epoch` と `remount_absorbs_stale_receipts` を既存のまま維持して pass させる |
| 表示タブを `ComponentSnapshot.selected` に寄せると、非選択の Tabs が `selected` を持つ | 既存の Table 向けの観測が変わる | Table の挙動は変えず、Tabs は常に `Some(active)` を返すと doc に明記 |
| GPUI の `debug_selector` が release にも残る | 余計なコード | gpui 側で no-op。release の挙動は変わらない |
| 実窓証拠が手動で再現性がない | 検証の再現が難しい | 自動化された pointer テスト（TestAppContext）で同じ経路を検査し、実窓は補助証拠と明記する |
| Windows のフォーカス制約で実窓クリックが失敗する | 証拠が取れない | OS 実ポインタ（`mouse_event`）でクリックした。フォーカス API には頼らない |

## 6. ロールバック

S1〜S4 は独立に revert できるように、コミットを論理単位で分ける。`TabsSpec.selection` を追加しているため、S2 を単独で revert すると S4 の fixture も壊れる。その場合は S2 と S4 を一緒に revert する。文書（S6）は最後に積み、単独で revert 可能。

## 7. 検証マトリクス

| 検証 | 種類 | 対象 | 結果（PR に記載） |
| --- | --- | --- | --- |
| `mise run fmt-check` | 静的 | 全体 | 実行して記録 |
| `mise run lint` | 静的 | workspace（clippy `-D warnings`） | 実行して記録 |
| `mise run lint-ui-gpui` | 静的 | GPUI adapter（feature `gpui`） | 実行して記録 |
| `mise run test` | 単体・結合 | workspace | 実行して記録 |
| `mise run test-ui-component-view` | contract | Fake（`ViewHost`） | 実行して記録 |
| `mise run test-ui-parity` | parity | Fake 側の共有 fixture | 実行して記録 |
| `mise run test-ui-consumer-observation` | parity | GPUI 側の共有 fixture（seam） | 実行して記録 |
| `mise run test-ui-gpui` | GPUI 単体 | pointer テストと drawn | 実行して記録 |
| `mise run test-ui-driver-contract` | manifest | driver-lab の整合 | 実行して記録 |
| `mise run check-ui-driver-boundary` | 境界 | `ruviform-ui` に GPUI 型が出ていないこと | 実行して記録 |
| `mise run check-ui-gpui` | 型検査 | GPUI adapter | 実行して記録 |
| 手動: `component_showcase` の実窓 | 実機 | タブのクリック切替 | スクリーンショットで記録（NotRun は理由を記載） |
| webview parity | — | — | NotRun（`ViewSpec` の描画経路がない） |
| キーボードによるタブ切替 | — | — | NotRun（未実装。follow-up） |

## 8. 完了の定義

- 検証マトリクスの自動検証がすべて pass する。
- 実窓の結果と NotRun 項目が PR 本文に区別して書かれている。
- 文書 chain が spec → design → plan → task → test-spec の順で揃い、索引に載っている。
