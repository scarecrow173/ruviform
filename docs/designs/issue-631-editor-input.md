---
id: "DESIGN-01M7WJTWYS9T4C5HCY30JGNV1V"
type: "design"
status: "approved"
title: "Issue #631 editor input/focus verbs 設計 — 境界語彙・L0-only command・fixture event log・title-suffix oracle"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-editor-input.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 editor input/focus verbs 設計 — 境界語彙・L0-only command・fixture event log・title-suffix oracle

## Context

`docs/specs/issue-631-editor-input.md` が定める slice の設計。
AC3 の残る editor verb（`send_key`/`set_focus`）を
driver-neutral な境界から `test_editor` の real `IPlugView`
まで通し、real lane が OS で観測できる効果として記録する。
先行 slice が確立した 2 つの資産を再利用する:

- `EditorSessionOps` の non-fatal verb 契約（`set_size` 先例 —
  拒否は `Ops(Unsupported)` で session 状態を動かさない）。
- child HWND の window text（`GetWindowTextW`）を OS-observable
  channel として使う attach-generation tag
  （`rvf-test-editor-attach-{n}`、`real-editor-attach`/
  `real-editor-redock` で確立）。

## Scope

### In Scope

- plugin-core boundary 語彙と `Plugin`/`ErasedPlugin` hook。
- plugin-host の L0-only `HostCommand` 2 variant と dispatch。
- `EditorSessionOps`/`InProcessEditorOps`/`L1EditorOps` と
  session convenience methods。
- VST3 adapter の `IPlugView` forward。
- fixture の ordered event log（window text 追記）。
- real lane の 2 case + 共有 `observe_editor_verb_lifecycle`。

### Out of Scope

- wire encoding / `L1EditorOps` の verb 対応、CLAP adapter の
  input/focus、IME、pixel oracle、macOS/Linux GUI。

## Components and Boundaries

### 境界語彙（`ruviform-plugin-core/src/boundary.rs`）

- `GuiKeyDirection` — `#[non_exhaustive]` enum、`Down`/`Up`。
- `GuiModifiers` — u16 bitmask wrapper。4 bool 設計は
  `clippy::struct_excessive_bools` に抵触したため bitmask に
  reshape（plugin format の実表現と同形）。const は
  `NONE`/`SHIFT`/`ALTERNATE`/`COMMAND`/`CONTROL`、API は
  `contains`/`union`。
- `GuiKeyEvent` — `{direction, key: char, key_code: i16,
  modifiers: GuiModifiers}` の plain data。`key` は logical
  char、`key_code` は driver が定義する virtual key 値で、
  OS handle や toolkit 型を一切含まない。

### plugin seam

`Plugin::{set_gui_focus(bool), send_gui_key(GuiKeyEvent)}` は
既定 `Err(HostError::Unsupported)` — 未対応 plugin が定義済みの
拒否で答える。`ErasedPlugin` も同じ既定で mirror する（両者の
forwarding impl は concrete hook に委譲）。

### L0-only `HostCommand`

`HostCommand::SetGuiFocus{handle,focused}` と
`SendGuiKey{handle,event}` は `SetGuiVisibility` と同じ L0-only
クラス:

- `command_handle`/`retarget` は `handle` フィールドを正しく
  読み替え・書き換え（composite の retarget 経路が破綻しない）。
- composite の out-of-process guard は既存 L0-only verb と同じ
  仕組みで `Unsupported` を返し、`to_wire` には到達しない —
  wire enum に variant を増やさないので wire version 不変。
- `host.rs` の `execute` match は plain slot と shared slot 双方で
  `plugin.set_gui_focus`/`plugin.send_gui_key` を呼ぶ。

### session 面（`editor_session.rs`）

- `EditorCapabilities` に `input`/`focus` flag を追加 — L0
  surface は verb 実行前に per-plugin support を証明できない
  ため、flag は unclaimed のまま（over-claim しない）。
- `EditorSessionOps` に `set_focus(session_id, focused)` と
  `send_key(session_id, event)`。
- `PluginEditorSession::set_focus`/`send_key` は
  `Attached`|`Hidden` を要求し、それ以外は `InvalidState`。
  plugin 拒否は `Ops(Unsupported)` で伝播するのみで状態遷移
  しない（`set_size` と同じ non-fatal 契約）。
- `InProcessEditorOps` は対応する `HostCommand` を
  `send_command` で host に投げ、`HostError` → `EditorOpError`
  の既存写像をそのまま使う。`L1EditorOps` は全 verb で
  `Unsupported` のまま。

### VST3 adapter（`ruviform-plugin-vst3/src/host.rs`）

- `set_gui_focus` → retained `ComPtr<IPlugView>` の
  `onFocus(TBool)`。
- `send_gui_key` → `direction` で `onKeyDown`/`onKeyUp` を
  分岐。`GuiModifiers` の各 bit を VST3 `KeyModifier` の対応
  bit（`kShiftKey`/`kAlternateKey`/`kCommandKey`/`kControlKey` =
  1/2/4/8）へ per-flag 写像（bit 位置は共有しないので
  passthrough 不可）。
- `key`（`char`）は `u32::from` → `u16::try_from` で 1
  UTF-16 `char16` に収める — 収まらない
  supplementary-plane は `InvalidValue`（truncate しない）。
- view 未 open → `Unsupported`、非 `kResultOk` → `PluginFailed`
  — 既存 GUI verb の honesty mapping と同一。

### fixture event log（`editor_view.rs`）

`TestEditorView` は `onFocus`/`onKeyDown`/`onKeyUp` の受信順に
token を event log へ append し、child HWND の window text を
`{attach_tag}|{t1},{t2},…` に再刻印する:

