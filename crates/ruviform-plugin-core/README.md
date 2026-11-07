# ruviform-plugin-core

Ruviform メディア基盤の規格中立プラグイン基盤クレート（Issue #29）。
CLAP・VST3・binding クレート（`clack`, `vst3`, `clap-sys` 等）由来の型・概念は
**非目標** — ここには規格を名指し・wrap するものは何も置かない。

## 責務

- プラグイン記述: `PluginDescriptor`
- パラメータ記述/値: `ParamInfo`, `ParamValue`, `ParamFlags`, `ParamId`
- サンプル精度イベント: `Event`, `EventKind`, `EventList`
- オーディオバス記述: `BusDescriptor`, `BusDirection`
- 規格中立トランスポートスナップショット: `TransportInfo`
- 共通ライフサイクル状態機械: `Lifecycle`, `LifecycleState`, `LifecycleEvent`,
  `LifecycleError`
- プラグイン処理契約: `PluginProcessor` と借用ビュー契約（`BufferView`,
  `BufferViewMut`）、支援型（`ProcessConfig`, `ProcessContext`,
  `ProcessResult`, `ProcessStatus`, `PrepareError`）
- 借用記述ビュー: `PluginDescriptorView`（owner より長生きしない pure borrow）
- typed plain パラメータチャネル: `PlainParamValue`, `PlainParamInfo`,
  `PlainParamInfoOwned` — 規格中立の typed 制御（normalized wire 経路ではない。
  `PlainParamInfoOwned` は `String`/`Vec` ストレージを持つ所有形 — video L1
  wire 経路（`GetParamInfo`/`ParamInfo`）がそのまま運ぶ返り値型）
- 動画プラグイン契約: `VideoProcessConfig`, `VideoPluginInstance` —
  **全操作 NONRT**（`render` 含む、audio `process` の RT 契約とは別経路）。
  adapter が登録する object-safe erased 面は `video_plugin` モジュール —
  `VideoPluginInstance` ↔ `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video`
  （audio 側の `Plugin` ↔ `ErasedPlugin`/`PluginFactory`/`erase` と同型の対応）
- ホスト境界語彙（`boundary` モジュール）: `PluginHandle`,
  `GuiHandle`, `GuiParentToken`, `PluginEditorSessionId`,
  `ParentSlotLease`（Issue #389 の session 面 opaque parent 語彙 —
  raw `GuiParentToken` を置き換えず、OS adapter 内 redemption の
  session 側対応物）、`HostError`
- L1 wire プロトコル（`wire` モジュール — アウトプロセス経路の codec 正本）:
  手書き LE codec・`Message`/`Command`/`Reply`/`Descriptor` 型・`WIRE_VERSION`
  （現行 **3** — Issue #117 が v2 で `instance` envelope・`CreateInstance`/
  `DestroyInstance` を、Issue #118 が v3 で video `Command` tag 12-22・
  `Reply` tag 4-7・`HostError` tag 10 `InvalidValue`・`VideoFrameDesc`/
  `PlainParamInfoOwned` codec・`WIRE_VIDEO_MAX_*` 上限を additive 追加。
  audio tag/envelope/scope 規則は不変）・`WIRE_MAX_*` 安全上限・shm 定盤
  2 系（audio `wire::shm` / video `wire::vshm` — 別レイアウト・別マジック）

規格アダプタ（`ruviform-plugin-clap` / `ruviform-plugin-vst3`）は規格 ABI
データをこれらの型へ翻訳して `PluginProcessor` を実装・駆動する —
アダプタ自身が処理契約を定義することはない。具体的なチャネルごと借用
プレーナビューは `ruviform_audio_core::PlanarBufferView`/`PlanarBufferViewMut`
— `process` は `BufferView` 契約上のジェネリックで、アダプタはホストバッファを
独自実装で包んでもよい。

## 層・許可依存

- 層: 拡張層（Plugin 拡張層、設計ベースライン）
- 許可依存: Domain foundations（`ruviform-audio-core`・`ruviform-video-core`）+
  Primitives（`ruviform-media-core`）のみ。`ruviform-rt` は意図的に依存しない —
  `RtContext` 能力の受け入れは RT 契約実装 Issue に委譲し、コンテキスト引数は
  不透明な `ProcessContext` とする。同階層・上位階層への依存は禁止。

## RT 契約

`process` は RT コールバック — alloc/dealloc・ブロッキング・I/O・無界の
待機/再試行・無界の処理を行わない。`prepare` は非 RT のアクティベーションフック。
値型は `Copy` でアロケーションなし、`EventList::new` は有界スキャンで検証し
確保しない。API ごとの保証は各項目の `**RT contract:**` 行に記載される。
`rt_` プレフィックスのテストが RT 契約レーン（`mise run test-rt`）を構成するが、
アロケーションは観測できない — Issue #22 RT 検証レーンが来るまで no-alloc は
文書化契約 + レビュー対象。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2、「プラグインアーキテクチャ」）
- プラグインアーキテクチャ ADR: docs/adr/plugin-architecture-clap-vst3.md
- RT 契約 ADR: docs/adr/realtime-contract-and-verification-policy.md
- 実装 spec/plan: docs/specs/implement-plugin-foundation.md, docs/plans/implement-plugin-foundation.md
