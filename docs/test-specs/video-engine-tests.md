---
id: "TSPEC-034JZVID00000000000050"
type: "test-spec"
status: "approved"
title: "動画エンジン テスト仕様"
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
  derived-by: []
  verifies:
    - "docs/designs/video-engine.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 動画エンジン テスト仕様

## ケース

- `scheduler_decision_table`: lateness 境界で Present/Wait/Drop を返す
- `pipeline_applies_stages_in_order`: ステージ適用順序を検証
- `session_drives_decode_loop`: モック demux/decoder で `next_frame` が pipeline 済みフレームを返す
- `session_seek_resyncs`: `seek` が `Demuxer::seek`→`Decoder::flush`→`scheduler.reset` を順序実行
- `audio_frame_is_skipped`: `Frame::Audio` は video セッションで読み飛ばす
