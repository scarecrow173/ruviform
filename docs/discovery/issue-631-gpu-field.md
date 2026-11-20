---
id: "DISC-g9Hv3kLm6Qp2rT8wXc5Nb"
type: "discovery"
status: "draft"
title: "Issue #631 AC4 'OS and GPU' display-adapter field"
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
  related:
    - "../specs/issue-631-gpu-field.md"
    - "../designs/issue-631-gpu-field.md"
    - "../adr/issue-631-gpu-field.md"
    - "../discovery/issue-631-evidence-fields.md"
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

# Issue #631 AC4 'OS and GPU' display-adapter field

## Summary

Issue #631 AC4 の 'OS and GPU' field の残り部分 — GPU 側の記録。
前 slice（`issue-631-evidence-fields`、PR #764）は GPU 情報を
「manifest/API gate で別件残件」と位置付けていたが、Coordinator
決定により **GDI の `EnumDisplayDevicesW` ベースの
display-adapter 記録で充足**と確定した。DXGI / GPUI
renderer-info は不要。既存の optional `windows-sys = "0.61"`
依存が有効化済みの `Win32_Graphics_Gdi` /
`Win32_UI_WindowsAndMessaging` feature に必要 API が全て収まる
ため、manifest / 依存 / feature 変更なしで実装できる（confirmed
— windows-sys 0.61.2 のソースで `EnumDisplayDevicesW`・
`DISPLAY_DEVICEW`・`EDD_GET_DEVICE_INTERFACE_NAME` の有効化を
確認済み）。

## Exploration Goal

- real lane の session `environment` に display-adapter census
  を記録する（field 名 `display_adapters`）。
- 列挙ソースは `EnumDisplayDevicesW`: `idevnum` を 0 から列挙し、
  各 `DISPLAY_DEVICEW` の `DeviceName` / `DeviceString` /
  `DeviceID` / `DeviceKey` / `StateFlags` を記録する。
- `EDD_GET_DEVICE_INTERFACE_NAME` の第2段で adapter ごとの
  monitor 対応付けを記録する。
- headless / blocked-probe 経路は session 不在のため刻まない —
  fail-closed を維持し、空配列や偽値を捏造しない。

## Findings

### 配置規約（既存パターン踏襲）

`real_dock.rs` には既に `session_scale_factor` と
`window_identities` を session 開始時に一度だけ採取し、
`finalize_case` の merged extras 経路で全 real case の
`environment` に刻む仕組みがある。本 slice は同じ経路に
`display_adapters` を追加する。blocked 経路
（`blocked_reports`）には触れない — session-scoped field は
session 存在時のみ記録する契約（PR #764 の fail-closed 契約）
をそのまま維持する。

OS 観測の補助 module は `ospix.rs`（pixel capture）・
`ospres.rs`（presentation inventory）・`osoccl.rs`（occlusion）・
`osmut.rs`（bounded mutation）と同じ `src/os*.rs` 配置規約に
従い、`src/osgpu.rs` を新規追加する。Win32 型は module 境界を
越えず、呼び出し側は driver-neutral な Rust 値と JSON を受け取る。

### API 確認

windows-sys 0.61.2 で以下が有効 feature の範囲で利用可能:

- `EnumDisplayDevicesW`・`DISPLAY_DEVICEW`・`DISPLAY_DEVICE_STATE_FLAGS`
  → `Win32_Graphics_Gdi`（有効済み）
- `EDD_GET_DEVICE_INTERFACE_NAME` →
  `Win32_UI_WindowsAndMessaging`（有効済み）

`idevnum` 列挙が `FALSE` を返した時点で列挙終端。`GetLastError`
は列挙終端では値を返さない規約のため、初回呼び出し（`idevnum
= 0`）の失敗のみを error として記録し、それ以降の `FALSE` は
正常終端として扱う。

### 識別の限界（正直な記録として明記）

GDI の display-adapter 列挙は OS の表示デバイス view であり、
GPUI / compositor が実際にレンダリングした adapter（DXGI
adapter）を一意に証明しない。`display_adapters` field には
`source: "EnumDisplayDevicesW"` を刻み、renderer 選択 adapter
の証明ではないことを README・spec・ADR に明記する。

### stale コメント（同梱の軽微残件）

`src/lib.rs` の AC1 残件コメントが「DPI・window-close は
Unsupported」と記したまま残っていたが、現状は両者とも実
verdict 化済みで `Unsupported` は `real-window-ime` のみ。
本 slice の lease 内ファイルのため現状追従で修復する。

## Decisions Recorded

- field 名: `environment.display_adapters`（`environment_kind`・
  `window_identities` と同じ session-scoped extras の命名規約に
  合わせ複数形・役割名）。
- monitor 列挙には `EDD_GET_DEVICE_INTERFACE_NAME` を使用
  （device interface 名を `DeviceID` に得る）。
- 列挙 safety cap: adapter 64・monitor/adapter 16（実セッション
  の display device 数を大きく超える値。adapter cap 到達時は
  `truncated: true` を記録）。
- verdict ロジック・catalog 件数・`real-dcomp-pixel` の既知
  Fail（`ruviform#532`）は不変。本 field は記録のみ。
