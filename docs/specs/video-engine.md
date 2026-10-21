---
id: "SPEC-034JZVID00000000000010"
type: "spec"
status: "approved"
title: "動画エンジン（フレームパイプライン・A/V 同期・シーク）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/adr/video-av-sync-policy.md"
    - "docs/designs/video-engine.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 動画エンジン（フレームパイプライン・A/V 同期・シーク）

## 背景

設計ベースライン `ruviform-video`: フレームパイプライン・A/V 同期（`MasterClock` 追従）・シーク・オフラインレンダ調停。codec/UI は持たない。内部処理は原則非 RT、A/V 同期面のみ `MasterClock` 読取で RT 接続。

## 要件

1. `ruviform-video` クレート新設（deps: `ruviform-rt`・`ruviform-media-core`・`ruviform-video-core`・`ruviform-media-io`）
2. フレームパイプライン: pull ベース・ステージ列（`VideoStage` trait）を decoded フレームへ適用
3. A/V 同期: audio master — `MasterClock` 時刻とフレーム PTS を比較し Present/Wait/Drop を判定（ffplay 型 PTS 追従）
4. シーク: `Demuxer::seek` + `Decoder::flush` + PTS 基準リセットの調停
5. `MasterClock` 参照は読み取り専用・ノンブロッキング。エンジン内部処理は非 RT
6. `ruviform-audio` への依存禁止（A/V 同期は共有時刻型経由）

## Acceptance Criteria

- [ ] `ruviform-video` が workspace に存在しパイプライン/スケジューラ/セッション API を公開
- [ ] `FrameScheduler` が PTS vs MasterClock で Present/Wait/Drop を返す（単体テスト）
- [ ] パイプラインがステージを順序どおり適用する（モックデコーダ駆動テスト）
- [ ] seek が demux.seek + decoder.flush + スケジューラリセットを実行（モックテスト）
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス

## 関連文書

- 方策 ADR: [video-av-sync-policy.md](../adr/video-av-sync-policy.md)
