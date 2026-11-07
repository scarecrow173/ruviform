# ruviform-poc-native-dock

Issue #388/#389 Native Dock 早期 Gate の利用側層 PoC harness。

## 責務

- アーキテクチャ baseline（plugin 系クレートと ui 系クレートの相互依存禁止、
  plugin GUI 結線は利用側層）に従い、本クレートが唯一の composition root と
  して `GpuiSurfaceAdapter`(ui 側)・`PluginEditorSession`/`InProcessHost`
  (plugin 側)・`LeaseRedeemer` seam を結線する。
- 非 default `gpui` feature で real GPUI window 上に probe plugin の実
  native editor（Win32 `WS_CHILD`）を attach し、`NativeSurfaceSlot` の
  declare/lease/redeem/geometry/visibility/focus/release を実 OS で
  検証する手動入口。
- 本番 API なし。headless(feature off)では拒否 message を出して exit 2。

## 実行

```sh
# Windows 実 OS session のみ（gpui feature + Win32 surface backend）
cargo run -p ruviform-poc-native-dock --features gpui
```

証拠: `evidence/rf388-389-native-dock/`（実 run log + report）。

## 責務境界

- plugin 系 ↔ ui 系の直接依存は持たない。session↔surface の redeem
  (`ParentSlotLease` → `GuiParentToken`) は本クレートの redeemer が
  adapter の live-lease 検証経由で行う。
- `raw-window-handle`/`HWND` などの OS 型は公開 API に露出しない
  （adapter 内・opaque `u64` token のみ境界越え）。
- Dock PASS ではない: 実 dock layout widget は未実装であり、本 harness は
  GPUI window 内 region への実埋込みまでを証拠化する。
