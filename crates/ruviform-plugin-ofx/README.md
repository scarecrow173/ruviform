# ruviform-plugin-ofx

OpenFX Image Effect ホストアダプタ — 中立 `VideoPluginInstance` 契約の裏側で、
実 foreign ABI 経路（pinned upstream OpenFX headers + 実 DLL/SO/dylib ロード）
により L0 same-process 同期ホスティングを行う（Issue #98）。

## 層・許可依存

- 層: 拡張層（adapter — engine と同じ利用側層）
- 許可依存: `ruviform-plugin-core`（中立契約）・`ruviform-video-core`・
  `ruviform-media-core`・`libloading`（ネイティブロード）のみ。
  `cc` は build/dev 依存のみ（C bridge・fixture コンパイル）。SDK/外部クレートの
  型・エラー・feature 名を公開 API に露出しない。

## RT 契約

全操作 NONRT（`render` を含む）— 制御プレーン同期呼び出し。
audio `process` 系の RT 保証とは別経路。

## 現状

`OfxHost::load_library` が実 OFX DLL を admission（`Load`/`Describe`/
`DescribeInContext`）し、C bridge 経由で `CreateInstance`〜`Destroy`・
パラメータ・clip preferences まで到達する。`render` は映像 task まで
`OfxError::Unsupported`。

## L1（out-of-process）境界 — Issue #114 AC5

`VideoPluginInstance` は全操作 NONRT（`render` を含む制御プレーン同期呼出し）
で、video wire は未確立（L1 の wire/shm データ面は planar `f32` audio 専用 —
Issue #113 design 確定面）。そのため OFX プラグインは L1 子プロセス経路
（`ruviform_plugin_export::run(factories)` + audio shm block）に**構造上
載らない** — `run` の argv 規約・`L1Instance` の audio block 契約は video
port を表現できず、audio 面の `#[ignore]` E2E lane
（`mise run test-l1-real-plugin`）も OFX を対象に取らない。

OFX の L1 経路・video wire 確立は Issue #118 で追跡する。延期 trigger は
NONRT video port の分離要求が確定した時点（L1 design の deferred 項目と
整合）。

## 関連

- vendored pinned headers: `vendor/openfx/`（`manifest.json` に sha256 記録）
- 実装 spec/design/plan: `docs/specs/ofx-host-adapter.md`,
  `docs/designs/ofx-host-adapter.md`,
  `docs/plans/implement-issue-98-ofx-host-adapter.md`
