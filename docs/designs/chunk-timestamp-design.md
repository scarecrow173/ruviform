---
id: "DESIGN-034JOoWdEHXnb8G1QyMzCm"
type: "design"
status: "approved"
title: "chunk 分割時の per-chunk timestamp 合成設計"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/59"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-device/src/convert.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/device.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/chunk-timestamp-synthesis.md"
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/chunk-timestamp-synthesis.md"
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
    - "audio-device-boundary-and-poc.md"
  derived-by:
    - "../plans/implement-issue-59-chunk-timestamp.md"
    - "../tasks/synthesize-per-chunk-timestamps.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/25"
  defers: []
  deferred-by: []
---

# chunk 分割時の per-chunk timestamp 合成設計

## Context

`write_interleaved` は `n > max_chunk` のとき backend の interleaved `data` を `max_chunk` フレームずつに分割し、各 chunk で `data_callback` を呼ぶ。各 chunk 先頭フレーム `f_k` は callback バッファ先頭から `f_k / sample_rate` 秒遅れて DAC に到達する — このオフセットを `OutputCallbackInfo` の両 instant に合成して渡す。`sample_rate` は open 時に確定する `negotiated.stream.sample_rate`（cpal `SampleRate(u32)`）を `DataCallbackState` に保持する。

## Scope

### In Scope

- `DataCallbackState` に `sample_rate: u32` フィールド追加・`new` の引数追加
- `device.rs` の `DataCallbackState::new` 呼び出しで `negotiated.stream.sample_rate.0`（または同等の `u32` 値）を渡す
- `write_interleaved` に `sample_rate: u32` 引数追加・chunk ループで `OutputCallbackInfo` を per-chunk 合成
- 合成不能時フォールバック（元 instant をそのまま渡す）と単体テスト

### Out of Scope

- `MasterClock` 配線（Issue #25）
- backend 報告 timestamp の精度向上
- `f32` 以外のサンプル形式対応

## Components and Boundaries

- `stream.rs::DataCallbackState` — `sample_rate` を保持し `handle` で `write_interleaved` に渡す。instant 合成の責務は持たない（`handle` は epoch 確立と `map_output_timestamp` まで）
- `convert.rs::write_interleaved` — chunk ループ内で各 chunk 先頭フレーム `frame` からオフセットを計算し、`info` を per-chunk に合成して `data_callback` に渡す。合成ロジックはこの関数内に閉じる（`DataCallbackState` や callback 側に漏らさない）
- `device.rs::open_output_stream` — `DataCallbackState::new` 呼び出しで `negotiated.stream.sample_rate` の `u32` 値を渡す（open 時・非 RT）

## Data and Control Flow

1. open（非 RT）: `negotiated.stream.sample_rate`（`cpal::SampleRate` → `.0` で `u32`）を `DataCallbackState` に保持
2. callback（RT）: `handle` が `map_output_timestamp` で base `OutputCallbackInfo` を構成し `write_interleaved` に `sample_rate` 込みで渡す
3. `write_interleaved` 内 chunk ループ: 各反復で `frame`（chunk 先頭の絶対フレーム index）を `sample_rate` でナノ秒オフセットに変換
   - `offset_ns = frame * 1_000_000_000 / sample_rate`（`u64`/`u128` 演算でオーバーフロー回避 — `frame` は `usize` で最大 ~16_384 * 分割数、ns 換算で十分 64bit 収まる。`i64` 飽和は `checked_add` 側が担う）
   - `MediaDuration::new(offset_ns as i64, DEVICE_CLOCK_TIMEBASE)` → `checked_add` で `callback`/`playback` 各 instant に加算
   - `checked_add` 失敗時はその instant は元値のまま（`playback` が `None` なら `callback` のみ合成）
4. chunk 0（`frame == 0`）はオフセット 0 — `checked_add` 自体をスキップして既存動作と一致させる

## Non-Functional Constraints

- RT 契約: 整数演算のみ・確保なし・panic なし（`checked_add` 経路は `Result` で扱う）
- `MediaDuration`/`MediaTime`/`TimeBase` の既存 API を使い、自前の時刻演算を増やさない
- `sample_rate == 0` は起こり得ない（cpal が正の値を保証・negotiate 済み）が、防御的に `0` なら除算をスキップしてオフセット 0 にする（RT 経路の panic 回避優先）

## Risks and Trade-offs

- **リスク**: `frame * 1_000_000_000` の `u64` 乗算は `frame` が ~10^10 フレーム（数時間）を超えるとオーバーフローしうる → 対策: `u64` → `u128` で乗算し、結果を `i64::try_from` で飽和。飽和時は `checked_add` が `Err` → フォールバックで元値
- **トレードオフ**: ナノ秒 tick 粒度で丸めるため、1 サンプル ≈ 20.8ns（48kHz）以下の誤差が残る — `DEVICE_CLOCK_TIMEBASE` の粒度として許容範囲（契約 doc に明記）
- **近似の限界**: 合成値は「この chunk の先頭フレームが到達する予想 DAC 時刻」であり、backend の真の per-chunk timestamp ではない — 設計・契約 doc に明記

## Deferred Design Concerns

- engine 統合時に per-chunk 合成値の実機精度を `play_sine` example の観測値で検証 | Issue #25 | 合成値が backend 実測とずれる場合のフィードバック経路 | engine 側がずれを知らずに同期判断する可能性

## Verification Notes

- [ ] chunk 分割時、k 番目 chunk の `callback`/`playback` が base + `f_k * 1e9 / sample_rate` ns に一致する単体テスト
- [ ] `playback: None` でも `callback` のみ合成され panic しない
- [ ] `checked_add` 失敗時に元 instant を渡すフォールバックテスト
- [ ] 単一 chunk（`n <= max_chunk`）は instant 無変化
- [ ] `mise run test`/`test-rt` 全パス
