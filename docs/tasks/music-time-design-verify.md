---
id: "TASK-gJ0wsCnJrG8Wo0JQSENFtz"
type: "task"
status: "done"
title: "音楽時間 ABI 整合検証 + Issue #33 クローズ"
created: "2026-11-03"
updated: "2026-11-03"
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
  implements:
    - "docs/plans/implement-issue-33-music-time.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-33-music-time.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ABI 整合検証 + クローズ

## Done When

- [x] `TransportInfo`/`TransportSnapshot` の `reserved: [u64; 2]` 拡張予約と `Option<MusicalTimeInfo>` フィールド追加方式が整合し、ABI を破壊しないことを確認（`crates/ruviform-plugin-core/src/transport.rs`・`crates/ruviform-media-core/src/transport.rs`）
- [x] CLAP/VST3 写像規則・丸め損失許容範囲が設計に規定
- [x] 検証列・doc 監査・graph complete
