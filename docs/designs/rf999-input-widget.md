---
id: "DESIGN-Rf999InputWidget01"
type: "design"
status: "approved"
title: "Issue #999 — Input widget の shape・submit 境界・拒否契約の設計判断"
created: "2026-11-25"
updated: "2026-11-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/999"
    - "docs/specs/rf999-input-widget.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/rf999-input-widget.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/rf999-input-widget.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-944-ui-command-params.md"
    - "docs/specs/issue-448-table-selection-command.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/944"
    - "https://github.com/scarecrow173/synvelis/issues/125"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue:
    number: 999
    url: "https://github.com/scarecrow173/ruviform/issues/999"
---

# Issue #999 — Input widget 設計判断

`docs/specs/rf999-input-widget.md`（wire 契約正本）の裏側にある形状判断を
記録する。#944（user-entered params）と #448（Button/selection）の先例に
揃えた上で残った自由度をここで確定する。

## 決定事項

### D1: variant 形状 — 1 variant + `kind` タグ付き value 契約

`ComponentSpec::Input(InputSpec)` 1 つの wire type（`"type": "input"`）とし、
入力種別は `InputSpec.value` の内部タグ enum `InputValueSpec`
（`{"kind": "text" | "integer" | "number", ...}`）で表す。

|| 選択肢 | 判定 | 理由 |
| --- | --- | --- |
| (i) 1 variant + `kind` タグ付き value 契約 | 採用 | `key()`/`kind()`/`a11y_node()`/receipt routing が 1 経路で済み、`TabsSelectionSpec` 等の nested-spec 先例と整合。per-kind field（将来の text `max_len` 等）の拡張も kind 内に閉じる |
| (ii) kind 別 2〜3 variant（`TextInput`/`NumberInput` …） | 棄却 | `Fader`/`Seek`/`Knob`/`Toggle` の複数 `type` 共有 `ControlSpec` 先例は「同一 contract の別描画」であり、入力系は kind ごとに制約 field が異なるため同型に乗らない。`type` 値を 3 つ増やす対価に見合わない |
| (iii) `Button` 的な `params: serde_json::Value` + submit 値を spec 外部で結合 | 棄却 | typed profile（`CommandParams`）を経る経路が #944 の期待形であり、「param 名を spec が宣言」するのが本 widget の目的 |

### D2: `integer` kind の存在理由

`number`（f64）だけでも数値入力は表せるが、PPQN（1..=32767）のような
離散値を `480`（JSON 整数）として運ぶには `CommandParam::Integer` が
必要 — `480.0` と `480` は consumer 側の param schema で別物になりうる。
`CommandParam::Integer` の doc が "indices, counts, PPQN" を明示しており、
本 Issue の動機付け consumer（synvelis `syn.project.ppqn`/`new_ppqn`）と
一致するため、`integer`/`number`/`text` の 3 kind とした。

### D3: 拒否は mount 時 `InvalidSpec` と submit 時 `InvalidInput` の 2 段

- **mount 時**（`check_input`）: `param` 空・`min > max`・`number` bound の
  非有限値は spec の構造的不正として `InvalidSpec`。`check_command` の
  `target`/`action` 空チェックは `Button` と同一 arm に乗せた。
- **submit 時**（`InputValueSpec::parse`）: parse 不能・非有限・範囲外は
  `InvalidInput` を返し intent を emit しない（`select_row` の
  "rejected inputs emit nothing" 規則、`dispatch_action` の非有限拒否と同型）。
- min/max の責務分界: spec の bounds は widget の UI 契約（host が submit
  を制限する範囲）であり、consumer 側 admission（`syn.project.ppqn` の
  op 側範囲検証）とは別層。host は consumer の正当性を判断しない
  （#944 D7 の分界を継承）。

### D4: `param`（params メンバ名）は spec 宣言

`ButtonSpec` が `target`/`action`/`params` を spec に書くように、Input は
`param`（submit 値を置く params キー名）を spec に書く。renderer は値の
文字列表現だけを供給し、intent の語彙はすべて spec 由来 —— renderer が
consumer 語彙を知らなくてよい（#944 D4 と同じ分界）。

### D5: 提出 API — `submit_input(generation, key, text: &str)`

`invoke`（Button の spec 静的 press）と `dispatch_action`（runtime params
の任意 mint）の中間: spec が `target`/`action`/`param` を持ち、値だけが
runtime 入力。headless 契約上の入力は `&str`（text field の生文字列）とし、
parse は host 側 —— renderer は描画と文字列供給に徹し、kind ごとの意味
付けを複製しない。

- numeric kind は trim してから parse（renderer が前後空白を残してよい）。
- `Text` kind は verbatim（trim しない — 空白も値の一部）。
- 非有限数値（`NaN`/`inf`/`1e400`）は `CommandParam::to_value` が wire 表現を
  持たないため submit 段で拒否（#944 D2 の canonical codec 規則と整合）。

### D6: a11y — `TextBox`/`SpinButton` の 2 role 追加

`A11yRole`（`#[non_exhaustive]`、camelCase serde）に `textBox`（text
entry）と `spinButton`（bounded number entry）を追加し、既定 role を
`value.kind` から導出する（`a11y.role` で明示 override 可 — Button と同じ
fallback 規則）。`A11ySpec.value_min`/`value_max` は範囲制約の a11y 露出に
使えるが、host は spec の `value` bounds を自動転記しない —— consumer が
a11y field に書くのが既存の宣言モデル。

### D7: receipt routing・非回帰

`action_binding_id` に `Input` を登録（id = component key、Button と同じ
規則）。`spec_binding_id`・`owner_key_for`・`release`・`observe` は同経路を
自動的に辿る。`CommandBinding`/`CommandPhase`/`selection_undo` 等の追跡
意味、`drain_events`/`ViewEvent::Intent`/`apply_receipt` の観測経路は不変。
`dispatch_action`/`invoke`/`select_row`/`select_tab` の既存 mint 経路と
`ButtonSpec`/`action_params` の serde 契約は一切変更しない。

## 非回帰条件

- 既存 variant・`CommandIntent`・`params` 語彙の serde 出力は不変 —
  driver-lab parity fixture（`crates/ruviform-ui-test`）の既存ケースが
  pass することが直接証拠。
- 新 `type` 値は下流の wildcard arm で `"unsupported component"` 表示に
  落ちる —— compile 安全、adapter render は別 slice。
- `A11yRole`/`InputValueSpec` は `#[non_exhaustive]`（公開 enum ポリシー）。

## Deferred Design Concerns

|| 論点 | 本書での扱い | 移管先 |
| --- | --- | --- |
| `Input` の render 実装（GPUI/webview）・IME・focus・placeholder | headless wire 契約のみ実装 | adapter render slice（実 GUI gate synvelis#114 / ruviform#631 と共に） |
| parity fixture の `Input` ケース | `ruviform-ui-test` 編集範囲外 | render adapter slice と同じ段で |
| text kind の制約語彙（max_len・pattern）・Enum/Point 等の他 CommandParam kind の入力 | `value` の per-kind field 拡張 or 新 kind で対応可能 | 必要になった時点の個別 Issue |
| consumer 側の typed-projection 配線（host→widget 向き） | #944 D6 既存経路を継承 | synvelis#125 統合検討 |
