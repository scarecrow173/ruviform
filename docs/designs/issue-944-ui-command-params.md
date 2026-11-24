---
id: "DESIGN-944uiCommandParams01"
type: "design"
status: "draft"
title: "Issue #944 rvf.ui.command.v1 user-entered params wire 契約"
created: "2026-11-24"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/944"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-944-ui-command-params.md"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/ui_command_params.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-630-editor-verbs.md"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/synvelis/issues/125"
    - "https://github.com/scarecrow173/synvelis/issues/89"
    - "https://github.com/scarecrow173/synvelis/issues/623"
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
    number: 944
    url: "https://github.com/scarecrow173/ruviform/issues/944"
---

# Issue #944 rvf.ui.command.v1 user-entered params wire 契約

## 目的と範囲

`rvf.ui.command.v1`（rvf.ipc.v1 の `Intent` に乗る JSON op、`COMMAND_INTENT_OP`
= `crates/ruviform-ui-gpui/src/render.rs:47`）に、widget が実行時に生成する
user-entered params（数値・文字列・列挙・座標の基本型）を運ぶ型付き契約を
定義する。本書は payload schema・版数方針・consumer admission との責務分界・
receipt/coalesce 意味の設計判断を記録する。

本 slice の scope は次の 3 点である。

1. `Action` intent の `params` フィールドに載る user-entered params の型付き
   payload schema（`CommandParam` / `CommandParams`）の定義。
2. `ViewHost` の params 付き `Action` intent mint 経路（`dispatch_action`）の
   実装。
3. host ↔ widget の params 往復を検証する headless contract test。

対象外は Issue どおり、consumer（synvelis・audio-lens）の admission/routing
変更、`rvf.ui.command.v1` 以外の wire の params 化、実 GUI widget の描画と
実 GUI 受入（consumer Issue 側の gate）。

## 現状整理

確定済みの事実を設計の足場として置く。

- wire の実体: `rvf.ui.command.v1` は op 名であり、envelope は rvf.ipc.v1 の
  `Intent{v=1, kind="intent", id, op, params}`（`crates/ruviform-ui/src/ipc.rs`
  :29,107-137）。`RvfView::flush_events`（`render.rs:741-768`）が
  `ViewEvent::Intent{intent, command_id, expect_receipt}` を
  `{"command_id","expect_receipt","intent": <CommandIntent の serde>}` に包み
  `Intent::new(command_id, COMMAND_INTENT_OP, params)` で送出する（`:749-754`）
  —— intent 本体は serde 透過であり renderer は payload を解釈しない。
- payload schema: `CommandIntent`（`crates/ruviform-ui/src/behavior.rs:2403-2500`、
  `#[serde(tag = "kind")]`）の `Action` variant は
  `params: Option<serde_json::Value>`（`:2498`）を wire 上に既に持つ。
  `skip_serializing_if` により `None` ではキー自体が出ない。
- mint 経路の現状: `ViewHost::invoke`（`view.rs:2129`、Button の spec 静的
  params）、`select_row`（`{"row": n}`、`:2044`）、`select_tab`
  （`{"index": n}`、`:2088`）の固定形のみ。実行時生成の params を型付きで
  乗せる API がない。synvelis 側も `docs/specs/tim-01-ppqn-ui.md:137` で
  「shell の Action 経路は params を渡せない」と確認済み。
- host→widget 向き: `deliver_event`（`view.rs:2327`）の `Notify` →
  `StateProjection` → `apply_projection`（`:2289`）は
  `value: serde_json::Value` で任意 payload を既に運ぶ。receipt は
  `Event::Result` → `CommandReceipt::from_result` → `apply_receipt`（`:2209`）。
- receipt/coalesce 意味: `CommandBinding::dispatch`（`behavior.rs:2895-2961`）は
  `Commit`/`Action` に `expect_receipt = true` を与える。`is_droppable`
  （`:2519`）が true を返すのは `Preview` のみ（latest-wins coalesce 対象）。
