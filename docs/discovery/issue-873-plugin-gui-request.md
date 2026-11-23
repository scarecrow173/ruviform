---
id: "DSC-873pgGuiReqChInv01"
type: "discovery"
status: "final"
title: "Issue #873 plugin→host GUI 要求チャンネル — 現状棚卸しと設計判断記録"
created: "2026-11-23"
updated: "2026-11-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/873"
    - "https://github.com/scarecrow173/ruviform/issues/389"
  related:
    - "../specs/issue-873-plugin-gui-request.md"
    - "../designs/issue-873-plugin-gui-request.md"
    - "../test-specs/issue-873-plugin-gui-request.md"
---

# Issue #873 plugin→host GUI 要求チャンネル — 現状棚卸し

Issue #389 の AC「plugin-side preferred size/resize 拒否」受入ケースに必要な
plugin→host 方向の GUI 要求チャンネルが存在しない、という出発点の棚卸し記録。
要求の受理・裁定主体（adapter ↔ session ↔ GUI driver）と語彙の置き場所を
確定するための一次調査。結論の正本は
`docs/designs/issue-873-plugin-gui-request.md`。

## 現状棚卸し（調査時点の HEAD `97991329`）

- `crates/ruviform-plugin-vst3/src/host.rs:720-742` — `BridgePlugFrame` は ZST
  （`struct BridgePlugFrame;`）、`resizeView` は無条件 `kResultFalse`。
  `docs/designs/issue-389-vst3-l0-editor-attach.md` D2 の defined refusal:
  container geometry は slot 側（`NativeSurfaceSlot`/`set_geometry`）所有で、
  中立の要求経路がないため受理を偽装しない。
- `crates/ruviform-plugin-clap/src/host.rs:313-343` — `BridgeShared` は
  `request_restart`/`request_process`/`request_callback` のみの acknowledge。
  `declare_extensions` は `HostGui` を宣言しないため plugin 側の
  `clap_host_gui` verb は到達不能。
- `crates/ruviform-plugin-host/src/editor_session.rs` — `EditorSessionOps` は
  `open`/`close`/`set_size`/`set_visibility`/`reattach`/`set_focus`/`send_key`
  の host→plugin verb のみ。plugin→host 方向の配送面・注入 seam は
  `LeaseRedeemer`（parent 払い戻し用）のみで、要求裁定の seam はない。
- `crates/ruviform-plugin-host/src/command.rs` — `HostCommand` 全 variant が
  host→plugin 命令語彙。plugin→host 要求を乗せると方向が混ざる。
- 要求を発火する fixture/plugin が存在しない — test-vst3 `editor_view.rs` は
  `IPlugFrame` を保持するが `resizeView` を一度も呼ばず、test-clap
  `editor.rs` に `request_*` 呼出なし。

## 形式間の非対称（設計の核）

- **VST3 `IPlugFrame::resizeView`** — plugin の UI スレッド上の同期呼出で
  即答必須。accept 時は container effect 確定 + `view.onSize` 往復を持って
  `kResultOk` とするのが Steinberg 契約。
- **CLAP `clap_host_gui`** — `request_resize`/`request_show`/`request_hide` は
  `Result<(), HostError>` → ABI bool。`[thread-safe]` で任意スレッドから
  呼ばれ得、async ack が許容（clack-extensions `HostGuiImpl` doc:
  "a successful return value may only mean the Host acknowledged the
  request, and will process it asynchronously later"）。
  `resize_hints_changed`/`closed(was_destroyed)` は戻り値なしの通知。

## 責務境界の裁定肢と調査判断

| 肢 | 内容 | 判定 |
| --- | --- | --- |
| (A) | session 境界 arbiter callback（注入 seam）— `LeaseRedeemer` と同型に構成側が裁定 callback を session/ops に注入し、adapter → host → session → arbiter へ配送 | **採用（queue 併用）** — 同期 verdict を返せ、VST3 即答契約と整合。CLAP は queue → drain → 同じ arbiter へ |
| (B) | drain queue 単独 — `drain_events`/`drain_output_reports` 同型の pull 経路 | 棄却 — VST3 `resizeView` の即答契約には queue だけでは honest な `kResultOk` を返せない |
| (C) | adapter 内裁定 | 棄却 — container geometry は slot 側所有（#388・389 D2）。`Accepted` は effect 確定を意味するため effect 権限のない adapter は裁定主体にできない |

## 中立語彙の置き場所

- `GuiRequest`/`GuiRequestVerdict`/`GuiRequestRefusal` は
  `ruviform-plugin-core/src/boundary.rs` に配置（条件付き lease 採用）。
  `GuiSize`・`GuiKeyEvent`・`ParentSlotLease` と同じ plain-data 語彙の置き場。
  adapter は plugin-host に依存しないため plugin-host 内に閉じると名指せず、
  #872 の将来 wire 化で二重定義も生む。
- `GuiRequestSink`（`dyn FnMut` trait object）は plain-data でないため
  `plugin.rs` に配置。

## roundtrip 検証の起点

- 採用: `crates/ruviform-plugin-host/tests/` で scripted fake ops + scripted
  arbiter の headless roundtrip + adapter 側は `BridgePlugFrame` /
  `BridgeShared` を COM/Rust double で直接検証。
- 不採用: test-vst3/test-clap fixture への要求発火追加（実 ABI 経由）と
  real lane 行追加 — `real_catalog.rs`/`real_dock.rs` は
  `scarecrow173/ruviform#874` が占有するため本 slice 対象外（前 Worker の
  不採用確定を踏襲）。
