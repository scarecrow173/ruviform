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
  （アダプタ内部の実装依存。`clack-host` はテスト用 dev-dependency に加え、
  feature `host` では optional prod dependency — 後述）。

## RT 契約

`process` は RT コールバック — alloc/dealloc・ブロッキング・I/O・無界の
待機/再試行・無界の処理を行わない。インスタンスごとのスクラッチは全て
`activate` で確保し、ブロックごとのパスは有界スキャンのみ行う。
`rt_` プレフィックスのテストが RT 契約レーン（`mise run test-rt`）を構成;
Issue #22 RT 検証レーンはブロックごとパスを `assert_no_alloc` で囲み、
確保/解放を機械検出する。CI 通過は RT 安全性の*証明*ではない — 静的制約
（ロック時間・優先度・実行時間上界）は `docs/adr/realtime-contract-and-verification-policy.md`
に従い文書化 + レビューの責務として残る。

## feature `host` — 規格 host-side bridge + L1 runner（Issue #114）

default feature 集合に含まない `host` を有効化すると、host 側 bridge
`ruviform_plugin_clap::host` が公開される。`ClapLibrary::load(path)` が実
`.clap` 共有オブジェクトを `clack-host` 経由で dlopen し、`descriptors()` /
`factories()` がスキャン結果を中立 `PluginDescriptor`/`PluginFactory`/
`ErasedPlugin` 契約へ写像する（`PluginDescriptor`/`ParamInfo` の `'static`
境界は load/factory 構築時の 1 回限り bounded `String::leak`/`Vec::leak` で
実体化）。`save_state`/`restore_state`/`open_gui`/`close_gui` は `Plugin`
既定の `Err(HostError::Unsupported)` のまま — bridge は規格 state/GUI 動詞を
呼ばない（fixture の state 面は未実装・偽の round-trip を観測しない）。

runner bin `ruviform-plugin-clap-l1-runner`（`[[bin]] required-features =
["host"]`）は compile-time 固定 — fixture cdylib
`{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}` を `RUVIFORM_L1_REAL_PLUGIN_DIR`
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
- 実装 spec/plan（task T3）: docs/specs/implement-plugin-foundation.md, docs/plans/implement-plugin-foundation.md
- binding 方式 ADR: docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md
- PoC 所見: docs/impl/ir/poc-clap-binding-adapter.md
- Issue #29
