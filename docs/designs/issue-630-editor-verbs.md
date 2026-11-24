---
id: "DESIGN-630editorVerbs01"
type: "design"
status: "draft"
title: "Issue #630 editor verb 設計提案 — send_gui_key / set_gui_focus / popup の unsupported 行を実現する GUI 動詞 API"
created: "2026-11-24"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-ac-inventory.md"
    - "docs/designs/issue-631-editor-input.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-630-editor-verbs.md"
      - type: "doc"
        path: "evidence/rf630/editor-verbs-design/investigation.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-631-editor-input.md"
    - "docs/designs/issue-630-fault-modes.md"
    - "docs/specs/issue-630-test-editor.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/designs/issue-630-clap-editor-verbs.md"
    - "evidence/rf630/plugin-dock-real-clap.jsonl"
    - "evidence/rf631/plugin-dock-real-scenario.jsonl"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-630-ac-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/rf630/plugin-dock-real-clap.jsonl"
    - "evidence/rf631/plugin-dock-real-scenario.jsonl"
    - "evidence/rf630/editor-verbs-design/investigation.md"
  defers: []
  deferred-by: []
metadata:
  issue:
    number: 630
    url: "https://github.com/scarecrow173/ruviform/issues/630"
---

# Issue #630 editor verb 設計提案 — send_gui_key / set_gui_focus / popup の unsupported 行を実現する GUI 動詞 API

## 目的と範囲

本書は Issue #630 AC4 の CLAP leg に残る 3 行の unsupported ——
`real-clap-editor-input`・`real-clap-editor-focus`・`real-clap-editor-popup` ——
を実現するための GUI 動詞 API（editor verb: `send_gui_key` / `set_gui_focus` /
popup）の設計材料を提供する。対称確認として、全 pass 済みの VST3 leg
（`real-editor-{input,focus,popup}`）にも同じ verb 表面を適用し、fixture 契約の
語彙と観測点を揃える。

本 slice は設計提案のみを納品する。実装コード・fixture・adapter・lane 定義の
変更は一切含まない。AC4 gate 本体（実 Window・child HWND・画素・Close・
再 Open の受入枠）と `ruviform#631`・`ruviform#532` の責務は動かさない。

本文書は `status: "draft"` の提案文書であり、記述する API 形状・語彙・
slice 分割はすべてユーザー決裁待ちである。

## 決定事項と未決裁の提案

ユーザーが決裁済みの 5 方針を本設計の前提とする。

1. `send_gui_key` はショートカット seam —— OS イベント注入
   （`SendInput`/`keybd_event`/`PostMessage` 擬似入力）ではなく、dock-test
   driver プロトコル（session ops → `HostCommand` → adapter → plugin）経由の
   合成コマンドとしてキーイベントを plugin 側に渡す。合成イベントの意味論と
   「実 OS イベントではない」ことの正直な表現を設計する。
2. `set_gui_focus` は target パラメータ化 —— focus 対象を plugin editor 内の
   element と host 側トップレベル窓で指定可能にし、両ケースをカバーする。
   focus 取得の観測方法も設計対象とする。
3. `popup` は kind パラメータ化 —— child window / dialog の両 kind を定義し、
   実装コスト差が大きい場合は優先順位・段階実装を提案に含める。
4. 両 ABI 対称 —— CLAP（`clap_plugin_gui`・embedded native window）と VST3
   （`IPlugView` frame・`ruviform#885` fault carrier 済み）で対称の verb 表面と
   し、VST3 側の既存 pattern と fault carrier（`TestFaultProcessor`）の流儀に
   寄せる。
5. 実 GUI 受入は Coordinator 駆動の対話型 session —— 実装後の実 Windows GUI
   検証はローカル対話型 session（Coordinator が computer-use で駆動）を前提と
   し、観測点・証拠フォーマットを本書に含める。

これ以降の内容はすべて Worker の提案（ユーザー決裁待ち）である。verb 語彙の
型名・variant、wire tag・`WIRE_VERSION` の扱い、CLAP 側受け口の形、観測
token 文法、slice 分割、検証の段階構成は未決裁であり、「推奨」と記す案も
決定ではない。

## 現状整理

確定済みの事実を、設計の足場として 4 層に分けて置く。行範囲の根拠と調査手順は
`evidence/rf630/editor-verbs-design/investigation.md` に記録した。

### trait・境界語彙・wire（`ruviform-plugin-core`）

