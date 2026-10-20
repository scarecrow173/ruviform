---
id: "IMPL-034J5L4kgMP7Hjh57OhJcs"
type: "impl"
status: "completed"
title: "Implement audio-core data model"
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
    - "../../tasks/implement-audio-core-data-model.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-issue-18-data-model.md"
    - "../../designs/ruviform-architecture-design-baseline.md"
    - "small-fixes-from-followup-triage.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/media-data-model-api.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement audio-core data model

## Summary

task `TASK-034J5KQD1rDMZHFRah7pU0`（plan `PLAN-034J5K603qXDNmT82wI9v7` Phase B）として、`ruviform-audio-core` に planar f32 正準の音声データモデル API を実装し、`ruviform-media-core` への path 依存を配線した。sealed `Sample`・`SampleRate`/`SampleIndex` 音声時間契約・`SpeakerMask`/`ChannelLayout`・`AudioFormat`・チャンネルメジャー `AudioBuffer` + 借用ビュー + 境界 interleaved 変換を含む（PR #39・merge `d041ba9`）。

## Context

- 実施タスク: [../../tasks/implement-audio-core-data-model.md](../../tasks/implement-audio-core-data-model.md)
- 上流: [plan](../../plans/implement-issue-18-data-model.md)、[spec](../../specs/implement-media-data-model-api.md)、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md)「音声データモデル」節、[RT ADR](../../adr/realtime-contract-and-verification-policy.md)
- 正本要件: `tmp/issue-18/api-brief.md`「Crate 2 — ruviform-audio-core」+ Global constraints + dependency wiring
- 前提: media-core 実装済み（task `TASK-034J5KJgwtjFonTMQQIvpX` の `MediaTime`/`TimeBase`/`Rounding`/`Error`/`Result` を使用）。外部依存ゼロ・`unsafe` 不使用・f64 は型パラメータのみ

## Implementation

### Changes

- `crates/ruviform-audio-core/Cargo.toml` — `ruviform-media-core = { path = "../ruviform-media-core" }` 追加（`Cargo.lock` 更新）
- `src/sample.rs` — sealed `Sample`（private `mod sealed`、`f32`/`f64` のみ、`const BYTES`、f32 を RT 正準として文書化）
- `src/time.rs` — `SampleRate`・`SampleIndex(pub i64)`（`to_media_time` 厳密・`from_media_time(t, rate, Rounding) -> Result` は `MediaTime::convert` へ委譲）
- `src/layout.rs` — `SpeakerMask(u32)`（WAVEFORMATEX 順 8 定数 + `empty`/`from_bits`/`bits`/`is_empty`/`contains`/`union`、全 `const fn`）・`ChannelLayout`（Mono/Stereo/Quad/5.1/7.1/`Custom(SpeakerMask)`、`custom()` は空マスクで `Err`）
- `src/format.rs` — `#[non_exhaustive] SampleFormat { F32, F64 }`（`#[default] F32`）・`AudioFormat`
- `src/buffer.rs` — `BufferSpec`（layout がチャンネル数を保持）・`AudioBuffer<S = f32>`（`data: Vec<S>` チャンネルメジャー連続、`new`/`from_interleaved` は non-RT 確保系、`channel`/`channel_mut`/`view`/`view_mut`/`data`/`data_mut`/`copy_to_interleaved`/`copy_from_interleaved` は RT-suitable。成長 API なし）・`AudioBufferView`（`Copy`）・`AudioBufferViewMut`
- `src/lib.rs` — crate doc（planar 正準・境界 interleaved・デバイス境界は別 Issue）+ `pub use` 再エクスポート + `version()` 維持 + `rt_` テスト 3 本

### Decisions During Implementation

