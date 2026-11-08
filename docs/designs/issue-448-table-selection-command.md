---
id: "DESIGN-Hn4Wz8QeKs1TcVb6LrYg2D"
type: "design"
status: "approved"
title: "Issue #448 — 行選択と command 起動の ViewSpec 契約の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/448"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-448-table-selection-command.md"
---

# Issue #448 — 設計

## 1. 設計判断の要旨

既存の intent 語彙を再利用し、新しい入力経路と spec 項目だけを足す。

- **行選択は `Table` の任意機能**とする（新 kind にしない）。#435 の D2 で List は一列の Table と決めており、選択を別 kind にすると a11y 意味論が二重になる。
- **command 起動は `Button` という新 kind** とする。値を持たない操作は `ControlSpec`（連続値・二値）に載らないため、control とは別 kind が自然。
- **intent は既存の `CommandIntent::Action`** を使う。語彙は #257 以前から discrete action 用に存在し、`CommandKind::Action` と `expect_receipt` の扱いも `CommandBinding` に実装済み。port の型は変えず、`params` だけ追加する。
- 行選択を `CommandIntent` の新 variant（`Select`）にしないのは、receipt・phase・generation 管理が `Action` で既に成立しているため（YAGNI / DRY）。

## 2. 契約（`ruviform-ui`）

### 2.1 型

| 型 | 役割 |
| --- | --- |
| `TableSelectionSpec { target: LogicalTarget }` | `TableSpec.selection` の値。`Some` のとき行選択可能。`target` は選択 intent の論理 target |
| `TableSpec.selection: Option<TableSelectionSpec>` | `#[serde(default)]`。既定 `None` は読み取り専用（#435 と同じ挙動） |
| `ButtonSpec { key, label, target, action, params, visual, a11y }` | 値を伴わない操作。`label` は表示文字列、`target`・`action`・`params` は発行する intent の内容 |
| `ComponentSpec::Button(ButtonSpec)` | serde tag は `button`。`kind()` は `"button"` |
| `A11yRole::Button` | Button の role。a11y 木の name は `a11y.label`、無ければ `label` |
| `ComponentSnapshot.selected: Option<usize>` | 選択可能 Table の表示上の選択行。それ以外は `None` |
| `ViewError::InvalidInput { key, detail }` | 対象 kind でない・非選択可能・行範囲外の入力 |
| `CommandIntent::Action.params: Option<serde_json::Value>` | `#[serde(default, skip_serializing_if = "Option::is_none")]`。既存の出力 JSON は不変 |

定数: `TABLE_SELECT_ACTION = "select"`（選択 intent の action 名）。選択 intent の `params` は `{"row": <index>}`。

### 2.2 検証（mount/update 時）

- `ButtonSpec.target` と `ButtonSpec.action` は空文字を拒否する（`InvalidSpec`）。
- `TableSelectionSpec.target` は空文字を拒否する（`InvalidSpec`）。
- 既存の Table の検証（行長・上限・非有限数）は変更しない。

### 2.3 入力 API（`ViewHost`）

```rust
pub fn select_row(&mut self, generation: u64, key: &ComponentKey, row: usize) -> Result<(), ViewError>;
pub fn invoke(&mut self, generation: u64, key: &ComponentKey) -> Result<(), ViewError>;
```

- どちらも `check_gen` で世代を検査する（stale は `StaleGeneration`）。
- `select_row`: key が未 mount なら `UnknownComponent`。Table でない・`selection` が `None`・`row >= rows.len()` は `InvalidInput`。成功時は `Mounted.selected = Some(row)` にし、`Action { target, control_id: key, action: "select", params: {row} }` を発行する。
- `invoke`: key が未 mount なら `UnknownComponent`。Button でなければ `InvalidInput`。成功時は `Action { target, control_id: key, action, params }` を発行する。
- `Action` の `sequence` は host 単位の単調増加カウンタ（`action_sequence`）。`origin_view` は既存 gesture と同じ `view-<id>`、`source` は `None`。

### 2.4 binding

- Button と選択可能 Table は control を持たないが、command binding を持つ。binding id は component key（`ComponentKey.0`）。
- 既存の `emit_gesture` の intent 部分を `emit_intent(control_id, intent) -> CommandId` に切り出し、gesture と action の両方が使う（DRY）。
- binding の namespace は既存の `binding_namespace(control_id)` を使う（attach epoch と mount epoch を含む）。
- `observe(key)` の `phase` / `observed` は、control の binding id または action の binding id から引く。

### 2.5 選択表示と receipt

