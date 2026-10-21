---
id: "DESIGN-034JZVID00000000000030"
type: "design"
status: "approved"
title: "動画エンジン設計（パイプライン・スケジューラ・セッション）"
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
    - "docs/specs/video-engine.md"
    - "docs/adr/video-av-sync-policy.md"
  derived-by:
    - "docs/plans/implement-issue-27-video-engine.md"
    - "docs/test-specs/video-engine-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/video-engine-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# 動画エンジン設計

## Overview

3 要素: `FramePipeline`（pull ベース処理列）・`FrameScheduler`（A/V 同期判定）・`VideoSession`（demux→decode→pipeline 調停 + シーク）。全て非 RT。`MasterClock` は読み取り専用で同期面のみ使用。

## 構成

- `VideoStage` trait: `fn process(&mut self, frame: VideoFrame) -> Result<VideoFrame>` — 変換/合成ステージ
- `FramePipeline`: `Vec<Box<dyn VideoStage>>` を順序適用。`fn run(&mut self, frame) -> Result<VideoFrame>`
- `FrameScheduler { thresholds }`: `fn decide(&self, pts: MediaTime, master: MediaTime) -> SyncDecision`（純粋関数）。`SyncDecision::{Present, Wait(MediaTime), Drop}`。`reset()` でシーク後再同期
- `VideoSession<D: Demuxer, C: Decoder>`: `next_frame()` = `demuxer.read_packet` → `decoder.send_packet`/`receive_frame` → Video フレームを pipeline へ。`seek(pos)` = `demuxer.seek(pos)` + `decoder.flush()` + `scheduler.reset()`
- エラーは `ruviform_media_io::Error` 経由で伝播（engine 固有エラー型は持たない）

## RT 契約

全公開 API は非 RT。`MasterClock::tick` の読取のみノンブロッキング値読み（RT 接続面）。

## Verification

- [x] 層ルール適合: engines → foundations+primitives のみ
- [x] `ruviform-audio` 非依存
- [x] 全 API 非 RT 宣言・`unsafe` 0
