---
id: "TSPEC-034JOtlBoKc1LUny4Iu67z"
type: "test-spec"
status: "approved"
title: "chunk timestamp 合成の契約テスト"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/59"
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
  derives-from: []
  derived-by: []
  verifies:
    - "../specs/chunk-timestamp-synthesis.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/25"
  defers: []
  deferred-by: []
---

# chunk timestamp 合成の契約テスト

## Purpose

`write_interleaved` の chunk 分割時に、各 chunk が `f_k / sample_rate` ナノ秒のオフセットを持つ `OutputCallbackInfo` を受け取ることを検証する。Issue #25 `MasterClock` 配線の前提契約を pin する。

## Feature

`OutputCallbackInfo` per-chunk timestamp 合成（`ruviform-audio-device`）

## Rules

- Rule: chunk 先頭フレーム `f_k` に対し `callback`/`playback` instant が `f_k * 1e9 / sample_rate` ナノ秒だけ base instant から進む
- Rule: `playback: None` でも panic せず `callback` のみ合成される
- Rule: `checked_add` 失敗時はその instant に元値を渡す（timestamp を失わない）
- Rule: 単一 chunk（`n <= max_chunk`）では instant が変化しない（オフセット 0）
- Rule: 合成は整数演算のみ・確保なし・RT 契約を破らない

## Examples

- Given `frames_capacity = 4`・`sample_rate = 48000`・10 フレームの入力、When `write_interleaved` を実行、Then chunk 0/1/2 の `callback` instant は base + 0 / `4*1e9/48000` / `8*1e9/48000` ns である（≈ 83_333ns / 166_667ns — 1ns 未満の端数は tick 粒度で切捨て）
- Given `playback: None` の `OutputCallbackInfo`、When chunk 分割が発生、Then `callback` のみオフセットが合成され `playback` は `None` のまま
- Given base instant が `i64::MAX` 近傍で `checked_add` が `Err`、When chunk 分割が発生、Then その chunk の instant は元値のまま
- Given `n <= max_chunk`、When `write_interleaved` を実行、Then `info` は変更されない（`callback`/`playback` が base と一致）

## Guarantees

- `DataCallbackState` の `sample_rate` は `negotiated.stream.sample_rate`（`u32`）で open 時に確定・callback 内で変わらない
- `MediaDuration`/`MediaTime::checked_add`/`DEVICE_CLOCK_TIMEBASE` の既存 API のみで合成（自前時刻演算を増やさない）
- `OutputCallbackInfo`/`OutputTimestamp`/`MediaTime` は `Copy` 値型のまま

## Non-goals

- backend 真の DAC 時刻との一致（合成値はバッファ内位置に基づく推定 — 契約 doc に明記）
- `MasterClock` 実配線（Issue #25）
- cpal 以外 backend の timestamp 契約