- consumer 側の decoder（先例・境界）: synvelis `decode_view_command`
  （`synvelis-app/src/binding.rs:948-1035`）は `deny_unknown_fields` の
  envelope（`command_id`/`expect_receipt`/`intent`）と per-kind の
  `deny_unknown_fields` な body struct で strict decode する。`Action` の
  `params` は `Option<Value>` として受理し、params を許す target を
  `view`・`synvelis.library`・`autx.*` に限定して、非許可 target の params は
  decode で reject する（`:994-1007`）。未知の `kind` は
  `"unsupported intent kind"` で正直に reject（`:1026`）。
- 非回帰の固定点: params を持たない既存 intent の serde 出力は driver-lab
  parity fixture（`crates/ruviform-ui-test/fixtures/driver-lab/parity/*.json`
  の `expect.intents`、検証は `consumer_observation.rs:387-447`）と
  `component_view.rs` の既存ケースで固定されている。

## 決定事項

### D1: params 契約の形 — `Action.params` 内の型付き profile（選択肢 i を採用）

user-entered params は、既存 `Action` variant の `params` フィールドに載る
**型付き profile** として定義する。`Action.params` の wire 型
（`Option<serde_json::Value>`）は変更せず、新しい mint 経路が生成する payload
の形を `CommandParam` / `CommandParams` で拘束する。

検討した選択肢と棄却理由:

| 選択肢 | 判定 | 理由 |
| --- | --- | --- |
| (i) `Action.params` の型付き profile + 新 mint API | 採用 | wire 文法が不変で旧 consumer がそのまま decode できる。`params` は v1 の文法に既にあるフィールドで、中身は常に opaque `Value`（admission 対象）だった |
| (ii) 新 `CommandIntent` variant | 棄却 | consumer decoder は strict kind match —— 新 `kind` は旧 consumer で `"unsupported intent kind"` の decode reject になり、`rvf.ui.command.v2` への op 名 bump を要する非互換変更になる。本 slice の要件（基本型の運搬）は既存 field で完結し、bump の対価に見合わない |
| (iii) 新 `ComponentSpec` kind（入力 widget） | 棄却 | render 側の描画 + input→intent plumbing が必要で、driver-lab parity fixture の更新が lease 外になる。wire 契約 slice の範囲を超える製品機能であり、後続 Issue の検討対象として分離する |

### D2: payload schema — `CommandParam` / `CommandParams`

`CommandParam` は user-entered params が取りうる基本型の和、
`CommandParams` は `BTreeMap<String, CommandParam>` の newtype で、
`Action.params` に載る canonical JSON object を構成する。

| `CommandParam` variant | wire JSON | 用途 |
| --- | --- | --- |
| `Integer(i64)` | JSON 整数 | PPQN・行番号・カウント等の離散値 |
| `Number(f64)` | JSON 数値（有限のみ） | 連続量・比率 |
| `Text(String)` | JSON 文字列 | 自由入力文字列 |
| `Enum(String)` | JSON 文字列（variant 名） | 閉集合からの選択値 |
| `Bool(bool)` | JSON bool | confirm/toggle 系の二値 |
| `Point { x: f64, y: f64 }` | `{"x": <数値>, "y": <数値>}` | 座標・XY 系入力（有限のみ） |

canonical codec の規則:

- encode: `CommandParam::to_value()` / `CommandParams::to_value()` は
  `Option<serde_json::Value>` を返し、非有限数値（NaN・±inf）を含む場合は
  `None` —— wire に `null` や欠落キーを silent に出さない。`Integer` は整数
  表現（`960`）を、`Number` は浮動小数表現（`0.5`）を保つ。
- decode: `CommandParam::from_value()` / `CommandParams::from_value()` は
  canonical 形のみ受理する。JSON 整数（i64 範囲）→ `Integer`、それ以外の
  有限 JSON 数値 → `Number`、文字列 → `Text`、bool → `Bool`、
  `{"x","y"}` ちょうど 2 キーのオブジェクト → `Point`。i64 を超える u64
  整数は `Number` 側に倒れる。それ以外（null・配列・`{"x","y"}` 以外の
  キーを持つオブジェクト・さらに深い入れ子）は `None` —— 拒否。