`Plugin::set_gui_focus(bool)`・`Plugin::send_gui_key(GuiKeyEvent)` は trait
default `Err(HostError::Unsupported)` であり、`ErasedPlugin` が同じ面を mirror
する（`crates/ruviform-plugin-core/src/plugin.rs:269-295, 436-457`）。
`GuiKeyEvent{direction, key, key_code, modifiers}` と
`GuiKeyDirection::{Down,Up}`（`#[non_exhaustive]`）、`GuiModifiers` bitmask は
`boundary.rs:164-256` に確立済みで、`key_code` は「adapter の key-code 空間」の
opaque 値と定義されている。wire 面では `Command::SetGuiFocus`（tag 26）と
`Command::SendGuiKey`（tag 27）が wire-v6 として存在し、`WIRE_VERSION = 7`、
「new verbs arrive only with a `WIRE_VERSION` bump」と codec 側で明記される
（`wire.rs:135, 709-715, 909-921`）。`OpenGuiMode::{Floating,Docked}` も既に
語彙にあるが、これは L1 open の mode であって popup verb ではない。

### session ops・command dispatch（`ruviform-plugin-host`）

`PluginEditorSession::set_focus`/`send_key` → `EditorSessionOps` →
`InProcessEditorOps`/`L1EditorOps` → `HostCommand::SetGuiFocus`/`SendGuiKey` の
経路が complete に存在する（`editor_session.rs:584-600, 1319-1355, 2010-2044,
2280-2314`、`command.rs:145-162`）。session 面の契約は non-fatal ——
`Attached`|`Hidden` 以外では `InvalidState`、plugin の拒否は
`Ops(Unsupported)` で伝播するだけで状態遷移しない。`EditorCapabilities` には
`input`・`focus`・`popup` の flag（bit 8/9/6）が既に定義されているが、L0
surface は verb 実行前に per-plugin support を証明できないため全 flag が
unclaimed のままである。

### VST3 側（pass 済み・既存 pattern の正本）

adapter は `set_gui_focus`→`IPlugView::onFocus`、`send_gui_key`→
`onKeyDown`/`onKeyUp` を実装済み（`ruviform-plugin-vst3/src/host.rs:3431-3486`）。
fixture `test_editor` は受信した verb を `f:{state}`・
`kd:{key:04x}:{key_code:04x}:{mods:04x}`・`ku:{…}` token として child HWND
title に刻印し、`key_code==0x50`（`VK_P`）の `onKeyDown` で `popup_toggle`
—— plugin 所有 `WS_POPUP` HWND `rvf-test-editor-popup-{seq}` の open/close を
`po:{seq}`/`pc:{seq}` token と実 top-level HWND で観測可能にする
（`ruviform-plugin-test-vst3/src/editor_view.rs`）。fault carrier
（`TestFaultProcessor` の 8 帯域 stepped `FAULT_PARAM`）は `ruviform#885` で
両 ABI に載っている。

### CLAP 側（unsupported の現状）

`ClapPlugin` は `send_gui_key`/`set_gui_focus` を実装せず、trait default の
`Err(Unsupported)` に留まる（`ruviform-plugin-clap/src/host.rs:1716-1873` に
実装する GUI verb は `open_gui`/`close_gui`/`set_gui_size`/
`set_gui_visibility`/`set_gui_parent`/`drain_gui_requests`/
`drain_dirty_notices` のみ）。`clap_plugin_gui` 規格には embedded window への
key/focus 配送 verb が存在しない。`open_gui` は `is_floating: false` を
hardcode し、embedded `set_parent`→`show` で plugin 所有 child HWND を作る
（`host.rs:1736-1762`）。

fixture `test_editor`（`ruviform-plugin-test-clap/src/editor.rs`）は
floating-mode popup surface —— `set_transient`/`suggest_title`/`show` →
`win32::popup(owner, title, size)` の実 `WS_POPUP` HWND —— を既に持つが、
embedded `open_gui` 経路からは到達不能である。また CLAP fixture の child HWND
は title なしで生成されており、VST3 fixture のような event-log title channel
を持たない。

### dock-test 行の現行 verdict

