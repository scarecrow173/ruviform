---
id: "DESIGN-pL8wRtYbN3xKcVhJ5mQs2E"
type: "design"
status: "approved"
title: "Issue #414 — projection→controlled display 正規経路の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/414"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-414-projection-controlled-display.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-414-projection-controlled-display.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-387-component-view-specs.md"
    - "docs/designs/viewlink-authoritative-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-414-projection-controlled-display.md"
  derived-by:
    - "docs/plans/issue-414-projection-controlled-display.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-414-projection-controlled-display.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #414 — projection→controlled display 正規経路の設計

## 三状態の区別（正規化）

本 Issue で明文化する表示契約。3 つの状態は別物であり、混同しない。

| 状態 | 保持場所 | 意味 |
| --- | --- | --- |
| **Observed** | `CommandBinding.last_observed`（`StateProjection`） | consumer が authoritative に push した domain truth。revision guard で stale を排除。`CommandPhase::Observed` はこれが存在することを示す。domain state が変わったことの証明。 |
| **Applied** | `CommandBinding` の receipt ledger（`last_receipt` / head record rank） | gateway が報告した command 進捗（`accepted`/`queued`/`applied`/terminal）。domain 変更の報告であり、値そのものを運ばない。`Applied` ≠ `Observed`。 |
| **Controlled Display** | `InteractionControl.committed_value` / gesture preview（`value()`） | renderer が実際に描く値。gesture 中は local preview、idle 時は committed。 |

`ViewHost::observe` の `ComponentSnapshot` はこの区別をそのまま
写す：`display`/`committed` = Controlled Display、`phase`/`observed`
= Applied/Observed。

## 欠陥の根本原因

`StateProjection` → `last_observed` の経路のみ存在し、
Observed → Controlled Display の辺が存在しなかった。
`apply_receipt` も ledger のみ更新する（Applied は値を運ばない
ため display を更新しないのは正しい）。

## 正規経路

```
wire Notify {control_id, revision, value, provenance?}
  → drain_inbox が StateProjection<serde_json::Value> を構築
  → ViewHost::apply_projection(control_id, projection)
      → CommandBinding::on_projection（attachment + revision guard、
        変更なし — accepted のみが先へ進む）
      → projection_value_input(&projection.value)   … decode
      → InteractionControl::apply_external(input)    … write
          ├─ gesture active → pending_external に stage
          │                    （preview が display を保持）
          └─ idle           → committed_value に即時 commit
      → end_gesture が pending_external を committed_value へ flush
  → ViewHost::observe().display = InteractionControl::value()
  → renderer は snapshot.display.normalized を描画
```

### payload decode（`projection_value_input`）

業務 JSON を無条件に `f64` 解釈しないための明示契約：

- 有限 JSON number → `ValueInput::Plain`（product-value domain。
  参照実装の `value` property と同じ domain）
- `{"plain": <有限数>}` → `ValueInput::Plain`
- `{"normalized": <有限数>}` → `ValueInput::Normalized`
- それ以外（文字列・null・複数キー object・非有限数）→ `None`

`None` の場合も binding は projection を `observed` として記録する
（truth は truth）が、display は推測で更新しない。

### `InteractionControl::apply_external`

- `ValueInput::Plain`/`Normalized` の絶対値のみ受理
  （`Steps`/`Extreme` は絶対 truth として意味を持たない → `None`）。
- 解決は user input と同一の `ValueMapper` 経路
  （range policy・step snap・domain mapping）を通るため、
  descriptor が user 入力に課す制約を projection も破れない。
- gesture active 中は `pending_external` に stage。`end_gesture`
  （commit/cancel/detach/focus-lost/availability-cancel の全終了
  経路が集約される一点）で `committed_value` へ flush される。
  commit/detach-commit は preview を committed に代入してから
  `end_gesture` を呼ぶため、staged truth が一律に最終値になる
  （参照実装の「event 処理後に pendingExternal_ を flush」と同じ
  順序）。
- event を一切 emit せず input seq も消費しない — domain truth の
  反映であり consumer への command ではないため、controlled-write
  loop は閉じたまま。

### 再登録時の復元

`ViewHost::register_control` で binding が生き残る場合
（descriptor 変更による unregister+register、update での
remove+re-add）、binding の `last_observed` を新しい control の
mapper 経由で `apply_external` し、display を最後の observed
truth に復元する。次の projection を待たずに表示が default に
張り付くことを防ぐ。

## 保持する既存契約

- stale/revision guard：`on_projection` が false を返す
  projection は display に一切触れない。
- detach：binding は `stale` となり projection を吸収。
  `detach_all` が gesture を終了させる際に staged truth は flush
  される（display は最後の observed を示す）。
- reattach：次の advancing projection が通常通り display を更新。
- receipt：`apply_receipt` は ledger のみ — Applied は値を運ば
  ないため display を更新しない（Observed のみが display truth）。
- availability：`apply_external` は availability を見ない —
  disabled/read-only control も authoritative state の表示は
  追従する（user 入力の gate であり truth 反映の gate ではない）。

## 却下した代替案

- **`ControlInput::Set`（atomic pair）の再利用**：begin+commit
  event が emit され `intent_from_gesture` 経由で intent が mint
  されるため、consumer への phantom command ループを生む。
  event 抑制を別途入れるより、event を出さない専用 write 経路の
  方が境界として明確。
- **`CommandBinding` 内での display 更新**：binding は
  `InteractionControl` を所有しない（ViewHost が両者を対で持つ）
  ため、binding に mapper/display 責務を持たせると層が崩れる。
- **`pending_external` を ViewHost 側に持つ**：gesture 状態機械と
  `committed_value` は control が所有するため、stage/flush も
  control 内に置く方が不変条件（pending ⟹ gesture active）を
  一箇所で保てる。
