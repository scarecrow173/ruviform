---
id: "DESIGN-Hk9Qz4WsNt6XrLc1VbYe8M"
type: "design"
status: "approved"
title: "Issue #451 — Tabs pointer 切替と descriptor 更新の ViewSpec 契約の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-451-tabs-pointer-descriptor.md"
  implemented-by: []
  refines:
    - "docs/designs/issue-448-table-selection-command.md"
  depends-on:
    - "docs/designs/issue-448-table-selection-command.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-451-tabs-pointer-descriptor.md"
---

# Issue #451 — Tabs pointer 切替と descriptor 更新の ViewSpec 契約の設計

## 1. 概要

本設計は次の 2 点を定める。

1. Tabs ヘッダの pointer 操作を `CommandIntent::Action`（`action: "tab-select"`）として発行し、表示タブを `ViewHost` が保持する。#448 の Table 行選択と同じ構造に揃える。
2. mount 後の descriptor 更新は `ViewHost::update` を正式経路とする。remount は要求しない。進行中の receipt を失わないように、再登録時の routing の扱いを分ける。

## 2. 現状の観察（コード根拠）

| 観察 | 根拠 |
| --- | --- |
| Tabs の active は spec の `active` だけで決まり、host は表示状態を持たない | `view.rs` の `TabsSpec.active`、GPUI `element_for` の `t.active.min(..)` |
| Table 選択は `Mounted.selected` を楽観的に保持し、`selection_undo` で rejected を revert する | `view.rs` `select_row` / `settle_selection` |
| descriptor 変更は `reconcile_action` で `contract_changed` になり、control を unregister + register する | `view.rs` `update` の `Updated { contract_changed }` |
| 旧実装の `unregister_control` は `command_index` も削除していた。再登録（同一 key）でも in-flight receipt が routing を失う | `view.rs` 旧 `unregister_control`（修正前）。`descriptor_update_keeps_in_flight_receipt_routing` が修正前に失敗 |
| `mount` は mount 済み host では `update` に委譲する。GPUI の `update_view` も `host.update` を呼ぶ | `view.rs` `mount`、`ruviform-ui-gpui/src/lib.rs` `Op::UpdateView` |
| 範囲の更新は dispatcher の control（`ValueMapper`）に反映される | `view::tests::descriptor_update_widens_seek_range_without_remount` が修正前から pass |

## 3. 決定

### 3.1 Tabs の選択

- `TabsSpec` に `selection: Option<TabsSelectionSpec>` を追加する（`serde(default, skip_serializing_if)`）。`TabsSelectionSpec` は `target: LogicalTarget` を持つ。`TableSpec.selection` と同じ形で、選択しない Tabs は spec 駆動のまま（既存の挙動）。
- `TAB_SELECT_ACTION = "tab-select"`。params は `{"index": n}`。intent は `CommandIntent::Action { target, control_id: key, action, params }` で、receipt 契約は既存の `CommandBinding` に従う。
- `ViewHost::select_tab(generation, key, index)` を追加する。`select_row` と同じ検査順序（epoch → 存在 → selectable → 範囲）で、拒否は intent を出さずに `ViewError::InvalidInput` 等を返す。
- 表示タブは `Mounted.selected` に持つ。Tabs は常に表示タブを持つ（selection の有無に関わらず spec の `active` で初期化）。`ComponentSnapshot.selected` が表示タブを返す。
- 同一 key の update の採用規則: spec の `active` が前回から変わったら表示を新しい `active` にする。変わらなければ利用者が選んだ表示タブを保持する（範囲外になったら `active` に戻す）。
- `selection_undo` と `settle_selection` は Table と共有する。rejected / unsupported で直前のタブへ戻す。
- binding id は key（`action_binding_id`）。`release` で receipt routing を解放する。

#### 共通化

