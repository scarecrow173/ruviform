---
id: "PLAN-Qf3Nr9WxKc7TbVz2LmHd4P"
type: "plan"
status: "approved"
title: "Issue #451 FU-1 — Tabs のキーボード切替の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-451-keyboard-tab-nav.md"
      - type: "file"
        path: "docs/test-specs/issue-451-keyboard-tab-nav.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-451-keyboard-tab-nav.md"
  implemented-by: []
  refines:
    - "docs/plans/issue-451-tabs-pointer-descriptor.md"
  depends-on:
    - "docs/designs/issue-451-keyboard-tab-nav.md"
    - "docs/specs/issue-451-keyboard-tab-nav.md"
  verified-by:
    - "docs/test-specs/issue-451-keyboard-tab-nav.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-448-table-selection-command.md"
---

# Issue #451 FU-1 — Tabs のキーボード切替の実装計画

設計 `DESIGN-Hm4Vq8ZsLw2KxTn6RcYb1G` を実装に落とす。変更は GPUI の描画層と seam、共有 fixture に限り、契約（`ruviform-ui`）は変えない。

## スライス

| スライス | 内容 | 依存 |
| --- | --- | --- |
| S1 | 失敗するテストを先に書く。`tab_key_target` の unit test、gpui-test の key 経路テスト、seam テスト、Fake 側の parity テスト、fixture の `expect.tabs.keyboard` | — |
| S2 | `RvfView` に `tab_focus` を足し、`displayed_tab` を `(host, key, active)` に変える | S1 |
| S3 | Tabs ヘッダに `track_focus` / `tab_stop` / `on_key_down` を付け、mouse-down でフォーカスを移す。`tab_key_target` を実装する | S2 |
| S4 | `GpuiObserver::press_tab_key` と `window_of` を追加する | S3 |
| S5 | fmt、lint、各 UI gate を実行する。実行できない項目は NotRun として記録する | S4 |
| S6 | main の取り込み確認、docs の索引再生成、PR 作成 | S5 |

## ファイル対応

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/src/render.rs` | `tab_focus` フィールドと `tab_focus()`、Tabs ヘッダの focus / key 処理、`tab_key_target`、`displayed_tab` の引数変更、テスト |
| `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` | `GpuiObserver::press_tab_key`、`GpuiDriver::window_of` |
| `crates/ruviform-ui-gpui/tests/consumer_observation.rs` | `observes_tab_key_intents_through_seam` |
| `crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json` | `expect.tabs.keyboard` |
| `crates/ruviform-ui-test/tests/driver_parity.rs` | `ui_dvr_03_parity_tab_keyboard_intents` |
| `docs/**` | 文書 chain と各 README の索引 |

`Cargo.toml` と `Cargo.lock` は変更しない。依存は追加しない（`gpui` の `Keystroke` と `AppContext` は既存依存から取り込む）。

## 検証マトリクス

| 検証 | コマンド | 目的 |
| --- | --- | --- |
| 形式 | `mise run fmt-check` | 整形 |
| GPUI lint | `mise run lint-ui-gpui` | `-D warnings` の clippy |
| GPUI テスト | `mise run test-ui-gpui` | key 経路、seam、既存の render テスト |
| Fake parity | `mise run test-ui-parity` | キーボード経路の intent（Fake 側） |
| 観測 seam | `mise run test-ui-consumer-observation` | GPUI 側の intent と snapshot |
| 契約 | `mise run test-ui-driver-contract`、`test-ui-component-view` | manifest と既存 fixture |
| 境界 | `mise run check-ui-driver-boundary` | 契約層に GPUI 型を出していないこと |
| 実窓 | 手動 | NotRun（K3） |

## ロールバック

変更は GPUI の描画層と seam、テストと fixture に限られる。問題があれば、この PR の差分を revert する。契約と既存の pointer 経路は変わらないため、revert しても #457 の機能は残る。

## リスク

| リスク | 対応 |
| --- | --- |
| GPUI の focus 挙動が実窓と test platform で異なる | key 経路は gpui-test で確認。実窓は K3 として NotRun と明記 |
| strip のフォーカスハンドルが unmount 後も残る | 有界であり、現時点では対処しない（設計 4） |
| Tab 循環が既定では効かない | ホスト側の `focus_next` 配線を後続とする（設計 7） |
