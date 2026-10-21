---
id: "PLAN-034JZVID00000000000040"
type: "plan"
status: "approved"
title: "Issue #27 実装計画 — 動画エンジン"
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
    - "docs/designs/video-engine.md"
  derived-by:
    - "docs/tasks/video-engine-crate.md"
    - "docs/tasks/video-engine-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/video-engine-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #27 実装計画

## 手順

1. `crates/ruviform-video` 新設（workspace 登録・deps）
2. `pipeline.rs`: `VideoStage`/`FramePipeline`
3. `sync.rs`: `SyncDecision`/`SyncThresholds`/`FrameScheduler`
4. `session.rs`: `VideoSession`（demux+decode+pipeline 調停・seek）
5. テスト（モック demuxer/decoder・判定表・パイプライン順序・seek 調停）
6. 文書クローズ + 検証列

## Verification

- [ ] 判定表テスト（Present/Wait/Drop 境界）グリーン
- [ ] seek が demux.seek→decoder.flush→scheduler.reset を順序実行
- [ ] mise 検証列パス・doc 監査 findings 0