- `Enum` の非対称: wire 上 `Enum` は `Text` と同一の JSON 文字列であり、
  canonical decode は文字列を `Text` に写す（`Enum("x")` → `"x"` →
  `Text("x")`、round-trip で `Enum` には戻らない）。variant 集合への所属
  検証は action ごとの schema を持つ consumer 側 admission の責務であり、
  wire は「列挙値は JSON 文字列で運ぶ」ことだけを規定する。
- `CommandParams` は `BTreeMap` なので wire 上のキー順は辞書順に決定的であり、
  golden/parity の比較対象として安定する。`Some(空)` は `"params": {}` を
  出力し、`None`（`Action.params` 自体が無い）とは区別される。

spec 宣言 params（`ButtonSpec.params`、consumer が spec に書く任意 JSON）は
この profile の対象外であり、`serde_json::Value` のまま残す —— 型付き
profile は「user-entered mint 経路が生成する payload」を規定するもので、
consumer 定義 payload を狭めない。

### D3: 版数方針 — `rvf.ui.command.v1` を維持（bump なし）

この wire の版数 token は op 名の `.v1` 接尾辞である（`WIRE_VERSION` const は
存在しない）。Issue #119 確定方針（`docs/specs/issue-119-l1-wire-version-policy.md`
の「最新の単一バージョンのみ・非互換変更時にのみ版数を更新」）に照らし、
本変更は **v1 内 extension** とする。根拠は 2 点ある。

1. 文法非変更: envelope キー・`kind` 集合・`Action` のフィールド集合は
   何一つ変わらない。`params` の中身を型付き profile にするのは「既存
   フィールドの値域に対する mint 側の規約」であり、decoder が照合する
   wire 文法自体の変更ではない。
2. strict decoder との整合: synvelis `decode_view_command` の
   `deny_unknown_fields` は `params` を `Option<Value>` として受理するため、
   profile 適合の params を含む新 intent を旧 consumer は文法どおり decode
   できる。受理可否は target admission（consumer 側ポリシー）の問題であって、
   wire 版数の問題ではない。逆に (ii) の新 `kind` / Action への新 field を
   取った場合は `deny_unknown_fields` / kind match で正直に reject され、
   op 名 bump が必須になる —— 採用しない (i) と相俟って v1 維持が成立する。

bump 基準（この wire への適用）: 新 `kind` variant の追加、既存 variant への
field 追加・削除・rename、envelope キーの変更、`expect_receipt`/
coalesce 意味の変更があった時点で `rvf.ui.command.v2`（新 op 名）への
切替を検討する。payload profile の拡張（新しい `CommandParam` variant の
追加等）は、旧 consumer がその値をどう解釈するかが action schema の問題で
ある限り v1 内の変更として扱い、本書の変更履歴に記録する。

なお `docs/designs/issue-630-editor-verbs.md` が計画する
`Command::SetGuiFocusTarget` / `SetGuiPopup` と `WIRE_VERSION` 7→8 は
`crates/ruviform-plugin-core/src/wire.rs` の L1 plugin バイナリ wire の話であり、
`rvf.ui.command.v1` とは別 wire である。両者の版数管理は独立しており、
本設計の v1 維持判断は L1 wire の `WIRE_VERSION` とは無関係である。

### D4: mint 経路 — `ViewHost::dispatch_action`

```rust
pub fn dispatch_action(
    &mut self,
    generation: u64,
    key: &ComponentKey,
    target: LogicalTarget,
    action: &str,
    params: Option<CommandParams>,
) -> Result<CommandId, ViewError>
```

- `generation` ガード（`StaleGeneration`）と mounted 検査
  （`UnknownComponent`）は `invoke`/`select_row` 等と同じ入口規則。
