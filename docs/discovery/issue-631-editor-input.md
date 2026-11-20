---
id: "DISC-01M7WJTWYSRTVAMEGHYE8WA4F4"
type: "discovery"
status: "resolved"
title: "Issue #631 editor input/focus verb 面 — EditorSessionOps 拡張と fixture の OS-observable 効果チャンネル"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 editor input/focus verb 面 — EditorSessionOps 拡張と fixture の OS-observable 効果チャンネル

## Summary

Issue #631 AC3 の editor verb 面には `set_size`/`set_visibility`/
`reattach` までが実判定済みで、`send_key`/`set_focus`（input/focus）
だけが残っていた。`EditorSessionOps` には input/focus の verb が
存在せず、`test_editor` fixture も `onKeyDown`/`onKeyUp`/`onFocus`
を `kResultOk` で握り潰していたため、real lane が「verb が届いた」
ことを観測する手段がなかった。本 slice は

- 境界語彙 `GuiKeyEvent`/`GuiKeyDirection`/`GuiModifiers` を
  plugin-core に追加し、
- `Plugin`/`ErasedPlugin` に `set_gui_focus`/`send_gui_key` hook
  （既定 `Unsupported`）、
- L0-only `HostCommand::SetGuiFocus`/`SendGuiKey`（wire 非対象・
  wire version 不変）、
- `EditorSessionOps::set_focus`/`send_key` +
  `InProcessEditorOps` 実装（`L1EditorOps` は全 verb
  `Unsupported` のまま）、
- VST3 adapter の `IPlugView::onFocus`/`onKeyDown`/`onKeyUp`
  forward、
- fixture が受け取ったイベントを child HWND の window text に
  追記する OS-observable 記録、
- `real-editor-input`/`real-editor-focus` 2 case（24→26 行）

を追加し、すべて実装・実走済み（confirmed）。

## Exploration Goal

- `EditorSessionOps` に input/focus verb を追加する際、
  driver-neutral 性（GPUI・HWND・NSView・X11 handle を露出しない）
  を保ったまま VST3 `IPlugView` まで届く最小の seam を確定する。
- 「verb が plugin に届いた」ことを real lane が OS から検証できる
  効果チャンネルを確定する。
- L0-only verb を wire/composite 経路で既存 L0-only verb と同じ
  `Unsupported` に写像する guard のあり方を確定する。

## Key Issues and Assumptions

- VST3 `IPlugView::onFocus(TBool)` /
  `onKeyDown(char16, int16, int16)` / `onKeyUp(...)` は view が
  live のときだけ意味を持つ — view 未 open 時は defined
  `Unsupported` で答える（confirmed）。
- VST3 の `char16` は UTF-16 の 1 コードユニットで、
  supplementary-plane 文字は surrogate pair が必要だが verb が
  pair を運ばない — 表現不能な key は `InvalidValue` で拒否し、
  truncate しない（confirmed）。
- `GuiModifiers` の 4 bool 設計は
  `clippy::struct_excessive_bools` に抵触したため、u16 bitmask
  wrapper（`NONE`/`SHIFT`/`ALTERNATE`/`COMMAND`/`CONTROL` +
  `contains`/`union`）へ reshape 済み — plugin format の実際の
  表現と同じ形で、VST3 `KeyModifier` bit への写像も per-flag で
  行う（confirmed）。
- fixture の event log は child HWND の window text
  （`GetWindowTextW`）に追記する方式を採用した — attach
  generation tag（`rvf-test-editor-attach-{n}`）が既に同じ
  channel で読まれており、新規 API・共有メモリ・ファイルを
  一切増やさない（confirmed）。
- input/focus は non-fatal verb（`set_size` と同じ契約）:
  plugin の拒否は `Ops(Unsupported)` として返り、session は
  `Attached`/`Hidden` のまま遷移しない（confirmed —
  `editor_session.rs` `set_focus`/`send_key`）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| fixture が受信 event を共有メモリ/名前付き file に記録 | 任意データを運べる | 新規 IPC・cleanup・同期待ちが増える。lane 側の観測も新しい channel が要る | no |
| fixture が child HWND の window text に event token を追記 | 既存 `GetWindowTextW` 観測をそのまま使い回せる。追加 API ゼロ。順序付き log になる | title 長に上限がある（fixture 用途では問題なし） | yes |
| lane が `IPlugView` spy を別途注入 | plugin 側変更ゼロ | production `Vst3Plugin` の型差し替えが必要で lane の fidelity が下がる | no |
| `GuiKeyEvent` に raw OS key code を持たせる | driver が直接渡せる | driver-neutral 境界に OS 依存値が混入する | no |
| key を `char` + `key_code: i16` + `GuiModifiers` bitmask で運ぶ | VST3/CLAP 双方が表現できる共通分母。driver-neutral | 補助平面文字は `InvalidValue` 拒否が必要 | yes |

## Tentative Conclusions and Hypotheses

- confirmed: 境界語彙は `GuiKeyDirection::{Down,Up}`
  （`#[non_exhaustive]`）、`GuiModifiers` u16 bitmask wrapper、
  `GuiKeyEvent{direction,key:char,key_code:i16,modifiers}`。
- confirmed: `HostCommand::SetGuiFocus`/`SendGuiKey` は L0-only —
  `to_wire` に到達せず、composite の out-of-process guard が既存
  L0-only verb と同じ `Unsupported` を返す。wire version 不変。
- confirmed: session verb は non-fatal — `set_focus`/`send_key` は
  `Attached`|`Hidden` 必須で `InvalidState` 以外は state を
  変えない。
- confirmed: real lane の effect oracle は child HWND title suffix
  （`|kd:…`,`ku:…` / `|f:…`）— 画素ではなく OS の window text。
- confirmed: `L1EditorOps` は input/focus を含め全 verb で
  `Unsupported` のまま — out-of-process editor verb は本 slice
  の外。

## Open Questions

- なし（実装・実走済み。残課題は Deferred Design Concerns として
  spec/design に記録）。

## Promotion Candidates

- `real-editor-input`/`real-editor-focus` の 2 catalog 行
  （`real_dock.rs` `real_entries` + `real_catalog.rs`
  `REAL_CATALOG`）— 実装済み。
- plugin-core boundary 語彙の追加 — 実装済み。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — no auto-close）。
- 先行: `docs/specs/issue-631-real-editor-attach.md`（child HWND
  OS 観測の確立）、`docs/designs/issue-631-real-row.md`（real
  catalog 行の判定写像）、`docs/designs/issue-389-vst3-l0-editor-attach.md`
  （L0-only verb と thread-affinity 契約）。
- fixture: `crates/ruviform-plugin-test-vst3/src/editor_view.rs`
  （`onFocus`/`onKeyDown`/`onKeyUp` の記録先）。
- 境界: `crates/ruviform-plugin-core/src/boundary.rs`、
  `crates/ruviform-plugin-host/src/editor_session.rs`。

## Evidence

### 2026-11-20 — input/focus verb 面の実装と実走

- `cargo check`/`cargo clippy -D warnings` が plugin-core、
  plugin-host、plugin-vst3、plugin-test-vst3、plugin-dock-test
  （`real-dock` feature 両構成）で全 green。
- plugin-host editor_session 契約テスト 57 pass、plugin-vst3 44
  pass、plugin-test-vst3 99 pass + 19 群 pass（Windows-only HWND
  テストは ignore のまま）。
- plugin-dock-test は `real-dock` feature で 17+14+5 pass、
  headless console は `9 headless + 26 real` row を確認。
- real lane は interactive session で実走させる
  （`test-ui-plugin-dock-real` — 本環境では headless probe で
  検証）。
