---
id: "SPEC-873pgGuiReqChCtrct02"
type: "spec"
status: "approved"
title: "Issue #873 plugin→host GUI 要求チャンネル — 中立語彙・配送 seam・形式別 verdict 契約"
created: "2026-11-23"
updated: "2026-11-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/873"
    - "docs/discovery/issue-873-plugin-gui-request.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-873-plugin-gui-request.md"
  implements:
    - "docs/discovery/issue-873-plugin-gui-request.md"
  implemented-by:
    - "docs/designs/issue-873-plugin-gui-request.md"
  depends-on:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  blocks: []
---

# Issue #873 plugin→host GUI 要求チャンネル — spec

## 背景

Issue #389 の AC「plugin-side preferred size/resize 拒否」には、plugin 発の
GUI 要求を host 側へ届け裁定する経路が必要だが、現状は host→plugin verb のみ
存在する（discovery 棚卸し参照）。本 spec は中立の要求語彙・配送 seam・
形式別の verdict 契約を要件として定める。責務境界の裁定と実装対応の正本は
`docs/designs/issue-873-plugin-gui-request.md`。

## 要件

### R1: 中立の要求・裁定語彙（AC1）

`ruviform-plugin-core::boundary` に plugin 発 GUI 要求の中立 enum
`GuiRequest`（`Resize{size: GuiSize}`/`Show`/`Hide`/`ResizeHintsChanged`/
`Closed{was_destroyed}`）と裁定 verdict `GuiRequestVerdict`
（`Accepted{granted: Option<GuiSize>}`/`Refused{reason}`）、拒否理由語彙
`GuiRequestRefusal`（`Unrouted`/`Rejected`/`Busy`/`Gone`）を定義する。
全公開 enum は `#[non_exhaustive]`（repo 規約）。中立語彙に
`ruviform-ui` 型・raw OS handle・plugin-standard 具象型
（`ViewRect`/`ClapGuiSize`/`IPlugFrame` 等）を出さない。

### R2: plugin 側配送面（AC1）

`Plugin`/`ErasedPlugin` に 2 つの default hook を定義する:

- `set_gui_request_sink(Option<Box<GuiRequestSink>>)` — 同期 verdict 形式
  （VST3）用に host が install/detach する `FnMut(GuiRequest) ->
  GuiRequestVerdict + Send` の sink。
- `drain_gui_requests(&mut Vec<GuiRequest>)` — async ack 形式（CLAP
  `[thread-safe]` verb）用に adapter 側 bounded queue から要求を取り出す
  pull 経路。

要求面は `HostCommand` variant にしない（host→plugin 命令語彙と方向が逆）。

### R3: session/ops 境界の裁定 seam（AC1）

`EditorSessionOps` に `drain_gui_requests`/`adjudicate_gui_request`/
`dispatch_gui_requests` を default 付きで追加し、構成側（GUI driver）が
注入する裁定 callback `GuiRequestArbiter` へ要求を配送する。裁定の終端
（実際に slot geometry を動かす側）は ui/ui-gpui driver 側の別範囲 —
本 spec は session 境界に露出する裁定 seam までを要求範囲とする。
`L1EditorOps` は全 verb `Err(Unsupported)` を維持する（L1 GUI verb は
#872 範囲）。

### R4: verdict 契約（AC2 — defined refusal の解消）

- `Accepted` は container-side effect 確定のみを意味し、effect 未確定で
  `Accepted` を返さない。`granted` は resize 系で container が実際に適用
  した size（補正 `Some`/そのまま適用 `None`）。通知 variant の `Accepted`
  は受理 ack であり grant 意味を持たない。
- **VST3**: `resizeView` → sink → arbiter 同期 verdict。`Accepted` →
  `view.onSize(granted or requested)` 往復完結で `kResultOk`。
  `Refused`/経路未接続/re-entrant → `kResultFalse`。
- **CLAP**: `request_*` → shared bounded queue へ enqueue → `Ok(())`
  （enqueue 受理 ack — 裁定 verdict ではない）。queue full/poisoned →
  `Err` → ABI `false`。`resize_hints_changed`/`closed` は通知 — full 時
  defined drop。
- arbiter 競合の写像: `try_lock` 失敗 → `Refused{Busy}`、poisoned →
  `Refused{Gone}`。re-entrant FFI は panic でなく定義済み応答。
- `docs/designs/issue-389-vst3-l0-editor-attach.md` D2 の「`resizeView`
  無条件 `kResultFalse`」は「request route なし」時のみの応答へ本契約が
  更新する（design D8 で宣言）。

### R5: 検証（AC3）

- `crates/ruviform-plugin-host/tests/` で scripted plugin + real
  `InProcessHost`/`InProcessEditorOps` + scripted arbiter の headless
  roundtrip: 要求配送・accept 時 verdict・refuse 時 defined refusal・
  refuse で plugin 側 verb 非実行・L1 `Unsupported` を検証。
- adapter 側: VST3 `resizeView` の accept → `kResultOk`（+ `onSize` 往復）/
  refuse・unrouted・re-entrant → `kResultFalse`、null 引数 →
  `kInvalidArgument`。CLAP `HostGuiImpl` enqueue → drain 写像と bounded
  queue の拒否。
- verdict 語彙は `Pass`/`Fail`/`NotRun`/`Unsupported`。
  **accepted + effect confirmed のみ Pass**。real-dock lane への影響は
  非回帰確認（既存行の verdict を変えない）— plugin 発要求の real 行追加は
  本 slice 対象外。