`select_row` と `select_tab` は `emit_selection(key, target, (action, params), index)` を共有する。previous の読み取り、intent の発行、表示更新、undo の登録の 4 手順を 1 箇所に置く。

### 3.2 GPUI 側

- タブヘッダの `Div` に `on_mouse_down` を付ける（selection を持つ場合だけ）。押下で `RvfView::select_tab` を呼び、`host.select_tab` の後に `flush_events` する。`select_row` と同じ形。
- 描画の表示タブは `displayed_tab(host, t)`（`host.observe(key).selected`、無ければ `active`）から読む。`element_for` と drawn plan の両方が同じ関数を使う。
- `drawn_plans` は spec ではなく `ViewHost` を受け取る。描画される木と drawn plan の木を一致させるため。
- `debug_selector("<key>-tab-<i>")` を付ける（test-support ではノーオペ。実窓の自動テストでヘッダの bounds を引くため）。

### 3.3 descriptor 更新

- **決定: descriptor 更新は `ViewHost::update`（mount 済み host への `mount` を含む）で行う。remount は要求しない。**
- 理由: `update` は generation を変えず、`AttachEpoch` と mount epoch を保ち、receipt の名前空間を変えない。remount は mount epoch を進め、既存の receipt を全て stale にする。project length の変化は表示の更新であり、命令の寿命を区切る必要はない。
- 修正: contract change の再登録では control の gesture だけを detach し、`command_index` は残す（`detach_control`）。key の削除（`release` / `unregister_control`）だけが routing を解放する。
- 不変: 再登録後の control は `observed`（最後に採用された projection）を再適用する（既存の `register_control` の挙動）。range が変わると、その値は新しい range の値として解釈される。
- gesture は contract change で cancel される（既存の detach policy どおり）。これは既存の挙動であり、本 Issue では変えない。

### 3.4 検討して退けた案

| 案 | 退けた理由 |
| --- | --- |
| remount を正式経路にする（key を作り直す） | mount epoch が進み、進行中の command の receipt がすべて stale になる。Seek の range 変更で命令の寿命を区切る理由がない |
| Tabs の表示を spec だけで決め、consumer の再 push を待つ | pointer 切替が見えない。#448 の楽観的表示と揃わない。受入条件 V2 を満たせない |
| `TabsSpec.selection` を必須にする | 既存の Synvelis の Tabs（操作しない表示切替）を壊す。opt-in とした（Table と同じ） |
| Tabs のタブ key を intent の params に載せる | 表示状態は index で持つ（`TabsSpec.active` と同じ）。key による identity は YAGNI |

## 4. 境界と不変条件

- `view.rs` に GPUI 型を出さない。描画の観測と pointer 入力は `ruviform-ui-gpui` 内で完結させる。
- 選択は index で表す。範囲外は拒否し、intent を出さない。
- 表示タブは `ComponentSnapshot.selected` 経由でのみ観測する。`TabsSpec.active` は初期値と spec 更新の authority であり、描画の唯一の source ではない。

## 5. 検証の方針

- `view::tests`: タブ選択の発行、拒否、revert、update の採用規則、descriptor 更新での range 反映と receipt 維持。
- `driver_parity` / `consumer_observation`: 同じ `selection-command.json` から Fake と GPUI の Action intent・snapshot・drawn plan を検査する。
- GPUI: `tab_header_press_selects_the_tab_through_the_pointer`（TestAppContext の mouse-down）と、実窓での手動証拠（Windows の `component_showcase`）。

## 6. 影響範囲

- `TabsSpec` に `selection` フィールドを追加する（`serde(default)` で既存 JSON は読める）。Rust の struct literal は `selection` を指定する必要がある（workspace 内 5 箇所を更新済み）。
- `ComponentSnapshot.selected` の意味を、Table の行に加えて Tabs の表示タブへ広げる（doc comment を更新）。
- `drawn_plans` の引数を spec から `ViewHost` へ変える（`gpui-test` feature 限定の crate 内関数）。
