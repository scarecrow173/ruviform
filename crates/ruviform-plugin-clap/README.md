# ruviform-plugin-clap

Ruviform プラグイン基盤の薄い CLAP 規格アダプタ（Issue #29）。
`clack` binding クレート（`clack-plugin`/`clack-common`/`clack-extensions` 0.2.0）
上に構築する。CLAP binding 方式 PoC（task `TASK-034JgsGOtyuy2MkLRLZQI5`）でもあり、
最小 gain プラグインを実際の `clap_entry` cdylib エントリポイントとしてエクスポートする。

## 責務（薄アダプタ）

- 規格エントリポイントのエクスポート（`clack_export_entry!` → `clap_entry`）、
  プラグイン記述子、プラグインファクトリ。
- CLAP ABI データの中立基盤型への翻訳 — オーディオバッファ → プレーナビュー、
  `ParamValueEvent` → `EventList`/`ParamEvent`、`TransportEvent` →
  `TransportInfo`、`ProcessResult` → CLAP process status。
- 共通 `Lifecycle` 機械を CLAP 呼び出し列から駆動し、ホストのプロトコル違反を
  黙って受け入れず共有遷移表で reject する。
- ブロックごとのスクラッチ（チャネルポインタ簿記・イベントスクラッチ）を全て
  アクティベーション時に確保し、RT パスは確保しない。

`process` パスはホスト供給のチャネルごとバッファを
`ruviform_audio_core::PlanarBufferView`/`PlanarBufferViewMut` で包んで
中立 `PluginProcessor` 契約へ**ゼロコピー**で渡す。`PluginProcessor` 契約は
`ruviform-plugin-core` が所有し、本クレートは実装・駆動のみを行う。
binding クレート型は実装詳細 — `clack`/`clap-sys` の型はこのクレートの
公開 Rust API に現れない。

## 層・許可依存

- 層: 拡張層（Plugin 拡張層、設計ベースライン）
- 許可依存: `ruviform-plugin-core` + Domain foundations（`ruviform-audio-core`）+
  Primitives（`ruviform-media-core`）、および `clack` binding クレート
  （アダプタ内部の実装依存。`clack-host` はテスト用 dev-dependency）。

## RT 契約

`process` は RT コールバック — alloc/dealloc・ブロッキング・I/O・無界の
待機/再試行・無界の処理を行わない。インスタンスごとのスクラッチは全て
`activate` で確保し、ブロックごとのパスは有界スキャンのみ行う。
`rt_` プレフィックスのテストが RT 契約レーン（`mise run test-rt`）を構成;
Issue #22 RT 検証レーンはブロックごとパスを `assert_no_alloc` で囲み、
確保/解放を機械検出する。CI 通過は RT 安全性の*証明*ではない — 静的制約
（ロック時間・優先度・実行時間上界）は `docs/adr/realtime-contract-and-verification-policy.md`
に従い文書化 + レビューの責務として残る。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md
- 実装 spec/plan（task T3）: docs/specs/implement-plugin-foundation.md, docs/plans/implement-plugin-foundation.md
- binding 方式 ADR: docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md
- PoC 所見: docs/impl/ir/poc-clap-binding-adapter.md
- Issue #29
