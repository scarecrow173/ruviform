---
id: "SPEC-Kb7Tn3QwRx9LpVc2Hy5Ud8"
type: "spec"
status: "approved"
title: "Issue #451 FU-1 — Tabs のキーボード切替"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-451-keyboard-tab-nav.md"
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
  implements: []
  implemented-by: []
  refines:
    - "docs/specs/issue-451-tabs-pointer-descriptor.md"
  depends-on:
    - "docs/specs/issue-451-tabs-pointer-descriptor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-448-table-selection-command.md"
---

# Issue #451 FU-1 — Tabs のキーボード切替

## 背景

PR #457 で Tabs の pointer 切替（`tab-select` Action intent、`ViewHost::select_tab`、`TabsSpec.selection` opt-in、`ComponentSnapshot.selected`）を実装した。#451 spec の A1 は「pointer は達成、キーボードは未実装」の部分達成で、キーボード操作は FU-1 として残っている。

GPUI のタブヘッダには、現時点でフォーカスも key 入力の経路もない。GPUI 側の他の control（Seek/Fader/Knob）にもキーボード操作はなく、`InputSource::KEYBOARD` は定義のみで未使用である。

## 目的

- `TabsSpec.selection` を持つ Tabs が、フォーカスとキー入力でタブを切り替えられる。切替は pointer と同じ `tab-select` intent・receipt・snapshot の遷移を通る。
- Fake と GPUI の両方で、キーボード経路の intent と snapshot が一致する。

## 受入条件

| # | 受入 | 扱い |
| --- | --- | --- |
| K1 | selection を持つ Tabs で、フォーカスとキー入力によりタブを切り替えられる。pointer と同じ `tab-select` intent・receipt・snapshot 遷移を通る | 達成。GPUI の gpui-test（実フォーカスと実キー dispatch）と `observes_tab_key_intents_through_seam` |
| K2 | Fake/GPUI parity で、キーボード経路の intent が一致する | 達成。`selection-command.json` の `expect.tabs.keyboard` を Fake（`ui_dvr_03_parity_tab_keyboard_intents`）と GPUI seam の両方が検査する |
| K3 | GPUI 実窓でのキーボード切替の実機証拠 | NotRun。本作業では実窓への key 操作を行っていない。PR 本文に理由と後続を記載する |

## キー割当

WAI-ARIA の Tabs パターンに従う。

- Left / Right: 前後のタブへ移動する。端では循環する。
- Home / End: 先頭 / 末尾のタブへ移動する。
- 移動と同時に選択する（自動アクティベーション）。Enter / Space による確定は不要。
- 修飾キー付きの入力は無視する。

既存の GPUI control にはキーボード規約がないため、この標準パターンを採用する。独自割当ではない。

## フォーカス

- `selection` を持つ Tabs のヘッダだけがフォーカス可能である。`selection` のない Tabs はフォーカスも key 処理も持たず、従来どおりキーボード対象外となる。
- タブの押下はヘッダにフォーカスを移してから選択する。
- フォーカス中のヘッダは枠線で示す。
- `tab_stop(true)` を設定し、ホストアプリが Tab で `focus_next` をバインドしたときに Tab 循環の対象にする。GPUI 0.2.2 は既定の Tab バインディングを持たないため、既定の状態では Tab で到達しない点を既知の制約とする。

## スコープ（本 PR）

- `ruviform-ui-gpui`: タブヘッダのフォーカス、`on_key_down`、キーから移動先を求める規則（`tab_key_target`）、表示タブの読み出し。
- `ruviform-ui-gpui` の seam（`GpuiObserver::press_tab_key`）: ヘッダにフォーカスしてから GPUI の key dispatch で実キーを送る。
- `ruviform-ui-test`: parity fixture `selection-command.json` の `expect.tabs.keyboard` と Fake 側の検査。
- 文書 chain（spec → design → plan → task → test-spec）と docs index。

## 非スコープ

- **`ruviform-ui` の契約変更。** `ViewHost::select_tab` は既存のまま使う。キーから index への規則は GPUI の描画層に置く。
- **Seek / Fader / Knob のキーボード操作。** GPUI に規約がないため、別 Issue とする。
- **ホストアプリの Tab キー配線。** `focus_next` のバインドはホスト側の作業とする。
- **実窓でのキー操作の手動証拠（K3）。** NotRun とする。
- **webview driver での Tabs 描画。** #451 と同じ理由で NotRun とする。

## 不変条件

- 共通層（`ruviform-ui`）に GPUI 型と key 名を出さない。`check-ui-driver-boundary` が pass すること。
- キーによる移動先は表示中のタブを基準に求める。移動先が表示中のタブと同じなら intent を出さない。
- キーによる選択は pointer と同じ `select_tab` を通る。rejected / unsupported による revert の規則も同じ。
- 公開 API の追加は、テスト seam（`GpuiObserver`）に限る。

## 成功指標

- `mise run test-ui-gpui` が、新しい key テストと seam テストを含めて pass する。
- `mise run test-ui-parity` が、キーボード経路の intent を Fake 側で検査して pass する。
- `mise run lint-ui-gpui`、`check-ui-driver-boundary`、`test-ui-driver-contract`、`test-ui-component-view`、`fmt-check` が pass する。
