---
id: "SPEC-Gt5Rw2KqZm8LxNv3TcYb7H"
type: "spec"
status: "approved"
title: "Issue #451 — Tabs ヘッダの pointer 切替と ViewSpec descriptor の mount 後更新契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-451-tabs-pointer-descriptor.md"
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
  implements: []
  implemented-by: []
  refines:
    - "docs/specs/issue-448-table-selection-command.md"
  depends-on:
    - "docs/specs/issue-448-table-selection-command.md"
    - "docs/specs/issue-387-component-view-specs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-448-table-selection-command.md"
---

# Issue #451 — Tabs ヘッダの pointer 切替と ViewSpec descriptor の mount 後更新契約

## 背景

Synvelis PR#183 の検証で、ruviform の ViewSpec 契約に次の 2 つの gap が見つかった。

1. **Tabs の pointer 切替がない。** `ComponentSpec::Tabs` の active タブは spec の `active` だけで決まり、タブヘッダにはマウス入力の経路がない。Mixer タブをクリックしても切り替わらない。
2. **Seek の range を mount 後に変えられる契約が見えない。** Synvelis の `transport_seek` descriptor は `ValueDescriptor::ranged(0.0, 240.0)` で固定され、project length の変化に追従できない。

#448 で Table の行選択と Button の command 起動を、既存の `CommandIntent::Action` 語彙に載せた。本 Issue も同じ語彙と receipt 契約に載せる。

## 目的

- Tabs ヘッダの pointer 操作によるタブ切替を、driver-neutral な intent（`tab-select`）として契約化する。Fake（`ViewHost`）と GPUI の両方で同じ intent と snapshot を得られること。
- mount 後に descriptor（Seek の range を含む）を更新する経路を契約として明示する。既存の `ViewHost::mount`・receipt・generation・`AttachEpoch` の整合を壊さないこと。

## 受入条件（Issue #451 本文に対応）

| # | Issue #451 受入 | 本 PR での扱い |
| --- | --- | --- |
| A1 | Tabs ヘッダのクリック/キーボードでのタブ切替が driver-neutral intent（`tab-select` 相当）として契約化され、Fake/GPUI 両 driver で動作する | 部分達成。pointer（クリック）は契約・Fake・GPUI で達成。キーボード操作は未実装（GPUI に key 入力経路がない。下記非スコープ） |
| A2 | Seek descriptor の mount 後更新 API: project length 変化時に Seek range を更新する driver-neutral な契約（descriptor 更新 or remount 設計の明示） | 達成。決定: descriptor 更新は `ViewHost::update`（mount 済み host への `mount` も同じ経路）で行い、remount は不要とする。Issue 本文の受入に対応する contract test を追加 |
| V1 | FakeDriver で tab 切替 intent が観測できる contract test | 達成。`driver_parity`（`ui_dvr_03_parity_selection_command_intents_and_snapshot`）と `view::tests` |
| V2 | GPUI 実窓でタブクリックによる切替が動作する実機証拠 | 達成（手動証拠）。Windows の `component_showcase` 実窓で OS 実ポインタの左クリックにより `Ch 1 → Ch 2 → Ch 1` の切替を確認（スクリーンショット 2 枚を PR に記載）。自動化された実窓テストではない |
| V3 | Seek range を mount 後に更新する API の contract test（または remount 設計の決定記録） | 達成。決定記録（design）と `view::tests::descriptor_update_*` |

## 発見した不具合（本 Issue の一部として修正）

- **descriptor 更新で in-flight receipt が失われる。** 同じ key の control の descriptor が変わると `ViewHost::update` は control を再登録するが、その際に `command_index` も解放していた。進行中の command（例: Seek の commit）の receipt は routing を失い、binding が pending のまま残る。`view::tests::descriptor_update_keeps_in_flight_receipt_routing` が修正前に失敗することを確認した。`command_index` の解放は key の削除（removal）時だけに限定した。

## スコープ（本 PR）

- `ruviform-ui`: `TabsSpec.selection`（任意）、`TabsSelectionSpec`、`TAB_SELECT_ACTION`、`ViewHost::select_tab`、Tabs の displayed tab（`ComponentSnapshot.selected`）、descriptor 更新時の routing 維持。
- `ruviform-ui-gpui`: タブヘッダの mouse-down による選択、表示タブの描画（drawn plan を含む）、`RvfView::select_tab`、`GpuiObserver::select_tab`（テスト seam）、`component_showcase` のタブを選択可能にする（手動検証用）。
- `ruviform-ui-test`: parity fixture `selection-command.json` へのタブ選択の追加と Fake 側の検査、`component_view` の TabsSpec 追従。
- 文書 chain（spec → design → plan → task → test-spec）と docs index。

## 非スコープ（未達・別対応として明示）

- **キーボードによるタブ切替（GPUI 側）。** 契約（`ViewHost::select_tab`）は driver-neutral だが、GPUI のタブヘッダにフォーカスとキー入力の経路を足す作業は本 PR に含めない。follow-up Issue として切り出す。
- **webview driver での Tabs 描画と intent 発行。** `ruviform-ui-webview` は `ViewSpec` の描画経路を持たない（#448 と同じ理由）。NotRun とする。
- **consumer 側の project length 通知の接続。** Synvelis が project length 変化をどう検知して `update_view` を呼ぶかは consumer 側作業。
- **Seek の単位（秒）。** Synvelis#175 の単位（表示・書き込み両方が秒）は本 Issue の対象外。本 Issue は range と descriptor の更新契約だけを扱う。
- **複数の表示タブ・タブの追加削除時の選択保持の細則。** タブ数が減って表示中のタブが消えた場合は spec の `active` に戻すだけとし、それ以上の規則は定めない。
- **tab row の key による identity。** 選択は spec 上の index で表す（#448 の Table と同じ方針。YAGNI）。

## 不変条件

- 共通層（`ruviform-ui`）に GPUI 型を出さない。`check-ui-driver-boundary` が pass すること。
- 公開 enum は `#[non_exhaustive]` を維持する。`TabsSpec` への追加フィールドは `serde(default, skip_serializing_if)` で既存 JSON と後方互換にする。
- タブ選択は `ViewEvent::Intent`（`CommandIntent::Action`、`action: "tab-select"`、`params: {"index": n}`）として発行する。範囲外の index、selection を持たない Tabs、dead epoch の入力は `ViewError` で拒否し、intent を発行しない。
- 選択表示は楽観的に更新する。receipt が `rejected` / `unsupported` で終端したら直前のタブへ戻す。accepted / applied は表示を変えない。
- spec の `active` が変わった同一 key の update は、その `active` を表示へ採用する。`active` が変わらない update は、利用者が選んだ表示タブを保持する。
- descriptor 更新（contract change）は `update` の generation を変えず、進行中の command の receipt を失わない。gesture は従来どおり detach policy で終了する。

## 成功指標

- `driver_parity` と `consumer_observation` が、同じ `selection-command.json` から Fake と GPUI の同一の Action intent・snapshot・drawn plan を検査する。
- `view::tests` が、タブ選択の発行・拒否・revert・update の採用規則と、descriptor 更新での range 反映・receipt 維持を検査する。
- 既存の全テストと `mise run lint` / `lint-ui-gpui` / `fmt-check` が pass する。