- `f:{0|1}` — `onFocus` の state。
- `kd:{key:04x}:{key_code:04x}:{modifiers:04x}` — `onKeyDown`。
- `ku:{…}` — `onKeyUp`（同形式）。

`attached` が新しい generation を刻むたび log は title 上で
保持され、detach では子 HWND ごと消える — つまり title は常に
「その attach generation で受け取った log」。lane が既に使う
`GetWindowTextW` だけで読め、新規 IPC・file・API を足さない。

### real lane oracle（`real_dock.rs`）

`observe_editor_verb_lifecycle`（resize/redock/visibility の
lifecycle 関数と同型）が 2 case の共有 body:

1. attach 契約 — `observe_attached_children` で child 存在・
   parent・visible・alive を確認。
2. verb — case ごとの `run` closure が production
   `InProcessEditorOps` 経由で verb 列を送る
   （input: `send_key` down+up 'a'、focus: `set_focus`
   true→false）。
3. landing — `Ops(Unsupported)` → `OpLanding::Refused`、
   `Ok` → `Accepted`、その他 → `Other`。
4. effect oracle — attach 中に再 enumerate し、全 child の
   title が case の `expected_suffix`（input:
   `|kd:0061:0041:0000,ku:0061:0041:0000`、focus:
   `|f:1,f:0`）で終わるかを検証。accepted で suffix が無い →
   `Fail`、観測不能（非 Windows 等）→ `NotRun` 側。
5. state — verb が non-fatal なので session は `Attached` の
   ままであることを確認。
6. close 収束 — `conclude_editor_session` で
   `[Attached{Docked}, Closed]`（`ATTACH_CLOSE_EVENTS`）と
   `IsWindow(child)==0`。

verdict mapping は `editor_op_body` 共通:
verified+accepted+effect → `Pass`、verified+refused →
`Unsupported`、accepted-unverifiable → `NotRun`、その他 →
`Fail`。

## Data and Control Flow

```
real-editor-input (run_scenario on RealOsGuiTest)
  → editor_op_body → drive_open_docked (real lease redeem)
  → observe_editor_input_lifecycle
      → session.send_key(ops, 'a' Down) → HostCommand::SendGuiKey
        → Vst3Plugin::send_gui_key → IPlugView::onKeyDown
        → fixture: log "kd:0061:0041:0000" + SetWindowTextW
      → session.send_key(ops, 'a' Up)  → onKeyUp → "ku:…"
      → EnumChildWindows → GetWindowTextW
        → title ends_with "|kd:0061:0041:0000,ku:0061:0041:0000"
      → session.close → removed() → DestroyWindow
        → events == [Attached{Docked}, Closed]
```

`real-editor-focus` は同じ経路で `set_focus(true|false)` →
`onFocus` → `f:1,f:0`。

## Non-Functional Constraints

- Driver-neutral: `GuiKeyEvent`/`GuiModifiers`/`GuiKeyDirection`
  は OS handle・toolkit 型を含まない。
  `editor_session.rs` に GPUI/HWND 型が入ると boundary テストが
  落ちる。
- L0-only: 新 command は wire enum に現れず wire version 不変 —
  composite guard が `Unsupported` を保証。
- Additive: `Plugin` hook は既定 `Unsupported` で後方互換。
- Non-fatal: session state machine に新遷移を足さない。
- RT 契約: すべて non-RT（UI thread）— RT-safe 経路は作らない。
- unsafe: fixture/lane の Win32 呼び出しは query/stamp のみで
  `#[expect(unsafe_code)]` + SAFETY コメント。

## Risks and Trade-offs

- **title の容量** — window text は長文を運べない。token は
  compact hex（最大 ~20B/件）で、lane は短い verb 列だけを
  期待するため問題にならない。超過時は新規 generation で
  reset される。
- **oracle の厳密さ** — `ends_with` は「最後に期待 token 列が
  付いた」ことを主張するが途中の余計な token を見逃す。
  fixture は受信順に append するため、余計な event が挟まると
  suffix がずれて検出できる — 厳密性と実装の単純さの均衡点。
- **`key` の表現域** — `char`→`char16` は BMP のみ。emoji 等は
  `InvalidValue` で拒否し、VST3 側の surrogate 仕様に踏み込ま
  ない（key verb が pair を運べない以上、拒否が誠実）。
- **modifier 写像** — `GuiModifiers` と `KeyModifier` の bit
  位置は異なる設計を選んだ（境界が VST3 レイアウトに引きずら
  れない）。adapter が写像を一手に担う。

## Deferred Design Concerns

- `SendGuiKey`/`SetGuiFocus` の wire encoding + `L1EditorOps`
  対応 — out-of-process editor verb slice で扱う。
- CLAP の view-input 対応（`clap` adapter の hook は既定
  `Unsupported` のまま）。
- IME composition 経路 — `real-window-ime` と同じく Issue #389
  scope。
- modifier key のみの key event（`key: char` が printable を
  想定するため、`key_code` のみの修飾キー単体押下は未表現）。

## Verification Notes

- plugin-core: `cargo check` + clippy。
- plugin-host: `cargo test -p ruviform-plugin-host` —
  `editor_session` 契約テスト 57（`send_key`/`set_focus` の
  `InvalidState`・`Ops(Unsupported)` non-fatal・L1 refusal を
  含む）。
- plugin-vst3: 44 tests — modifier 写像・`InvalidValue`・
  `Unsupported`/`PluginFailed` mapping。
- plugin-test-vst3: event log の順序と title restamp の unit
  tests。
- plugin-dock-test: `real-dock` feature で 17+14+5 tests —
  catalog/entries の 26-row pin、console `9+26`。
- 実走: `mise run test-ui-plugin-dock-real`（interactive）と
  `test-ui-plugin-dock-real-headless`（blocked-probe）で
  evidence 再生。
