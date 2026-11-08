---
id: "PLAN-1OVcOgiEi7tc3VMXPzF9F3"
type: "plan"
status: "approved"
title: "Issue #479 — 値制御キーボード操作の実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/479"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-479-keyboard-value-controls.md"
      - type: "file"
        path: "docs/test-specs/issue-479-keyboard-value-controls.md"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/key-ops.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-479-keyboard-value-controls.md"
  implemented-by: []
  refines:
    - "docs/plans/issue-451-keyboard-tab-nav.md"
  depends-on:
    - "docs/designs/issue-479-keyboard-value-controls.md"
    - "docs/specs/issue-479-keyboard-value-controls.md"
  verified-by:
    - "docs/test-specs/issue-479-keyboard-value-controls.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-451-focus-next-wiring.md"
---

# Issue #479 — 値制御キーボード操作の実装計画

設計 `DESIGN-64fKOE68pKEdIkGIEgcsyW` を実装に落とす。変更は GPUI の描画層と seam、共有 fixture に限り、契約（`ruviform-ui`）は変えない。

## スライス

| スライス | 内容 | 依存 |
| --- | --- | --- |
| S1 | 失敗するテストを先に書く。`control_key_input` の unit test、gpui-test の key 経路テスト、seam テスト、Fake 側の parity テスト、fixture `key-ops.json` | — |
| S2 | `RvfView` に `control_focus` を足し、`control_key_input` を実装する | S1 |
| S3 | `control_element` / `toggle_element` に `track_focus` / `tab_stop` / `.focus` / `on_key_down` を配線する | S2 |
| S4 | `GpuiObserver::press_control_key` / `focused_control` と `RvfView::focused_control_key` を追加する | S3 |
| S5 | fmt、lint、各 UI gate を実行する。実行できない項目は NotRun として記録する | S4 |
| S6 | main の取り込み確認、docs の索引再生成、PR 作成 | S5 |

## ファイル対応

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/src/render.rs` | `control_focus` フィールドと `control_focus()`、`focused_control_key()`、`control_key_input`、`control_element`/`toggle_element` の focus/key 配線、テスト |
| `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` | `GpuiObserver::press_control_key`、`GpuiObserver::focused_control` |
| `crates/ruviform-ui-gpui/tests/consumer_observation.rs` | `observes_key_op_intents_through_seam`、`tab_traversal_reaches_value_controls` |
| `crates/ruviform-ui-test/fixtures/driver-lab/parity/key-ops.json` | 新規 fixture |
| `crates/ruviform-ui-test/tests/driver_parity.rs` | `ui_dvr_03_parity_key_ops_keyboard_intents` |
| `docs/**` | 文書 chain と各 README の索引 |

`Cargo.toml` と `Cargo.lock` は変更しない。依存は追加しない（`Keystroke`/`Modifiers`/`InputModifiers`/`Extreme` は既存依存から取り込む）。

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
| 実窓 | 手動 | NotRun（K4） |

## ロールバック

変更は GPUI の描画層と seam、テストと fixture に限られる。問題があれば、この PR の差分を revert する。契約と既存の pointer/wheel 経路は変わらないため、revert しても既存機能は残る。

## リスク

| リスク | 対応 |
| --- | --- |
| 値制御が一律 tab stop になり Tab 巡回順が変わる | 意図した変更。既存テストは tabs のみの spec で影響なし。実窓の巡回体感は K4 の後続で確認 |
| `press_control_key` が非 control key で無効な focus を作る | `control_focus` は mounted spec に control がある場合のみ handle を作り、なければ `UiError::Host` |
| PageUp/Down の 10 ステップが descriptor に対して過大/過小 | effective step の 10 倍という相対量で一貫。絶対量は renderer が `effective_step` を複製するため却下（設計 2） |
| GPUI の key 名が実 OS で異なる | `pageup`/`pagedown`/`home`/`end` は gpui 0.2.2 の keystroke 名として確認済み。実 OS 経路は #313 |
