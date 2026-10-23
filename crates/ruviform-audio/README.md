# ruviform-audio

Ruviform メディア基盤の音声エンジン（Issue #25）: pull ベース処理グラフ、
planar-f32 ミキサー、デバイスセッション調停。
`ruviform-audio-core` のデバイスポート trait（`ruviform_audio_core::device`）と
`ruviform-rt` の `Processor`/`RtContext` 契約の上に RT 出力パスを重ねる。

## 責務

- `Graph` — 順序付き `AudioNode` リスト、ブロックごと pull 駆動
- `Mixer` — planar f32 加算ノード
- `DenormalFlush` — denormal を ±0 へ正規化するエンジン層ソフトウェア flush
  ノード（Issue #54）
- `GraphPlan` / `PlanSwap` / `RtPlan` — 不変スナップショット +
  `RwLock`/`try_read` スワップ
- `Session` — `OutputStream` を所有; デバイスコールバックがブロックごとに
  `RtContext` を mint してアクティブプランを駆動する

## 層・許可依存

- 層: Engines（設計ベースライン）
- 許可依存: Domain foundations（`ruviform-audio-core`）+ Primitives
  （`ruviform-rt`, `ruviform-media-core`）。`ruviform-audio-device` はアダプタ —
  エンジンはその型を名指さず、セッションはポート trait のみで会話する。

## RT 契約

デバイスコールバックパス（`Session` の内部コールバックとそれが駆動する全
`AudioNode::pull`）はバックエンド RT スレッド上で動く: アロケーション・
ブロッキング・OS 呼び出しは契約違反。全ての構築/変更 API は `non-RT only`。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 `ruviform-audio` 行）
- エンジン設計（Issue #25）: docs/designs/audio-engine.md
- スナップショットスワップ ADR: docs/adr/audio-engine-snapshot-swap.md
- denormal 対策（Issue #54）: docs/designs/denormal-flush-node.md, docs/adr/denormal-mitigation-policy.md
