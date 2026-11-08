---
id: "DESIGN-64fKOE68pKEdIkGIEgcsyW"
type: "design"
status: "approved"
title: "Issue #479 — 値制御コンポーネントのキーボード操作の設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/479"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  refines:
    - "docs/specs/issue-479-keyboard-value-controls.md"
  depends-on:
    - "docs/specs/issue-479-keyboard-value-controls.md"
    - "docs/designs/issue-451-keyboard-tab-nav.md"
    - "docs/designs/issue-451-focus-next-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-448-table-selection-command.md"
---

# Issue #479 — 値制御コンポーネントのキーボード操作の設計

## 1. 結論

キーの解釈は GPUI の描画層（`ruviform-ui-gpui`）に置き、値の変更は既存の `ViewHost::dispatch_input`（`ControlInput`）を呼ぶ。契約（`ruviform-ui`）は変更しない。FU-1（Tabs キー）と同じ責務配置である。

```
key down (focused value control)
  -> RvfView key listener          [ruviform-ui-gpui: render.rs]
       -> control_key_input(key, modifiers)   # key name -> ControlInput
       -> RvfView::input(key, input)          # same path as pointer/wheel
            -> ViewHost::dispatch_input        # ruviform-ui (unchanged)
                 -> InteractionControl::dispatch (atomic Adjust/Set)
                 -> begin+commit CommandIntent / receipt / snapshot
```

## 2. 共通 key 規約（正本）

対象は `ControlSpec` を持つ全値制御 — `Fader` / `Seek` / `Knob` / `Toggle`。規約は種別に依らず同一で、`Toggle` は `step: 1` の二値レンジのため矢印キーがそのまま 0/1 の往復になる。

| キー | 入力 | 意味 |
| --- | --- | --- |
| `right` / `up` | `Adjust { steps: +1 }` | 1 ステップ増加 |
| `left` / `down` | `Adjust { steps: -1 }` | 1 ステップ減少 |
| `shift`+上記 | `Adjust { steps: ±1, fine }` | fine step（`InputModifiers.fine`） |
| `pageup` / `pagedown` | `Adjust { steps: ±10 }` | 大きな刻み（`PAGE_KEY_STEPS = 10`） |
| `shift`+`pageup`/`pagedown` | `Adjust { steps: ±10, fine }` | fine の大きな刻み |
| `home` / `end` | `Set { Extreme::Min / Max }` | 最小値 / 最大値へジャンプ |
| その他のキー、`control`/`alt`/`platform`/`function` 修飾 | — | 無視（`stop_propagation` しない） |

規約の根拠:

- 矢印キーの増減・Home/End の端ジャンプは WAI-ARIA slider/spinbutton の標準パターンであり、Tabs の Left/Right/Home/End（FU-1）とも向きが一致する。
- `shift` = fine は既存のホイール入力（`scroll_input` が `fine: ev.modifiers.shift` を渡す）と同じ約束事で、新しい修飾の意味を発明しない。
- PageUp/Down の「大きな刻み」は effective step の 10 倍（`PAGE_KEY_STEPS`）。step 未宣言時の fallback は normalized 0.01 なので page は 0.1 — いずれの記述子でも「1 ステップの 10 倍」という相対量として一貫する。絶対量（範囲の 10% 等）を renderer が計算すると `effective_step` の領域規則を複製することになるため採用しない。
- 各押下は `gesture_id: None` の atomic 入力。ホイールと同じ begin+commit synthesized pair（`atomic: true`）を生み、キーリピートは押下ごとの独立した atomic 入力となる。keydown/keyup で gesture を保持する方式は GPUI のキーリピート意味論と FocusLost 対応が複雑になるため採用しない。

## 3. 責務の配置

| 責務 | 置き場所 | 理由 |
| --- | --- | --- |
| GPUI の key 名・修飾の解釈、key → `ControlInput` の写像 | `render.rs`（`control_key_input`） | key 入力の正規化は renderer の責務（`tab_key_target`・pointer 正規化と同じ層） |
| `Adjust`/`Set` の値解決・量子化・fine step | `ruviform-ui`（`ValueMapper::adjust_normalized` / `effective_step`） | driver-neutral な唯一の実装。変更しない |
| gesture → `CommandIntent`・receipt・revert | `ruviform-ui`（既存） | 変更しない |
| フォーカスの保持 | `RvfView::control_focus`（control ごとの `FocusHandle`） | `tab_focus` と同じく view 単位で持つ |
| テスト用の key 注入 | `GpuiObserver::press_control_key` / `focused_control`（seam） | `press_tab_key` / `focused_tab` と同型。実フォーカスと実 key dispatch を通すため |

