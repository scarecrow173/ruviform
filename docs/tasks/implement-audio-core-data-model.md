---
id: "TASK-034J5KQD1rDMZHFRah7pU0"
type: "task"
status: "done"
title: "ruviform-audio-core のデータモデル実装"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/18"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-core/src/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/format.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/layout.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/sample.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/time.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-18-data-model.md"
  implemented-by:
    - "../impl/ir/implement-audio-core-data-model.md"
  depends-on:
    - "../plans/implement-issue-18-data-model.md"
    - "implement-media-core-data-model.md"
  blocks:
    - "sync-docs-and-verify-issue-18.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-18-data-model.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/media-data-model-api.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-audio-core のデータモデル実装

## Work

plan `PLAN-034J5K603qXDNmT82wI9v7` の Phase B。`tmp/issue-18/api-brief.md`「Crate 2 — ruviform-audio-core」+ Global constraints を正本として、planar f32 正準の音声データモデル API を実装する（実績としての遡及記録）。

- [x] `Cargo.toml` に `ruviform-media-core = { path = "../ruviform-media-core" }` を追加（`Cargo.lock` 更新）
- [x] `sample` モジュール: sealed `Sample` trait（`f32` / `f64` のみ実装・`const BYTES`、f32 が RT 正準）
- [x] `time` モジュール: `SampleRate`（`TimeBase(1/rate)` 保持・`time_base()`）・`SampleIndex`（signed、`to_media_time` / `from_media_time`、音声↔動画変換は音声サンプル時刻優先を明記）
- [x] `layout` モジュール: `SpeakerMask`（8 定数 + `from_bits`/`contains`/`union` 等）・`ChannelLayout`（Mono〜7.1 + `Custom`）
- [x] `format` モジュール: `#[non_exhaustive] SampleFormat`（`#[default] F32`）・`AudioFormat`
- [x] `buffer` モジュール: `BufferSpec`・チャンネルメジャー planar `AudioBuffer<S = f32>`・`AudioBufferView` / `AudioBufferViewMut`・`from_interleaved` / `copy_to_interleaved` / `copy_from_interleaved` 境界変換（確保系は non-RT、成長 API なし）
- [x] 全公開 API に `**RT contract:**` ラベル、`lib.rs` crate doc 更新 + 再エクスポート + `version()` 維持
- [x] `rt_` テスト 3 本（`rt_view_channel_access`・`rt_interleaved_roundtrip_via_views`・`rt_sample_index_media_time_conversion`）と doctest/単体テスト整備

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-18-data-model.md](../plans/implement-issue-18-data-model.md) — `PLAN-034J5K603qXDNmT82wI9v7`（completed）。Phase B の正本
- [../specs/implement-media-data-model-api.md](../specs/implement-media-data-model-api.md) — `SPEC-034J5FYaYfeL5zkzb4sIV4`（implemented）。音声データモデル要件の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。「音声データモデル」節の根拠

## Dependencies

- 前提タスク: [implement-media-core-data-model.md](implement-media-core-data-model.md)（`TASK-034J5KJgwtjFonTMQQIvpX`）— path 依存先の `MediaTime` / `TimeBase` / `Rounding` / `Error` / `Result` を提供
- 後続をブロック: [sync-docs-and-verify-issue-18.md](sync-docs-and-verify-issue-18.md)（総合検証対象を提供）
- `ruviform-video-core` への依存は作らない（層内順序ルール）。外部クレート依存ゼロ・`unsafe` 不使用

## Affected Files

- `crates/ruviform-audio-core/src/sample.rs` — 新規
- `crates/ruviform-audio-core/src/time.rs` — 新規
- `crates/ruviform-audio-core/src/layout.rs` — 新規
- `crates/ruviform-audio-core/src/format.rs` — 新規
- `crates/ruviform-audio-core/src/buffer.rs` — 新規
- `crates/ruviform-audio-core/src/lib.rs` — crate doc・再エクスポート・`rt_` テスト更新
- `crates/ruviform-audio-core/Cargo.toml` — `ruviform-media-core` path 依存追加
- `Cargo.lock` — path 依存の記録

## Done When

- [x] brief の全型・署名が公開 API として存在し、crate root から再エクスポートされる
- [x] planar ↔ interleaved が往復一致し、長さ不一致が `Err(InvalidValue)` で表出する
- [x] 確保系コンストラクタが `non-RT only`、アクセサ・ビュー・コピーが `RT-suitable` とラベルされる
- [x] 実施記録 [../impl/ir/implement-audio-core-data-model.md](../impl/ir/implement-audio-core-data-model.md) に検証結果が残っている

## Verification

- [x] `mise exec -- cargo test -p ruviform-audio-core` — 30 単体テスト + 18 doctest 合格（修正 wave 後の最終値。初回実施時は 29 + 18）
- [x] `mise exec -- cargo clippy -p ruviform-audio-core --all-targets -- -D warnings` — 0 warnings
- [x] `mise exec -- cargo test -p ruviform-audio-core rt_` — `rt_` プレフィックス 3 テストが選択・合格
- [x] `mise run fmt-check` — 差分なし
