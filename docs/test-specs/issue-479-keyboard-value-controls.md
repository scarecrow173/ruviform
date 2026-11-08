---
id: "TSPEC-LqfidO2iIqcMjOeeHeLMd2"
type: "test-spec"
status: "approved"
title: "Issue #479 — 値制御キーボード操作の検証契約"
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
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/key-ops.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  verifies:
    - "docs/specs/issue-479-keyboard-value-controls.md"
    - "docs/designs/issue-479-keyboard-value-controls.md"
  depends-on:
    - "docs/specs/issue-479-keyboard-value-controls.md"
    - "docs/designs/issue-479-keyboard-value-controls.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-451-keyboard-tab-nav.md"
---

# Issue #479 — 検証契約

## Feature: 値制御コンポーネントのキーボード操作

### Rule: 共通 key 規約は矢印 ±1 ステップ・shift で fine・PageUp/Down で 10 ステップ・Home/End で端ジャンプを atomic 入力に写す

- Example: `control_key_input` は `right`/`up` → `Adjust{steps:+1}`、`left`/`down` → `Adjust{steps:-1}`、`pageup`/`pagedown` → `Adjust{steps:±10}`、`home`/`end` → `Set{Extreme::Min/Max}` を返す。`shift` 併用は `InputModifiers.fine = true`。`control`/`alt`/`platform`/`function` 修飾と未知キー（`enter`、`tab` 等）は `None`。
  - 検査: `control_key_input_maps_keys_to_atomic_inputs`（GPUI render unit、gpui-test feature）。

### Rule: フォーカスしていない control は key を受けない。フォーカス後の key が `source: keyboard` の gesture と commit を生む

- Example: 未フォーカスで `right` を送っても値は変わらない。control をフォーカスして `right` → `shift-right` → `pageup` → `home` → `end` を送ると、各押下が `begin`+`commit`（`atomic: true`、gesture meta の `source` は `"keyboard"`）を生み、committed normalized が規約どおり変わる。
  - 検査: `control_keys_adjust_value_through_focus`（GPUI gpui-test。実フォーカスと `simulate_keystrokes`）。

### Rule: 値制御は tab stop であり、Tab 巡回で focus が届く

- Example: Tabs strip と Fader を持つ view で Tab を押すと巡回順に focus が移り、Fader に focus がある状態で `focused_control` がその key を返す。Tab 自体は値を変えない。
  - 検査: `tab_traversal_reaches_value_controls`（GPUI consumer seam、`press_key`/`focused_control`）。

### Rule: Fake 側でも、キーボード経路の入力は同じ gesture source・intent・snapshot を生む

- Example: fixture `key-ops.json` の各 step が宣言する `adjust`/`extreme` を `source: keyboard` の `ControlInput` として dispatch すると、gesture event の `meta.source` は `"keyboard"`、intent kinds は `["begin","commit"]`、committed normalized は `committedNormalized` と一致する。
  - 検査: `ui_dvr_03_parity_key_ops_keyboard_intents`（Fake、`ruviform-ui-test`）。

### Rule: GPUI 側は fixture の key 名を実キー dispatch し、同じ intent・snapshot を生む

- Example: `press_control_key` で fixture の `key` を送ると、command intent kinds は `["begin","commit"]` の積み重ね、committed normalized は `committedNormalized` と一致する。Fader/Seek/Knob/Toggle の 4 種で同じ規約が効く。
  - 検査: `observes_key_op_intents_through_seam`（GPUI consumer seam）。

### Rule: `disabled` / `read_only` の control も tab stop のままだが、key 入力は `unavailable` を報告し intent を生まない

- Example: fixture の `disabled_fader`（`disabled: true`）と `readonly_knob`（`read_only: true`）に key を送ると、各押下は `source: "keyboard"` の `unavailable` 通知（reason は `disabled` / `readOnly`）を生むだけで、intent は 0 件、committed normalized は初期値のまま。control はフォーカスを保持する。
  - 検査: `ui_dvr_03_parity_key_ops_keyboard_intents`（Fake。unavailable の reason・件数・source を検査）と `observes_key_op_intents_through_seam`（GPUI seam。intent 0 件・committed 不変・`focused_control` が key を返すことで検査）。

## 未検証（NotRun）

- **実窓でのキー操作。** GPUI の実ウィンドウ上で key を送ることは本作業では行っていない。実 OS 入力経路の自動化は #313 の責務。手動証拠は後続 Issue で扱う。
- **キーリピートの時間軸。** 押下ごとの atomic 入力として検証し、OS のキーリピート間隔・連続押下の実時間挙動は対象外。