## 4. フォーカスの扱い

- `RvfView` は `control_focus: BTreeMap<ComponentKey, FocusHandle>` を持つ。control のフォーカスハンドルは、その control を初めて描画するときに作る。
- `Fader` / `Seek` / `Knob` は `control_element`、`Toggle` は `toggle_element` で、いずれも `track_focus` + `tab_stop(true)` + `.focus(枠線)` + `on_key_down` を付ける。`tab_stop` は FU-1b の教訓どおり **handle 自体**に付ける（gpui 0.2.2 は `track_focus` に渡した明示 handle に要素側フラグを伝播しない）。
- 値制御は opt-in なしですべて tab stop になる。`disabled` / `read_only` / `unsupported` でも stop にはなるが、契約層が `Unavailable` を報告し intent は生まれない（pointer が効かないのと同じ正直な扱い）。
- 未定義キー・対象外修飾のキーは `stop_propagation` せず、Tab 巡回（driver の `observe_keystrokes`）や他要素の処理を奪わない。
- 既知の制約: control のフォーカスハンドルは unmount 時に解放しない（`tab_focus` と同じ有界リーク。strip 数・control 数に比例し spec 依存で有界）。

## 5. key の処理

- `control_key_input(key, modifiers)` は `Option<ControlInput>` を返す純粋関数。`control`/`alt`/`platform`/`function` のいずれかが立っていれば `None`。`shift` のみ `fine` に写す。
- `home`/`end` は `ValueInput::Extreme` を `Set` で送り、`resolve_plain` 経由で range policy・snap を通る（`fine` は Extreme では無視される）。
- `Adjust` の `steps` は effective step の整数倍として `adjust_normalized` が量子化・クランプする。端でのクランプは契約層の責務で、renderer は端判定を持たない。
- 発行される intent は atomic な `begin`+`commit`（`atomic: true`）で、`CommandIntent` には `source` が載らない — pointer/wheel/keyboard の区別は `ViewEvent::Gesture` の `InteractionEventMeta.source`（`"keyboard"`）に現れる。GPUI 側は `press_control_key` が `InputSource::KEYBOARD` を乗せた入力を作るため、seam 越しの intent 列は pointer 系と同じ形になる。

## 6. 検証の設計

- 規則: `control_key_input` を純粋関数として unit test する（全キー・修飾組合せ・未知キー）。
- 実経路: render.rs の gpui-test で `RvfView` を mount し、focus → `simulate_keystrokes` で実キー dispatch → gesture event の `source == "keyboard"` と commit 値を確かめる。未フォーカスでは無視されること、Tab が control に届くことも確かめる。
- seam: `GpuiObserver::press_control_key`（フォーカスしてから key dispatch）と `focused_control` で、`consumer_observation.rs` が fixture 駆動の parity を検査する。
- 共有 fixture: `key-ops.json` の `expect.controls.*.keys` を、Fake 側（fixture が宣言する `adjust`/`extreme` を `source: keyboard` の `ControlInput` として dispatch し、gesture source・intent kinds・committed を検査）と GPUI seam（fixture の key 名を実 dispatch）の両方が検査する。fixture は key→入力の対応をデータとして pin するため、GPUI 側の写像がずれれば intent・値が一致せず fail する。

## 7. 代替案と却下理由

| 案 | 内容 | 判定 |
| --- | --- | --- |
| A | 契約に key 入力 API（`ViewHost::press_key` 等）を足す | 却下。契約に GPUI の key 名語彙が入り driver-neutral を崩す。他 driver の入力要件も未確定 |
| B | key→入力の写像を `ruviform-ui` に置く | 却下（現時点）。key 名の共通語彙がなく消費者もいない。webview が key 入力を持つ時点で昇格を検討する（FU-1 design と同じ判断） |
| C | keydown で Begin、リピートで Preview、keyup で Commit の実 gesture | 却下。GPUI のキーリピート・フォーカス喪失時の Cancel 対応が複雑で、ホイールの atomic 入力と非対称になる。atomic で十分 |
| D（採用） | 描画層で key を解釈し、既存の atomic `Adjust`/`Set` を `source: keyboard` で dispatch | 採用。契約変更なし。pointer/wheel と同じ経路で receipt と revert が効く |

## 8. 既知の未解決事項（本設計の外）

- Space/Enter 等の離散起動キーと Button のキーボード起動は未対応（後続 Issue）。
- 実 OS のキー入力経路は #313 の責務。実窓証拠は NotRun。
- control のフォーカスハンドルは unmount で解放されない（`tab_focus` と同じ有界リーク）。
