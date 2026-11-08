---
id: "DESIGN-Hm4Vq8ZsLw2KxTn6RcYb1G"
type: "design"
status: "approved"
title: "Issue #451 FU-1 — Tabs のキーボード切替の設計"
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
    - "docs/specs/issue-451-keyboard-tab-nav.md"
  depends-on:
    - "docs/specs/issue-451-keyboard-tab-nav.md"
    - "docs/designs/issue-451-tabs-pointer-descriptor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-448-table-selection-command.md"
---

# Issue #451 FU-1 — Tabs のキーボード切替の設計

## 1. 結論

キーの解釈は GPUI の描画層（`ruviform-ui-gpui`）に置き、タブの選択は既存の `ViewHost::select_tab` を呼ぶ。契約（`ruviform-ui`）は変更しない。

```
key down (focused tab header)
  -> RvfView key listener          [ruviform-ui-gpui: render.rs]
       -> tab_key_target(key, displayed, len)   # key name -> next index
       -> RvfView::select_tab(key, next)        # same path as the pointer
            -> ViewHost::select_tab              # ruviform-ui (unchanged)
                 -> tab-select Action intent / receipt / snapshot
```

## 2. 責務の配置

| 責務 | 置き場所 | 理由 |
| --- | --- | --- |
| GPUI の key 名（`"left"`、`"right"`）の解釈 | `ruviform-ui-gpui/src/render.rs` | key 入力の正規化は renderer の責務（既存の pointer 正規化と同じ層） |
| 移動先の規則（循環、Home/End、移動なしの扱い） | `render.rs`（`tab_key_target`） | 規則は GPUI のキー割当と一体で、他 driver はまだ key 入力を持たない（YAGNI） |
| 選択・intent・receipt・revert | `ruviform-ui`（既存） | driver-neutral な唯一の実装。変更しない |
| フォーカスの保持 | `RvfView::tab_focus`（strip ごとの `FocusHandle`） | GPUI のフォーカスは view 単位で持つ必要がある |
| テスト用の key 注入 | `GpuiObserver::press_tab_key`（seam） | 直接 `select_tab` を呼ぶと実フォーカスと実 key dispatch を通らないため |

## 3. 代替案と却下理由

| 案 | 内容 | 判定 |
| --- | --- | --- |
| A | 契約に key 入力 API（`ViewHost::press_tab_key(name)`）を足す | 却下。契約に GPUI の key 名語彙が入り、driver-neutral を崩す。他 driver の入力要件も未確定 |
| B | 移動規則を `ruviform-ui` に置く | 却下（現時点）。key 名の共通語彙がなく、消費者もない。webview が key 入力を持つ時点で昇格を検討する |
| C（採用） | 描画層で key を解釈し、既存の `select_tab` を呼ぶ | 採用。契約変更なし。pointer と同じ経路で receipt と revert が効く |

## 4. フォーカスの扱い

- `RvfView` は `tab_focus: BTreeMap<ComponentKey, FocusHandle>` を持つ。strip のフォーカスハンドルは、その strip を初めて描画するときに作る。
- `selection` を持つ Tabs のヘッダだけに `track_focus` と `on_key_down` を付ける。`selection` のない Tabs は従来どおり、フォーカスも key 処理も持たない。
- ヘッダのタブ要素の mouse-down は、ヘッダのフォーカスを移してから選択する（`window.focus` → `select_tab`）。
- 既知の制約: strip のハンドルは unmount 時に解放せず、同じ view 内で一度でも描画された selectable Tabs の数だけ残る。数は spec に依存して有界であり、現時点では対処しない。

## 5. キーの処理

- 処理するのは `left` / `right` / `home` / `end` のみ。`ev.keystroke.modifiers` に修飾キーがあれば無視する。
- 移動先は**表示中のタブ**（host の selection。spec の `active` は初期値）から求める。listener は event 時点の host を読むため、`displayed_tab` は `(host, key, active)` を引数に取る。
- 移動先が表示中のタブと同じ（1 タブのみ、または端での Home/End）なら、`select_tab` を呼ばず intent も出さない。
- `len == 0` の strip は移動しない（`% len` と `len - 1` の保護）。

## 6. 検証の設計

- 規則: `tab_key_target` を純関数として unit test する（循環、Home/End、移動なし、未知の key、タブ 0 / 1 個）。
- 実経路: gpui-test で実フォーカスと `simulate_keystrokes` を使う。フォーカス前の key は無視され、ヘッダ押下後の key が `tab-select` intent を正しい index で出し、snapshot が追従することを確かめる。
- 共有 fixture: `selection-command.json` の `expect.tabs.keyboard` を、Fake 側（契約の intent と snapshot）と GPUI seam（key dispatch 経由の intent と snapshot）の両方が検査する。Fake 側は key 入力を持たないため、契約の intent 形状の一致を検査する。key から index への規則は GPUI 側の検査が担う。

## 7. 既知の未解決事項（本設計の外）

- GPUI の Tab 循環はホストアプリの `focus_next` バインドに依存する（GPUI 0.2.2 は既定で持たない）。
- Seek / Fader / Knob のキーボード操作は未対応。共通の key 規約を決めてから別 Issue で扱う。
- 実窓でのキー操作の手動証拠は未取得（K3）。