`real-clap-editor-{input,focus,popup}` は real lease 経由で verb を実行し、
plugin の defined `Unsupported` landing をそのまま記録する（Coordinator
decision 1 —— `real_catalog.rs:348-352`、`real_dock.rs:17410-17475`）。verdict
mapping は `OpLanding::{Refused,Accepted,Blocked,Other}` で、Refused →
`Unsupported`、Accepted + effect 観測 → `Pass`、Accepted + 観測不能 →
`NotRun`、その他 → `Fail`（`real_dock.rs:10880-10928`）。popup の HWND oracle
は top-level inventory（class `RuviformTestEditor`・title prefix
`rvf-test-editor-popup-`・owner = child の `GA_ROOT`・`WS_POPUP` set /
`WS_CHILD` clear・(0,0) 200x100）として確立済み（`real_dock.rs:11706-12014`）。

## send_gui_key — 合成コマンドの意味論（提案）

### 合成イベントの意味論

`send_gui_key` が運ぶのは OS 入力ではなく、session ops → `HostCommand` →
adapter → plugin の verb chain を通る `GuiKeyEvent` 値の配送である。payload
各フィールドの意味は現行契約を維持する: `direction` は Down/Up の配送順序、
`key` は BMP 内の文字（supplementary-plane は `InvalidValue`）、`key_code` は
adapter の key-code 空間の opaque 値、`modifiers` は `GuiModifiers` bitmask。

検証対象は 2 点に限定する: (a) verb chain が値を受理すること（session の
`Ok` 返却）、(b) plugin 側が値を受領したことの観測（fixture event-log
token）。OS の input queue・focus routing・message pump を通る配送は検証
対象外とし、証拠語彙でも分離する（後述）。CLAP 側 seam が `key_code` を
解釈する空間は、VST3 ledger が刻む `kd:{key}:{key_code}:{mods}` token と
同じ grammar にするため Win32 virtual-key 空間とする —— `key_code` が
「adapter の key-code 空間」の opaque 値である契約を変えずに、CLAP seam
側の解釈を Win32 VK に定めるだけで、両 ABI の event-log token を同一文法に
揃えられる。

### CLAP 側の受け口の選択肢

`clap_plugin_gui` に key/focus 配送 verb がないため、CLAP 側の受け口は設計が
必要である。比較対象は 4 案。

| 案 | 形 | 誠実さ | コスト | 備考 |
| --- | --- | --- | --- | --- |
| K1 fixture-private extension seam | adapter が plugin に private extension を query し、その vtable で `GuiKeyEvent` を配送 | 高い — ABI 上の vtable 呼出であり、format 規格外の配送であることを名前で示せる | 中 | CLAP の `get_extension` と VST3 の `queryInterface` に対称な seam として設計できる |
| K2 OS window message | fixture が child HWND を private seam で公開し、adapter が `SendMessage(WM_KEYDOWN)` 等を投げる | 中 — 実 window message ではあるが OS 入力キューを通らず、synthetic 性は K1 と同程度 | 大 | fixture に独自 wnd_proc が要り、HWND 公開自体にも private seam が要る |
| K3 in-band event（params/event list 経由） | `ParamValue` 系の event で key を符号化 | 低い — `GuiKeyEvent` の各 field を素直に往復できず、param 面と GUI ops を混線させる | 中 | fault carrier の流儀とは別物（param 駆動は armed-fault の dispatch であって verb 配送ではない） |
| K4 defined refusal 維持 | `Err(Unsupported)` のまま | 高いが行が実現しない | 0 | タスクの前提に反する |

推奨は K1 である。`clap.ext.gui` と同じ mechanism —— 拡張 id で query した
vtable を通じた plugin 内関数呼出 —— であり、OS 注入でないこと、規格外の
test 用 seam であることを extension id の命名で正直に表せる。VST3 側でも
element 粒度の focus と popup verb が format verb を持たないため、同じ形の
private seam（VST3 は view の private IID `queryInterface`、CLAP は
`plugin.get_extension`）を揃えて引くと、fixture 契約を ABI 対称に保てる。
extension を持たない plugin は defined refusal `Err(Unsupported)` のまま ——
claim しない honesty を保つ。

K2 は実 window message で fidelity が高い一方、fixture に独自 wnd_proc と
HWND 公開 seam の双方が要り、得るもの（pump 経由の配送の実演）に対して
検証できる内容（verb が plugin 所有 state に届いたこと）が増えない。K3 は
payload が往復しない。いずれも推奨しないが、K2 は fidelity を上げたい場合の
後続案として残してよい。

### 「実 OS イベントではない」ことの正直な表現

evidence 系の語彙で synthetic delivery と OS 実入力を分離する。

