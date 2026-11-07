# ruviform-plugin-vst3

Ruviform プラグイン基盤の VST3 規格アダプタ（Issue #29、T4 binding PoC）。
中立 `ruviform_plugin_core` 契約を包む VST3 モジュール（`.vst3` cdylib）を
構築する — ステレオ入力 1 バス・ステレオ出力 1 バス・正規化 gain パラメータ
1 つの PoC gain プラグイン。

## 責務

- 全 `vst3` クレート binding 型は非公開 — このクレートは公開 Rust API を
  エクスポートせず、ホストは ABI エントリポイント経由でのみプラグインへ到達する。
- エントリポイント（VST3 モジュール契約のシンボル。モジュール内で衝突しない）:
  - `GetPluginFactory`（全プラットフォーム、規格必須）
  - `InitDll`/`ExitDll`（Windows — 規格上 optional、ローダパスを均一にする
    trivially-true フックとして実装）
  - `bundleEntry`/`bundleExit`（macOS — 構造的に必須。実 bundle 配線は
    パッケージング課題で PoC 範囲外）
  - `ModuleEntry`/`ModuleExit`（Linux — 必須、trivially-true フック）

### process パス（RT）

`process` は境界で `ProcessData` を検証する（`numSamples`、`kSample32` のみ —
`kSample64` は再解釈せず reject、チャネル数、ポインタ null/alignment、
チャネルペアごとの alias 検出）し、ホストの `channelBuffers32` 上に
`ruviform_audio_core::PlanarBufferView` ビューを介したゼロコピーで
`GainProcessor` を駆動する。インプレースエイリアシングは T2 承認の
bounce-scratch 戦略（チャネルごと bounce + `finish_block` 書き戻し）で処理する。
チャネルリスト・イベントスクラッチは `setupProcessing` で確保する
インスタンスごとの状態であり、process パスは有界処理のみで確保・
ブロック・I/O を行わない。

## 層・許可依存

- 層: 拡張層（Plugin 拡張層、設計ベースライン）
- 許可依存: `ruviform-plugin-core` + Domain foundations（`ruviform-audio-core`）+
  Primitives（`ruviform-media-core`）、および `vst3` binding クレート
  （アダプタ内部の実装依存）。

## RT 契約

`process` パスは RT コールバック: 境界検証 + ゼロコピー駆動 + 有界スクラッチ
処理のみで、確保・ブロック・I/O を行わない。Issue #22 検証レーンの
`assert_no_alloc` で境界の allocation/deallocation を機械検出する。

## feature `host` — 規格 host-side bridge + L1 runner（Issue #114）

default feature 集合に含まない `host` を有効化すると、host 側 bridge
`ruviform_plugin_vst3::host` が公開される。`Vst3Library::load(path)` が実
VST3 モジュールを `libloading` で dlopen し、raw `vst3` COM bindings 直駆動で
class scan（`"Audio Module Class"` filter）→ `createInstance` →
`IComponent`/`IAudioProcessor` + `IEditController` を中立
`PluginFactory`/`ErasedPlugin` 契約へ写像する（`PluginDescriptor`/`ParamInfo`
の `'static` 境界は load/factory 構築時の 1 回限り bounded `String::leak`/
`Vec::leak` で実体化）。`save_state`/`restore_state` は `Plugin` 既定の
`Err(HostError::Unsupported)` のまま（fixture の `setState`/`getState` は
no-op stub・偽の round-trip を観測しない）。`open_gui`/`close_gui` は
Windows `kPlatformTypeHWND` に対して実装済み —
`createView("editor")` → `isPlatformTypeSupported` →
`setFrame(BridgePlugFrame)` → `attached(parent HWND)`、close は
`removed()` → release（Issue #389 ・設計
`docs/designs/issue-389-vst3-l0-editor-attach.md`）。distributed plugin
向けに component↔controller の `IConnectionPoint` 双方向接続と host
`createInstance` の `IMessage`/`IAttributeList` 提供を行う。他 platform・
`resizeView` 受理は honest `Unsupported`/defined refusal のまま。

descriptor `id` は component CID（`GainComponent::CID` = `vst3::uid(
0x5A1B2C3D, 0x4E5F6A7B, 0x8C9D0E1F, 0x2A3B4C5D)`）raw 16 byte の小文字 hex
— `uid()` byte order は **platform 依存**（Windows `3d2c1b5a5f4e7b6a8c9d
0e1f2a3b4c5d`・非 Windows `5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d`）のため、
期待値は test が `uid()` bytes から実行時に derive する（`#[cfg]`-conditional
literal は regression pin のみ）。

runner bin `ruviform-plugin-vst3-l1-runner`（`[[bin]] required-features =
["host"]`）は compile-time 固定 — fixture cdylib
`{DLL_PREFIX}ruviform_plugin_vst3{DLL_SUFFIX}` を `RUVIFORM_L1_REAL_PLUGIN_DIR`
（任意 env override）→ 未設定時は runner executable sibling
（`target/<profile>/`）で解決し、スキャン済み factory を
`ruviform_plugin_export::run(factories)` へ渡す。wire v2 の `run` argv 規約は
`<runner>` のみ（v1 の `--ruviform-shm-key`/`--ruviform-plugin-id` bootstrap
flag は廃止 — plugin id/shm key は `CreateInstance` payload が運び、
`--ruviform-*` を含む任意引数は usage reject → exit 2）。`spec.args` は空を
維持し、plugin path を argv で受け渡す機構はない（load 失敗は `eprintln!` +
exit 1 pre-Hello → 親 spawn `Err(HostError::Ipc)`）。factory 選択は
`CreateInstance` の `plugin_id` のみ — 同一 id の factory を 1 binary に 2 つ
内蔵するのは authoring error。

検証入口は opt-in lane のみ: `mise run test-l1-real-plugin` が
`cargo build --features host`（fixture + runner 生成）→ `cargo test --lib
--bins`（in-process bridge 契約層 + runner bin の fixture 解決 unit test）
→ `cargo test --test l1_real_adapter_e2e -- --ignored`
（`PluginHost`/`OutOfProcessSpec` 経由の実バイナリ L1 E2E — `#[ignore]`
ゲートで既定 `cargo test` 経路から分離）の 3 段を実行する。
lint は `mise run lint-l1-real-plugin`（`--features host`・`-D warnings`）。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md
- 実装 spec/plan（task T4）: docs/specs/implement-plugin-foundation.md, docs/plans/implement-plugin-foundation.md
- binding 方式 ADR: docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md
- PoC 所見: docs/impl/ir/poc-vst3-binding-adapter.md
- Issue #29