- intent の `control_id` は component key、binding owner は
  `owner_key_for(key)`（spec 宣言の binding id、無ければ key）——
  `emit_intent` が binding を遅延生成するため、どの component kind でも
  mount されていれば mint できる。component kind による可否判定は
  設けない（target/action の admission は consumer 責務 —— D7）。
- `params` は `CommandParams::to_value()` で canonical JSON に変換し、
  非有限数値を含む場合は `ViewError::InvalidInput` で何も emit しない
  （`select_row` の範囲外 row と同じ「rejected inputs emit nothing」規則）。
- 戻り値は minted `CommandId` —— 呼び出し側（driver/contract test）が
  receipt を相関するための追跡 token。
- `emit_selection`/`invoke` 系の既存 mint 経路は変更しない。Button の
  「spec 静的 params + runtime の user-entered params を併せたい」ケースは
  renderer/driver が spec の `target`/`action` を読んで `dispatch_action`
  に渡せば足りる（params の merge 意味は導入しない —— YAGNI）。

### D5: receipt / coalesce 意味 — `Action` の既存規則を踏襲

params を運ぶ `Action` intent は `CommandIntent::Action` のままなので、
`expect_receipt = true`（`Commit`/`Action` 規則）・non-droppable
（`is_droppable` = false）の意味は変わらない。`Preview` のような
latest-wins coalesce 対象にはしない —— user-entered params は confirm
ボタンの submit 値のように離散的で、直前の入力を捨ててよい性質ではない。

### D6: host→widget 向き — 既存 `StateProjection<serde_json::Value>` を再利用

host→widget の params 投影は新規契約を作らず、既存の `Notify` →
`StateProjection` → `apply_projection` 経路で運ぶ。projection の `value`
は consumer 定義の opaque JSON であり、params 形の payload をそのまま載せ
られる（`deliver_event` は `control_id`/`revision`/`value`/`provenance` の
み読み、`value` の中身を解釈しない）。typed params の投影契約は
consumer ごとの payload schema が確定してからの検討事項とし、本 slice では
導入しない。

### D7: 責務分界 — admission は consumer 側、wire は運搬に徹する

どの target/action が params を許すかの admission は consumer 側責務であり、
ruviform 側に admission 相当ロジックを複製しない。根拠は synvelis
`decode_view_command` の先例 —— `Action.params` は target 許可リスト
（`view`・`synvelis.library`・`autx.*`）で admit され、非許可 target の
params は decode で reject される。ruviform 側の `dispatch_action` は
target の正当性を判断せず、型付き profile に適合する payload を正直に
運ぶことだけを保証する。

### D8: renderer（`render.rs`）は非変更

`flush_events` は任意の `CommandIntent` を serde 透過で `params.intent` に
入れるため、`Action.params` に載る新 profile は既存経路でそのまま wire に
出る。variant 追加でも field 追加でもないので renderer 側の変更は不要。
widget の描画側が runtime params を組み立てて `dispatch_action` を呼ぶ
plumbing は、params を生む component kind が実装される時点の後続 Issue で
設計する。

## 非回帰条件

- params を持たない既存 intent の serde 出力は不変 —— `Action` variant への
  変更はなく、`params` field も `Option<serde_json::Value>` のまま。parity
  fixture（`driver-lab/parity/*.json`）と `component_view.rs` の既存
  ケースがそのまま pass することが直接証拠になる。
- `dispatch_action` は既存 mint 経路（`invoke`/`select_row`/`select_tab`）を
  置き換えず追加する。既存経路の emit する intent 形状・receipt 追跡・
  selection undo 意味は不変。
- `CommandBinding::dispatch` の `expect_receipt`/`is_droppable` 規則は変更
  しない —— 新しい intent kind を増やさないため、receipt routing と
  pending record の寿命意味は既存規則どおり。

## wire 変更履歴（版数 token 記録）

`rvf.ui.command.v1` の変更履歴は本節に記録する（この wire には
`WIRE_VERSION` const がなく、op 名接尾辞が版数 token —— D3）。