- `evidence` tag: `clap-editor-input:synthetic-verb-delivery` のように配送
  seam を名指し、`real-os-*` 語彙を付けない。
- `pass_detail`: 「session.send_key(down+up 'a') accepted — the fixture
  extension recorded kd/ku tokens (synthetic verb delivery; no OS input
  queue or focus routing exercised)」のように、受理と受領観測だけを Pass
  根拠とし、OS input path の証明とは切り分ける。
- catalog `required`: `NativePluginDock` のみを維持し、
  `RealOsPointerOrKeyboard` をこれらの行の required にしない（OS 入力駆動を
  要求・主張しない）。
- verdict 意味: `Pass` = 「合成コマンドが verb chain を受理され、plugin が
  配送を記録した」。実 OS 入力の検証は別 case（pointer/keyboard capability
  を要求する行）の責務とする。

## set_gui_focus — target パラメータ化（提案）

### 対象語彙

```rust
#[non_exhaustive]
pub enum GuiFocusTarget {
    /// The plugin editor surface itself — `set_gui_focus(true)` と同じ到達先。
    Editor,
    /// Plugin 内部の element —— plugin が解釈する opaque id。
    /// 配送 semantics であり OS focus の主張ではない。
    EditorElement { element: u32 },
    /// Host 側トップレベル窓（session の container）——
    /// plugin ではなく host 側に focus を戻す対象。
    HostWindow,
}
```

`EditorElement` の element id は host が意味を解釈しない opaque 値とし、
`ParentLease::payload` や `GuiKeyEvent.key_code` と同じ「adapter/plugin 空間の
opaque」の先例に従う。`#[non_exhaustive]` は公開 enum ポリシーの規定。

### 既存 `focused: bool` 署名との関係

| 案 | 形 | wire/API 影響 | 評価 |
| --- | --- | --- | --- |
| F1 既存 verb の意味拡張 | `set_gui_focus(bool)` を `set_gui_focus(GuiFocusTarget)` に差し替え | trait 署名の source-breaking、全 implementor/caller 修正、`SetGuiFocus` payload 変更で wire bump 必須 | 破壊的 — 不採用候補 |
| F2 新 verb 追加 | `set_gui_focus_target(GuiFocusTarget)` を default `Unsupported` で追加し、`set_gui_focus(bool)` は Editor 相当の短手として残す | `ErasedPlugin` mirror 追加、`Command::SetGuiFocusTarget`（tag 29 案）+ `WIRE_VERSION` 7→8、session に non-fatal の `set_focus_target` | 推奨 — additive で先例に整合 |
| F3 verb 外で処理 | target 概念を置かず、HostWindow case は lane 側が直接 `SetFocus` する | verb 表面が HostWindow を表現できず、session ops の外に case 固有情が散る | 不採用候補 — 対称性と L1 経路を失う |

推奨は F2 である。`set_gui_focus(bool)` は `ruviform#754`/`ruviform#631` で
着地済みの public trait method であり、署名差し替えは plugin-core の公開 API
を source-breaking し、VST3 adapter・fixture・両 ops・codec を同時に触る。
一方で `SetGuiFocus` の `focused:u8` payload は target を運べないため、どのみち
wire bump は必要 —— ならば新 tag で追加するほうが、tag 26 の意味を凍結したまま
最小の波及で済む。dyn 語彙（`ErasedPlugin`）にも default `Unsupported` の
mirror を足せば trait object 面が揃う。

### focus 取得の観測方法

対象ごとに観測点を分ける。

- `Editor`: fixture event-log token `f:{0|1}` を基本 oracle とする（VST3 既存
  grammar を CLAP fixture にも揃える）。加えて fixture が `apply_focus` 内部で
  実 `SetFocus(child)` を呼ぶよう拡張すれば、host 側の `GetFocus()==child`
  という実 OS oracle を併記できる —— 実 plugin が `onFocus` 後に自前で
  SetFocus する振る舞いに沿う。実 `SetFocus` は computer-use session の操作
  focus を一時的に奪うため、case 内で `HostWindow`/`false` への戻しまでを
  一連として扱う。
- `EditorElement{n}`: fixture event-log token `fe:{element}:{0|1}`（提案文法）
  のみ。element は HWND ではなく論理 routing のため、観測は「plugin が
  element 宛ての focus intent を記録した」ことに止まる —— OS focus の主張は
  しない。
