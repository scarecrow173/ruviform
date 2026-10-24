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

## 関連

- vendored pinned headers: `vendor/openfx/`（`manifest.json` に sha256 記録）
- 実装 spec/design/plan: `docs/specs/ofx-host-adapter.md`,
  `docs/designs/ofx-host-adapter.md`,
  `docs/plans/implement-issue-98-ofx-host-adapter.md`
