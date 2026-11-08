---
id: "SPEC-z3dzJJAH4RMnLrkQfhkR0m"
type: "spec"
status: "approved"
title: "Issue #479 — Seek/Fader/Knob のキーボード操作（共通 key 規約）"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/479"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-479-keyboard-value-controls.md"
      - type: "file"
        path: "docs/test-specs/issue-479-keyboard-value-controls.md"
      - type: "file"
        path: "docs/plans/issue-479-keyboard-value-controls.md"
      - type: "file"
        path: "docs/tasks/issue-479-keyboard-value-controls.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/key-ops.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  refines:
    - "docs/specs/issue-451-keyboard-tab-nav.md"
  depends-on:
    - "docs/specs/issue-451-keyboard-tab-nav.md"
    - "docs/specs/issue-451-focus-next-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-448-table-selection-command.md"
---

# Issue #479 — Seek/Fader/Knob のキーボード操作（共通 key 規約）

## 背景

#451 FU-1 で Tabs のキーボード切替（`tab-select` intent）を、FU-1b で Tab/Shift-Tab のフォーカス巡回配線を実装した。これで focus は tab stop 間を移動できるが、Seek/Fader/Knob/Toggle の値制御コンポーネントにはキーボード操作の規約がなく、キーボードでは値を変えられない。`InputSource::KEYBOARD` は契約に定義済みだが未使用である。

ポインタ経路は `Begin` → `Preview*` → `Commit` の gesture 契約、ホイール経路は `Adjust` の atomic 入力（`begin`+`commit` の synthesized pair、`atomic: true`）を通る。キーボードも同じ driver-neutral な `ControlInput` 経路に乗せるのが自然である。

## 目的

- 値制御コンポーネント（`ControlSpec` を持つ Fader / Seek / Knob / Toggle）に、全種で共通のキーボード規約を定めて実装する。
- キー入力は既存の driver-neutral intent 経路（gesture → `CommandIntent`）を通り、pointer / wheel と同じ receipt・snapshot の遷移に従う。
- Fake と GPUI の両方で、キーボード経路の intent と snapshot が一致する。

## 受入条件

| # | 受入 | 扱い |
| --- | --- | --- |
| K1 | 共通 key 規約が design 文書に記録される（矢印キー・PageUp/Down・Home/End・修飾キーでの刻み幅変更） | 達成。`docs/designs/issue-479-keyboard-value-controls.md` |
| K2 | FakeDriver（`ViewHost`）でキー相当の `ControlInput` が driver-neutral intent を生む contract test | 達成。`key-ops.json` fixture の fake side（`ui_dvr_03_parity_key_ops_keyboard_intents`） |
| K3 | GPUI parity test：focus 済み control への実キー dispatch が同じ intent・snapshot を生む | 達成。`consumer_observation.rs` の seam テストと render.rs の gpui-test |
| K4 | GPUI 実窓でのキー操作の実機証拠 | NotRun。実 OS 入力経路は #313 の責務。PR 本文に理由と後続を記載する |

## キー規約（要約 — 正本は design 文書）

WAI-ARIA の slider / spinbutton パターンと、既存のホイール入力（Shift = fine step）に倣う。

- `left` / `down`: 1 ステップ減少。`right` / `up`: 1 ステップ増加。
- `shift` 併用: fine step（`InputModifiers.fine`、記述子の `fine_step` が効く）。
- `pageup` / `pagedown`: 10 ステップ分の大きな刻み（`shift` 併用で fine 10 ステップ）。
- `home` / `end`: 最小値 / 最大値へのジャンプ（`ValueInput::Extreme`）。
- `control` / `alt` / `platform` / `function` 修飾、未定義キー: 無視（propagation を止めない）。
- 各キー押下は atomic な `Adjust` / `Set` 入力 1 回 = `begin`+`commit` intent 1 組。キーリピートは押下ごとに独立した atomic 入力となる。

## フォーカス

- 値制御コンポーネントはすべて tab stop であり、Tab/Shift-Tab 巡回（FU-1b）の対象になる。
- フォーカス中の control は枠線で示す（Tabs ヘッダと同じ `.focus` 表現）。
- `disabled` / `read_only` / `unsupported` の control も tab stop にはなるが、入力は契約層が `Unavailable` を正直に報告し intent を生まない。

## スコープ（本 PR）

- `ruviform-ui-gpui`: `control_key_input`（key → `ControlInput` の純粋関数）、値制御 element への `track_focus` / `tab_stop` / `on_key_down` 配線、focus handle の保持、観測 seam（`press_control_key` / `focused_control`）。
- `ruviform-ui-test`: parity fixture `key-ops.json` と Fake 側の検査。
- 文書 chain（spec → design → plan → task → test-spec）と docs index。

## 非スコープ

- **`ruviform-ui` の契約変更。** `ControlInput::Adjust`/`Set`、`InputModifiers.fine`、`InputSource::KEYBOARD`、`ValueInput::Steps`/`Extreme` は既存のまま使う。キー名の解釈は GPUI の描画層に置く。
- **Tab/Shift-Tab のフォーカス巡回自体。** FU-1b で実装済み。
- **Space/Enter 等の離散起動キー。** Button のキーボード起動と併せて後続 Issue とする。Toggle は矢印キーで 0/1 を往復できるため本規約で操作可能。
- **実 OS 入力経路の自動化。** #313 の責務。
- **webview driver でのキー入力。** key 入力経路を持たないため対象外。

## 不変条件

- 共通層（`ruviform-ui`）に GPUI 型と key 名を出さない。`check-ui-driver-boundary` が pass すること。
- キー入力は pointer / wheel と同じ `dispatch_input` → gesture → `CommandIntent` 経路を通る。receipt・revert の規則も同じ。
- 処理したキーは `stop_propagation` するが、未定義キーと Tab は止めない（Tab 巡回と他のキー処理を奪わない）。
- 公開 API の追加は、テスト seam（`GpuiObserver`）に限る。

## 成功指標

- `mise run test-ui-gpui` が、新しい key テストと seam テストを含めて pass する。
- `mise run test-ui-parity` が、キーボード経路の intent を Fake 側で検査して pass する。
- `mise run lint-ui-gpui`、`check-ui-driver-boundary`、`test-ui-driver-contract`、`test-ui-component-view`、`fmt-check` が pass する。