- `HostWindow`: plugin 呼出ではなく ops 層が `SetFocus(container HWND)` を
  実行し（Windows のみ; 非 Windows は ops-level `Unsupported`）、観測は
  `GetFocus()`/`GetForegroundWindow()` 系の host 側 OS oracle とする。plugin
  には focus 喪失の通知だけが届く（VST3 は `onFocus(false)`、CLAP は seam
  経由の `apply_focus(false, Editor)`）— `f:0` token が残る。

## popup — kind パラメータ化（提案）

### kind 語彙

```rust
#[non_exhaustive]
pub enum GuiPopupKind {
    /// Editor 面の下位に属する plugin 所有の補助 window ——
    /// editor child HWND の `WS_CHILD` 子孫（EnumChildWindows で観測）。
    ChildWindow,
    /// Plugin 所有の top-level window —— editor の root ancestor を
    /// owner とする `WS_POPUP` 系（top-level inventory で観測）。
    /// 既存 VST3 `popup_toggle`・CLAP floating surface の形状。
    Dialog,
}
```

### verb 形態の選択肢

| 案 | 形 | 評価 |
| --- | --- | --- |
| P1 専用 verb 新設 | `set_gui_popup(kind, open)`（default `Unsupported`）+ `Command::SetGuiPopup`（tag 30 案）+ wire bump | 推奨 — kind を正直に運べ、`set_gui_visibility(bool)` 系の命名・引数 pattern に揃う |
| P2 `send_gui_key` トリガ維持 | 現行 VST3 の `VK_P` 起動を維持 | kind を運べず、`key_code` に kind を潜ませるのは不誠実 — 不採用候補 |
| P3 `GuiRequest` sink/drain 活用 | plugin→host request channel に popup を載せる | 方向が逆 — `GuiRequest` は plugin 発の要求で、本 verb は host→plugin の命令。不採用 |
| P4 `open_gui`/reparent 系の拡張 | `OpenGuiMode` や `set_gui_parent` に popup を混ぜる | popup は editor の存続中に開閉する補助 surface であり、open/reparent の lifecycle verb とは別物 — 不採用 |

### ABI 非対称の吸収

popup の非対称は「format verb の有無」と「fixture が既に持つ helper」にある。
VST3 `IPlugView` には popup verb がなく、現行は `onKeyDown(VK_P)` の fixture
契約で駆動する。CLAP `clap_plugin_gui` にも popup verb がなく、fixture の
floating-mode surface は embedded `open_gui` から到達不能である。吸収先は
private seam —— `set_gui_popup` は adapter から fixture-private ops（VST3 は
view の private IID、CLAP は private extension）へ転送され、fixture 側が
kind に応じて `win32::popup` 系 helper を呼ぶ構成を推奨する。fixture は既に
VST3 `win32::popup(anchor,title)`（Dialog 形状）と CLAP
`win32::popup(owner,title,size)` を持ち、Dialog kind は両 ABI で既存 helper を
再利用できる。`ChildWindow` kind は editor child HWND 配下の `WS_CHILD` を
作る小さな helper の新設が要る。

非 Windows では win32 shim が `None` を返すため、verb 受理と `po:`/`pc:`
token は記録されるが HWND 実在は観測不能 —— 現行の `po:`/`pc:` 先例どおり
論理面と HWND 面を分離し、verdict は `NotRun` 側へ倒す。

### 実装コストと段階実装

| kind | VST3 | CLAP | 先行順の提案 |
| --- | --- | --- | --- |
| Dialog | 小 — `win32::popup` 既存、verb を `popup_toggle` に direct 化 | 小 — `win32::popup(owner,title,size)` 既存、seam から直接駆動 | **先行** — 両 ABI で既存 helper + 確立済み oracle を再利用 |
| ChildWindow | 中 — `WS_CHILD` 補助 window helper 新設 + EnumChildWindows oracle | 中 — 同上 | **後段** — helper 新設と oracle 派生が要る |

## 両 ABI 対称の verb 表面（提案）

verb × ABI の着地点を一覧化する。非対称の吸収先は 3 種 —— adapter（format
verb への写像）、fixture seam（private IID/extension）、defined refusal —— の
いずれかで正直に落とす。

