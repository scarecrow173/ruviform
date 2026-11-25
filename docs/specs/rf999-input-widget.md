---
id: "SPEC-Rf999InputWidget01"
type: "spec"
status: "approved"
title: "Issue #999 — ComponentSpec 任意値 text/number 入力 widget の wire 契約"
created: "2026-11-25"
updated: "2026-11-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/999"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/rf999-input-widget.md"
      - type: "doc"
        path: "docs/designs/rf999-input-widget.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-387-component-view-specs.md"
    - "docs/specs/issue-448-table-selection-command.md"
    - "docs/designs/issue-944-ui-command-params.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/944"
    - "https://github.com/scarecrow173/synvelis/issues/125"
    - "https://github.com/scarecrow173/synvelis/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/631"
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

# Issue #999 — ComponentSpec 任意値 text/number 入力 widget の wire 契約

## 背景

Issue #944 で `Action` intent の `params` に user-entered 値を運ぶ型付き
profile（`CommandParam`/`CommandParams`）と、その mint 経路
`ViewHost::dispatch_action` が導入された。同設計の Deferred Concern として
「params を生む component kind（入力 widget）」が本 Issue へ分離されていた。

`ComponentSpec` の既存 kind のうち command 発行系は `Button`（spec 静的
params）と `Table`/`Tabs` の `selection`（固定 action・params）だけで、
consumer が spec に宣言した `target`/`action` へ**実行時入力の任意値**を
型付きで乗せる経路がなかった。PPQN 1..=32767 のような範囲内自由入力は
`ButtonSpec` の推奨値列挙では表現できない。

## 目的

- `ComponentSpec` に任意値 text/number 入力 widget `Input` を追加し、
  wire 経由で typed な入力値を `Action` intent として送出できる。
- 範囲制約（min/max）と不正入力の拒否を wire 契約で表現する。
- 上記を headless test（`view.rs` `#[cfg(test)]`）で検証する。

対象外（Issue 本文と整合）: 実 GUI 受入（別 gate — synvelis#114 /
ruviform#631）、GPUI/webview adapter の render 実装、synvelis 側 op 配線
（synvelis PR #650 で統合済み）。

## wire 契約

### `ComponentSpec::Input`（`"type": "input"`）

```json
{
  "type": "input",
  "key": "ppqn",
  "label": "PPQN",
  "target": "syn.project",
  "action": "ppqn",
  "param": "new_ppqn",
  "value": { "kind": "integer", "min": 1, "max": 32767 },
  "visual": {},
  "a11y": {}
}
```

| field | 型 | 意味 |
| --- | --- | --- |
| `key` | string | Component key。command binding id 兼 receipt routing owner |
| `label` | string | 表示ラベル。a11y name の fallback |
| `target` | string | 発行 intent の `LogicalTarget`。空文字は `InvalidSpec` |
| `action` | string | 発行 intent の discrete action 名。空文字は `InvalidSpec` |
| `param` | string | 提出値を運ぶ `params` のメンバ名。空文字は `InvalidSpec` |
| `value` | object | 受理する入力 kind とその範囲（下表） |
| `visual` | object | `ComponentVisualState`（表示のみ、submit を止めない） |
| `a11y` | object | `A11ySpec`。`role` 既定は `value.kind` 由来 |

### `InputValueSpec`（`value` フィールド、`kind` タグ）

| `kind` | 追加 field | parse 規則 | 提出 `CommandParam` | a11y 既定 role |
| --- | --- | --- | --- | --- |
| `"text"` | なし | 入力文字列を verbatim で受理 | `Text` | `textBox` |
| `"integer"` | `min?`/`max?`（i64・inclusive・省略可） | trim 後 `i64` parse | `Integer` | `spinButton` |
| `"number"` | `min?`/`max?`（f64・inclusive・省略可・有限のみ） | trim 後 `f64` parse・有限必須 | `Number` | `spinButton` |

- `min`/`max` は独立省略可。省略側は非制限。両方あるとき `min <= max`
  を必須とする（mount 時 `InvalidSpec`）。`number` の bound に非有限値を
  含む spec も `InvalidSpec`。
