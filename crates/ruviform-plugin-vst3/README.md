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

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md
- 実装 spec/plan（task T4）: docs/specs/implement-plugin-foundation.md, docs/plans/implement-plugin-foundation.md
- binding 方式 ADR: docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md
- PoC 所見: docs/impl/ir/poc-vst3-binding-adapter.md
- Issue #29