| verb | VST3 着地 | CLAP 着地 | 観測 | decline 時の意味論 |
| --- | --- | --- | --- | --- |
| `send_gui_key` | `IPlugView::onKeyDown`/`onKeyUp`（format verb） | private extension `deliver_key` | `kd:`/`ku:` token | view/extension 無し → `Unsupported`、plugin 拒否 → `PluginFailed`、supplementary `key` → `InvalidValue` |
| `set_gui_focus(bool)` | `onFocus(TBool)` | private extension `apply_focus(focused, Editor)` | `f:{state}` token + （実 `SetFocus` 採用時）`GetFocus` oracle | 同上 |
| `set_gui_focus_target(Editor)` | `onFocus(true)` | extension `apply_focus(true, Editor)` | `f:1` token + `GetFocus` oracle | 同上 |
| `set_gui_focus_target(EditorElement{n})` | private IID `apply_focus(n)` | extension `apply_focus(n)` | `fe:{n}:{state}` token（synthetic のみ） | 同上 |
| `set_gui_focus_target(HostWindow)` | ops 層で `SetFocus(container)` + `onFocus(false)` 通知 | ops 層で `SetFocus(container)` + extension 通知 | `GetFocus()==container` + `f:0` token | 非 Windows → ops-level `Unsupported` |
| `set_gui_popup(Dialog, open)` | private IID `popup` → `win32::popup`/`destroy` | extension `popup` → `win32::popup`/`destroy` | `po:`/`pc:` token + top-level inventory oracle | 同上 + seam 無し → `Unsupported` |
| `set_gui_popup(ChildWindow, open)` | private IID `popup` → child helper | extension `popup` → child helper | token + `EnumChildWindows` oracle | 同上 |

## 実装 slice 分割案（提案）

依存関係は 1 → 2 → {3,4} → 5 → 6。3 と 4 は ABI ごとに独立。

| slice | 内容 | 占有見通し | lane 影響 |
| --- | --- | --- | --- |
| 1. 境界語彙 + wire | `GuiFocusTarget`/`GuiPopupKind` 型、`Plugin`/`ErasedPlugin` 新 default verb、`Command::SetGuiFocusTarget`/`SetGuiPopup`（tag 29/30 案）、`WIRE_VERSION` 7→8、codec・roundtrip test | `crates/ruviform-plugin-core/` | wire bump は codec test のみ — lane 不変 |
| 2. host command + session ops | `HostCommand::{SetGuiFocusTarget,SetGuiPopup}`、`EditorSessionOps`/`PluginEditorSession` 追加 method、`InProcessEditorOps`/`L1EditorOps` forward、capability bit の claim 規則 | `crates/ruviform-plugin-host/` | session 契約 test のみ — lane 不変 |
| 3. VST3 adapter + fixture | private IID seam、`set_gui_focus_target`/`set_gui_popup` 実装、element token・child helper、`host_path` test | `crates/ruviform-plugin-vst3/` + `crates/ruviform-plugin-test-vst3/` | `test ruviform-plugin-test-vst3`・`host_path` |
| 4. CLAP adapter + fixture | private extension seam、`send_gui_key`/`set_gui_focus`/target/popup 実装、event-log title channel、popup helper、`host_path` test | `crates/ruviform-plugin-clap/` + `crates/ruviform-plugin-test-clap/` | `test ruviform-plugin-test-clap`・`host_path`・ABI validator 再実走の要否確認 |
| 5. dock-test row flip | `real-clap-editor-{input,focus,popup}` を effect-verified に、VST3 行に target/kind coverage | `crates/ruviform-plugin-dock-test/` | `test-ui-plugin-dock-real*` opt-in lane（変更自体は lane 定義外） |
| 6. 実 GUI 受入 | Coordinator computer-use 駆動 session で両 lane 実走、ledger 再生成 | 実走環境 + `evidence/` | opt-in lane の実走 |

## 検証計画（提案）

### 層の分離

headless/semantic 層（非 ignore unit test・`host_path`・`--blocked-probe`
fail-closed）と実 Windows GUI 層（`test-ui-plugin-dock-real*` opt-in lane）を
分離する。実 GUI 層は Coordinator が computer-use で駆動するローカル対話型
session を前提とし、本 slice は計画のみを置く。

### verb 別の観測点

- `send_gui_key`: session `send_key` down+up の受理 → child title event log の
  `|kd:0061:0041:0000,ku:0061:0041:0000` suffix（CLAP も同文法 —— fixture が
  event-log channel を実装する前提）。synthetic delivery であることを
  evidence tag と `pass_detail` に明記。