- 範囲制約は **host 側でも submit 時に強制**される: 範囲外・parse 不能・
  非有限数値は `ViewError::InvalidInput` を返し **intent を emit しない**
  （`select_row`/`dispatch_action` の "rejected inputs emit nothing" 規則）。

### submit → intent（`ViewHost::submit_input`）

```rust
pub fn submit_input(
    &mut self,
    generation: u64,
    key: &ComponentKey,
    text: &str,
) -> Result<CommandId, ViewError>
```

- 入口規則は `invoke`/`select_row`/`dispatch_action` と同一:
  `StaleGeneration` → `UnknownComponent` → `InvalidInput`（key が input
  kind でない場合を含む）。
- 受理時: spec の `target`/`action`/`param` と parse 済み値から
  `CommandParams`（`{"<param>": <value>}`）を構成し、既存の
  `action_intent` → `emit_intent` 経路で `CommandIntent::Action` を mint。
  `expect_receipt = true`・non-droppable の既存 `Action` 規則に従う。
- `control_id` は component key、binding owner は
  `owner_key_for(key)`（Input の `action_binding_id` = key）— Button と
  同一の binding・receipt routing・`apply_receipt` phase 遷移
  （`Pending` → `Accepted` → `Applied`）を辿る。component の除去で
  routing は `release` 経由で drop される。

## 版数・後方互換

`rvf.ui.command.v1` を維持する（bump なし）。

- 本変更は `ComponentSpec` enum への新 variant 追加（新 `type` 値）であり、
  Issue #119 方針と #944 D3 と同じく v1 内 extension。既存 variant・
  `CommandIntent` の serde・`params` 語彙は不変。
- `ComponentSpec` は `#[non_exhaustive]` かつ下流 matcher は wildcard arm
  を持つため、旧 adapter は `"input"` spec を `"unsupported component"`
  表示へ落とす（GPUI `render.rs` の `other =>` arm）。新 kind の render
  実装は別 slice の責務であり、本 slice ではこの降格挙動を正直な仕様と
  して扱う。
- `A11yRole` に `textBox`/`spinButton` を追加（同 enum は
  `#[non_exhaustive]` で camelCase serde）。

## 受入条件との対応

|| Issue #999 受入 | 対応 |
| --- | --- | --- |
| AC1 | `ComponentSpec` に任意値 text/number 入力 widget が追加され、wire 経由で typed な入力値を intent として送出できる | `ComponentSpec::Input` + `InputValueSpec`（text/integer/number）+ `ViewHost::submit_input`。入力値は `CommandParams` canonical JSON で `Action.params` に載る |
| AC2 | 範囲制約（min/max）・不正入力の拒否が wire 契約で表現され、headless test で input widget → intent mint → op 到達・範囲外拒否が確認できる | `value` の `min`/`max` を mount 時に検証（`InvalidSpec`）、submit 時に強制（`InvalidInput`・emit なし）。`view::tests::input_*` が intent mint → `drain_events` 到達 → `apply_receipt` round-trip と範囲外/parse 拒否を確認（「op 到達」= intent が consumer への drain 経路へ出ること。op への mapping は consumer 責務） |
| AC3 | `mise run` の対象 lane が pass する | `evidence/rf999/verification.md` の lane 実走記録 |

## 非スコープ（未達として明示）

- GPUI/webview adapter の `Input` render・テキスト入力 plumbing
  （`render.rs` の element 実装・IME・focus）。別 slice。
- driver-lab parity fixture の `Input` ケース追加 — render adapter 側の
  実装と同じ slice で扱う（fixture は `crates/ruviform-ui-test/` で本
  slice の編集範囲外）。
- 実 GUI 受入（実 OS window・実描画・IME 実入力）— 別 gate
  （synvelis#114 / ruviform#631）。
- `Text` kind の max length・pattern 制約、`placeholder`、選択肢/補完
  （YAGNI — 必要になった時点で `value` の per-kind field として拡張可能）。
- synvelis 側 `syn.project.ppqn` op の admission/routing — consumer 責務
  （params を許す target の admission は #944 D7 の分界を継承する）。