| 日付 | Issue | 変更 | 版数 token |
| --- | --- | --- | --- |
| 2026-11-24 | #944 | `Action.params` に user-entered params の型付き profile（`CommandParam`/`CommandParams`）と mint 経路（`ViewHost::dispatch_action`）を追加 | `rvf.ui.command.v1` 維持（v1 内 extension、bump なし） |

## 検証計画

headless lane（`crates/ruviform-ui-test`）での検証と AC の対応。

| AC | 検証 | 場所 |
| --- | --- | --- |
| params wire 契約 + 版数・履歴 | 本書（D2/D3 + 上表） | `docs/designs/issue-944-ui-command-params.md` |
| widget→host params | `dispatch_action` で mint した intent が `ViewEvent::Intent` として `expect_receipt = true`・`CommandIntent::Action`・canonical JSON params を持つこと、serde 出力が `{"kind":"action",...,"params":{...}}` 形になること、`{command_id,expect_receipt,intent}` envelope 経由で params が保持されること | `ui_command_params.rs` |
| host→widget params | mint した command に `Event::Result` receipt を返し、`Notify` の `value` に params 形 payload を載せて `deliver_event` → `apply_projection` で `observed` に着くこと | `ui_command_params.rs` |
| codec 境界 | `to_value`/`from_value` の canonical 形受理・非 canonical 拒否（null・配列・非 point オブジェクト）・非有限数値の `InvalidInput` 拒否・`Enum` → `Text` の decode 非対称 | `ui_command_params.rs` |
| 非回帰 | 既存 `component_view.rs`・parity fixture・`dispatch_action` の `params: None` で `"params"` キーが出ないこと | 既存 lane + `ui_command_params.rs` |
| 検証コマンド | `mise run fmt`/`fmt-check`/`check`/`lint`/`test`（ruviform-ui・ruviform-ui-gpui・ruviform-ui-test）・`mise run test-ui-component-view`・`mise exec -- prek run --all-files` | CI 同等入口 |

## Deferred Design Concerns

| 論点 | 本書での扱い | 移管先 |
| --- | --- | --- |
| coalescible な params 更新（XY pad ドラッグ等の高頻度 params stream） | `Action` は non-droppable のまま —— 高頻度更新が要る widget は `Preview`/gesture 系で設計するか、coalesce 意味を持つ別 kind の検討が必要 | params を生む component kind を設計する後続 Issue |
| typed params の host→widget 投影契約 | `StateProjection<serde_json::Value>` で運搬済み。typed 化は consumer payload schema 確定後 | synvelis#125 系の統合検討 |
| 新 `ComponentSpec` kind（入力 widget: テキスト入力・XY pad・数値スピン等） | 選択肢 (iii) として棄却 —— render plumbing と fixture 更新を伴う製品機能 | consumer の UI slice Issue（synvelis#125 残存 AC 側） |
| `ui/`（TS/WebView レガシー側）の同型契約 | 未調査 —— 本 slice は GPUI/headless 側の wire 契約のみ | レガシー UI seam が残る限りの follow-up 調査 |
| spec 宣言 params と runtime params の merge 意味 | 導入しない —— `dispatch_action` は runtime params を verbatim で運ぶ | 必要になった時点の個別 Issue |

## 関連

- Issue [#944](https://github.com/scarecrow173/ruviform/issues/944) — 一次ソース
- [issue-119 L1 wire バージョン方針](../specs/issue-119-l1-wire-version-policy.md) —
  「最新の単一バージョンのみ・非互換時のみ版数更新」の正本（D3 の適用元）
- [issue-630 editor verb 設計](issue-630-editor-verbs.md) — L1 plugin バイナリ
  wire（`WIRE_VERSION` 7→8 計画）の参照例。`rvf.ui.command.v1` とは別 wire
- synvelis `decode_view_command`（`synvelis-app/src/binding.rs:948-1035`） —
  consumer admission の先例・受け口（D7）
- synvelis `docs/specs/tim-01-ppqn-ui.md` — consumer 側の動機
  （「Action 経路は params を渡せない」の確認元）