- `set_gui_focus`: `Editor` は `f:1`/`f:0` token、実 `SetFocus` 採用時は
  `GetFocus()==child` を併記。`EditorElement{n}` は `fe:{n}:{state}` token のみ
  （synthetic のみ）。`HostWindow` は `GetFocus()==container` oracle + `f:0`
  token。
- `popup`: `Dialog` は `po:{seq}`/`pc:{seq}` token + 既存 top-level inventory
  oracle（class・title prefix・owner・style bit・rect・dead 確認・residual
  teardown）。CLAP 側は fixture の class が `RuviformClapTestEditor`
  （VST3 の `RuviformTestEditor` と共存のため別名）なので oracle 定数を
  ABI ごとに分け、popup title は `rvf-test-editor-popup-{seq}` の世代 suffix
  まで揃える（CLAP floating surface の現行 default は
  `rvf-test-editor-popup` —— 対称 oracle のため fixture 側で世代 suffix を
  刻む）。`ChildWindow` は token + editor child 配下の
  `EnumChildWindows` oracle（新設 —— title token + WS_CHILD style + parent
  一致を検証）。

### verdict 条件と証拠フォーマット

verdict mapping は `editor_op_body` 系の既存規範を維持する:
verified+accepted+effect → `Pass`、refused（defined `Unsupported`）→
`Unsupported`、accepted-unverifiable → `NotRun`、その他 → `Fail`。証拠は
JSONL row の `case`/`verdict`/`observed.steps[]`/`evidence[]`/
`environment{repo_sha, environment_kind, plugin_artifact.sha256}` キーを
維持し、`--blocked-probe` で全行 `NotRun`+`environment_blocked` の
fail-closed を実証する。`expected`/`observed` の step detail に「synthetic
verb delivery」を明記し、「OS 実入力を検証した」と混同しない。

## 失敗時の倒れ方（defined refusal 語彙）

既存先例に揃える: plugin surface 無しは trait default `Unsupported`、view/
surface 未 open は adapter `Unsupported`、plugin/extension の拒否は
`PluginFailed`、表現不能な値は `InvalidValue`、wire decode 不能は
`Malformed`、unadvertised child は capability gate の `Unsupported`。
ops 層では `map_host_error` が `EditorOpError::{Unsupported,Plugin,
InvalidValue,HostLost,Io}` に写像し、session は `Attached`|`Hidden` 以外で
`InvalidState`、plugin 拒否は `Ops(...)` の non-fatal 伝播 —— state 遷移を
新設しない。lane verdict は Refused→`Unsupported`、accepted-unverifiable→
`NotRun`、環境 block→`NotRun`+`environment_blocked`、contract mismatch→
`Fail` の既存軸を維持する。

## open questions

- private seam の採否と extension id / IID の命名（推奨案は
  `ruviform.editor-ops` 系の ruviform-namespaced seam —— adapter が optional
  capability として query し、未実装 plugin は defined refusal を保つ）。
- `GuiFocusTarget`/`GuiPopupKind` の命名と variant 追加余地
  （`#[non_exhaustive]` 前提で `Menu`/`Tooltip` 等の将来 kind を残すか）。
- `set_gui_focus(bool)` の存続: 推奨は残す（Editor 相当の短手）。破壊的
  変更を避ける一方、`set_gui_focus_target` との意味重複の整理要否。
- popup kind の先行順: 推奨は `Dialog` → `ChildWindow`。
- wire capability bit: 新 verb を `GUI_FLOATING` bit の family に含めるか、
  wire-v8 の新 bit を割るか。
- `EditorCapabilities.input`/`focus`/`popup` の claim 規則 — L0 surface は
  open 前に support を証明できないため、claim するなら open 後の
  `Opened.capabilities` か、verb 受理の実績ベースか。
- fixture が `Editor` target の focus を実 `SetFocus` まで上げるか（推奨は
  上げる —— 実 OS oracle を得られるが、対話 session での focus 干渉を許容
  するかの判断が要る）。
- `Dialog` kind に OS modal dialog（`DialogBox` 系の modal pump を要する形）
  を含めるか —— 推奨は含めず、modeless top-level を dialog kind とする。
- CLAP floating surface（`set_transient`/`suggest_title`/`show`）を verb 表面に
  載せるか —— 推奨は載せず、private seam で plugin 所有 popup を直接駆動
  する（floating surface は fixture 内部の helper 供給元として残す）。