- `select_row` は表示上の選択を即時に `row` へ変える（楽観的表示。Toggle の preview と同じく、phase が進捗を示す）。
- 発行した command の直前の選択を `selection_undo: HashMap<CommandId, (ComponentKey, Option<usize>)>` に保持する。
- `apply_receipt` で `Rejected` / `Unsupported` を受けたら、対応する選択を直前の値へ戻し、エントリを消す。`Applied` を受けたらエントリだけ消す（表示は変えない）。`Accepted` / `Queued` は保持する。
- 表示上の選択は必ず `clamp_selection` を通す。selectable な Table で、現在の行数の範囲内の行だけを保持する。直前の値へ戻すときも同じ検査を通るため、その間に行数が減っていれば範囲外の行は復元されない。
- `detach` と `unmount` で `selection_undo` を消す（stale な receipt は index で既に弾かれる）。
- phase は `CommandBinding` の既存遷移に任せる（`Action` は `expect_receipt = true` のため `Pending` から始まる）。

### 2.6 remove / update の扱い

- Button と選択可能 Table の削除時は、その binding id の `command_index` エントリを消す（control の `unregister_control` と同じ扱い）。
- Table の更新時は `selected` を新しい行数で切り詰める（範囲外になったら `None`）。
- Button と Table の更新は `contract_changed = false`（control の contract を持たないため）。binding は保持する。

## 3. Fake 側（ViewHost）

- mount/update は既存の reconcile に乗る。Button と選択可能 Table は `register_control` を呼ばず、binding だけを `ensure_binding` で用意する。
- `a11y_node` は Button に対して `Some` を返す（role `button`、name は `a11y.label` か `label`、value なし）。Table は #435 の木を変えない。

## 4. GPUI 側（renderer）

- `button_element(&mut self, spec, cx)`: `label` の div に `on_mouse_down(Left)` を付け、`RvfView::invoke(key)` を呼ぶ。
- `table_element` は行ごとに `on_mouse_down` を付ける（選択可能時のみ）。押された行を `RvfView::select_row(key, row)` で選ぶ。選択行は `palette.accent` の下線で強調する。
- `RvfView` の入力は既存の `input()` と同じく「host を呼ぶ → `flush_events`」で、`ViewEvent::Intent` を `intent_handler` へ渡す。
- `drawn_plans` には Button の描画を `DrawPlan::Button(label)` として追加し、`DrawnPlan::Button { label }` に写す。Table の drawn plan は変えない（選択は snapshot で観測する）。
- `GpuiObserver::invoke(window, key)` / `select_row(window, key, row)` をテスト用 seam として追加する（`dispatch_input` と同じ形）。

## 5. webview 側

- `ruviform-ui-webview` は `ViewSpec` の描画経路を持たない。よって本 PR では webview の component 実装を行わない。
- 契約は driver-neutral に定義する（`ViewHost` の戻り値と `ViewEvent::Intent` は driver の型を含まない）ため、webview が後から同じ intent を発行できる。
- webview の parity 検査は NotRun として `cases.json` に記録する（Pass にしない）。

## 6. 検証設計

| 層 | 検証 | 場所 |
| --- | --- | --- |
| contract | 選択 intent の発行・範囲外/非選択 Table の拒否・stale 世代・Button の intent・phase 遷移（pending → accepted → applied / rejected） | `ruviform-ui` の `view::tests` |
| contract | receipt rejected で選択が直前へ戻る・applied で表示が変わらない・remove で index が消える・Table の更新で selected が切り詰められる | `ruviform-ui` の `view::tests` |
| fake | parity fixture の mount/reconcile、選択 intent・Button intent の kind/action/target/params、snapshot.selected と phase | `ruviform-ui-test/tests/driver_parity.rs` |
| GPUI | Button と Table 行の押下が intent を発行し、snapshot が fake と一致する。drawn Button label | `ruviform-ui-gpui/tests/consumer_observation.rs` |
| boundary | 共通層に GPUI 型が出ていない | `check-ui-driver-boundary` |

## 7. 互換性

- `ComponentSpec` / `A11yRole` / `ViewError` は `non_exhaustive` のため variant 追加は非破壊。
- `TableSpec.selection` は `serde(default)` のため、既存の Table JSON（#435 の `label-table.json` を含む）はそのまま読める。
- `CommandIntent::Action.params` は省略時に出力しないため、既存の intent JSON は変わらない。Action はこれまでどこからも発行されていないため、既存 consumer への影響はない。
- 既存 kind の挙動・payload・intent は変更しない。
