---
id: "PLAN-034KoNkBdCIchL4xE1DI7Y"
type: "plan"
status: "approved"
title: "Issue #90 実施計画 — 音楽時間モデル（media-core::music）"
created: "2026-10-22"
updated: "2026-10-23"
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
    - "docs/designs/music-time-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/tasks/music-time-media-core-module.md"
    - "docs/tasks/music-time-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/music-time-media-core-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #90 実施計画

## 手順

1. `crates/ruviform-media-core/src/music.rs` 新設 — 値型（`Ppqn`/`Tick`/`TickDuration`/`TempoEvent`/`TimeSignatureEvent`/`MusicalPosition`/`MusicalTimeInfo`）+ `TempoMapBuilder`/`TempoMap` + 変換 API（`tick_to_media`/`media_to_tick`/`musical_position`/`time_info`）+ lib.rs re-export
2. 単体テスト: test-spec 全ケース（builder 検証・変換往復 ≤1 tick・拍子分解・ABI スナップショット・拍子なし時の None/未設定）
3. RT テスト `rt_*`（assert_no_alloc 窓で変換 API の 0 alloc 実測）
4. 文書クローズ + 検証列

## 設計から実装へ具体化する判断（設計の未規定点）

- `tick_to_media`/`media_to_tick` は `Result` 返却（`Error::TimeOverflow`/`InvalidValue`）— 既存 `MediaTime::convert` 系の失敗契約と整合
- 先頭テンポセグメントは `-∞` 方向へ外挿（負 tick = pre-roll を許容する型設計と整合）
- `musical_position`/`bar_start_tick` は適用拍子なし時 `None`/0 フィールド（`sig_num`/`sig_den` = 0 が未設定表現 — CLAP の flag ドロップ写像と整合）
- `MusicalTimeInfo` は `Copy`（フィールド並びを i64→u32/u16→u8 に整列して 24 bytes の設計意図を満たす）

## Verification

- [x] 全変換 API が allocation なし・O(log n)（設計 RT 制約）— `rt_conversion_no_alloc` で `assert_no_alloc` 実測
- [x] `mise run` 検証列全パス・doc 監査チェーン起因 findings 0（残存5件は wave-1 由来の既存非ブロッキング警告）