- **`SampleRate` は `TimeBase(1/rate)` を内部保持**（brief の `u32` スケッチからの変更。`rate > 0` を構築時に構造的に保証し `time_base()` を panic パスなしの `const` 射影にした。公開 API への影響は `PartialOrd`/`Ord` 追加のみ。レポートで可視化済み）
- **`SpeakerMask::is_empty()` と `data()`/`data_mut()` スラブアクセサを追加**（bitflags 流の自然な完成 + 境界/SIMD 向け連続 planar アクセス。いずれも RT-suitable borrow）
- **ビューにも `copy_to_interleaved`/`copy_from_interleaved` を対称配置**（brief は ViewMut の `copy_to` のみ要求。buffer/view 対称性のため追加）
- **共有 private helper**（`channel_range`/`pack_interleaved`/`unpack_interleaved`）に slicing/de-interleave を一元化
- **`AudioBuffer::new` の panic 契約**: `channels * frames` の `usize` overflow と `isize::MAX` バイト超過で panic（`-> Self` の形状を維持し、形状エラーは `Result` コンストラクタ側へ — coordinator ruling）。C1 指摘で `# Panics` に両パスを明記し `isize::MAX` 境界を `vec!` 前にガード
- **縮退 spec の扱い**: `Custom` 空マスクや `frames == 0` は 0 チャンネル/0 長バッファとして文書化（panic しない）
- **`Error`/`Result` は media-core から再利用**（`InvalidValue` payload: `"sample_rate"|"mask"|"src"|"dst"|"frames"`、`TimeOverflow` 伝播）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `mise run fmt` / `fmt-check` | 差分なし |
| 型チェック | `mise exec -- cargo check -p ruviform-audio-core --all-targets` | clean |
| lint | `mise exec -- cargo clippy -p ruviform-audio-core --all-targets -- -D warnings` | clean・workspace pedantic lints 下で 0 warnings |
| テスト | `mise exec -- cargo test -p ruviform-audio-core` | 初回 29 単体 + 18 doctest = 47 合格。修正 wave 後の最終値は 30 単体 + 18 doctest |
| RT レーン | `mise exec -- cargo test -p ruviform-audio-core rt_` | `rt_` 3 テスト（`rt_view_channel_access`・`rt_interleaved_roundtrip_via_views`・`rt_sample_index_media_time_conversion`）のみ選択・合格 |

レビューサイクル:

- Reviewer A: A1（`AudioBuffer::sample_rate()`/`layout()`/`data_mut()`・View/ViewMut 複数アクセサのテストカバレッジ欠落・P2）。Reviewer B: C1（`AudioBuffer::new` の容量 panic 境界・P2）・C3（`version()` RT ラベル・P3）。Validator は全件 CONFIRMED。Judge は A1・C3 を blocking、C1 を非 blocking（契約違反は実在・悪用性なし）判定
- 修正: 新テスト `buffer_and_views_expose_spec_and_slab` で A1 全アクセサをカバー、`AudioBuffer::new` に `isize::MAX` バイトガード + `# Panics` 両パス文書化、`version()` に `**RT contract:**` 追加
- ワークスペース最終検証（タスク 4 で実施）: `mise run check` / `lint` / `test` / `test-rt` 全 PASS

## Risks

- `ChannelLayout::Custom` の非ゼロマスク不変条件は文書化のみ（pub enum variant のため構造的強制はできず、`channels()` は 0 に縮退）。アダプタ層での検証が前提
- `SpeakerMask::from_bits` は予約ビットを保持する（masked しない）。`channels()` のカウントにも含まれる文書化済み動作
- ビュー型に公開コンストラクタがない（`AudioBuffer::view`/`view_mut` 経由のみ）。外部供給の planar スライスを包む需要が出た場合は `AudioBufferView::new`（`len == channels * frames` 検証付き）が自然な拡張点（YAGNI で未実装）

## Follow-ups

- デバイス境界 trait（アダプタポート契約）は別 Issue の範囲。`copy_to/from_interleaved` がその境界書き込み API として機能する想定
- `Sample` の f64 対応は型パラメータのみで留めている（RT 正準は f32 の決定。深化は需要ベースの後続評価）
