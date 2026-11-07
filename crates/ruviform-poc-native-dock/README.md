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
# probe plugin（in-process 決定論 fixture）— 既定経路
cargo run -p ruviform-poc-native-dock --features gpui

# 実外部 VST3 module — vst3 feature + RVF_POC_VST3_PATH で module 指定
# （mise 正規入口: mise run poc-vst3-editor）
RVF_POC_VST3_PATH='C:/Program Files/Common Files/VST3/iZotope/Vinyl.vst3' \
  cargo run -p ruviform-poc-native-dock --features gpui,vst3
```

### `vst3` feature（実外部 VST3 module lane）

- `ruviform-plugin-vst3`（`host` feature）を optional 依存として有効化。
  `RVF_POC_VST3_PATH` を設定すると、既定の in-process probe ではなく
  実 external `.vst3` binary を `Vst3Library::load` → class scan →
  `Vst3PluginFactory` 登録 → `InProcessHost` instantiate/initialize で
  実 module 経路を通す。`createView`→`attached(HWND)` まで実 VST3
  editor を lease 済み container へ attach する。
- 環境変数:
  - `RVF_POC_VST3_PATH` — `.vst3` module の絶対パス（必須）。
  - `RVF_POC_VST3_PLUGIN_ID` — 複数 class 同梱時の選択 ID（任意。
    hex `class.id`、未指定時は先頭 descriptor）。
- `RVF_POC_VST3_PATH` 未設定時は `vst3` feature が有効でも既存の
  in-process probe 経路にフォールバックする（決定論 lane 不変）。
- 実 editor HWND は adapter 内部所有のため `EnumChildWindows` で
  container の子 window を列挙して観測する（probe の shared state
  経路とは別）。
- `load` 失敗は `vst3_load=fail` を吐いて exit 3。`createView` が
  null を返す plugin（editor 非搭載）は `open=fail
  error=Ops(Unsupported)` として honest に分類し、attach 成功とは
  記録しない。

証拠: `evidence/rf388-389-native-dock/`（probe）、
`evidence/rf389-vst3-editor/`（実外部 VST3）（実 run log + report）。

## 責務境界

- plugin 系 ↔ ui 系の直接依存は持たない。session↔surface の redeem
  (`ParentSlotLease` → `GuiParentToken`) は本クレートの redeemer が
  adapter の live-lease 検証経由で行う。
- `raw-window-handle`/`HWND` などの OS 型は公開 API に露出しない
  （adapter 内・opaque `u64` token のみ境界越え）。
- Dock PASS ではない: 実 dock layout widget は未実装であり、本 harness は
  GPUI window 内 region への実埋込みまでを証拠化する。
