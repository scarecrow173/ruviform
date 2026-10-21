---
id: "TASK-034JOul8zCtHMR49d9c78V"
type: "task"
status: "done"
title: "write_interleaved の per-chunk timestamp 合成"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-device/src/convert.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-59-chunk-timestamp.md"
  implemented-by: []
  depends-on:
    - "add-sample-rate-to-callback-state.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-59-chunk-timestamp.md"
    - "add-sample-rate-to-callback-state.md"
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
    - "../designs/chunk-timestamp-design.md"
  derived-by:
    - "add-chunk-timestamp-tests.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/59"
  defers: []
  deferred-by: []
---

# write_interleaved の per-chunk timestamp 合成

## Work

`convert.rs::write_interleaved` の chunk ループで、各 chunk 先頭フレーム `frame` の `f_k / sample_rate` 秒オフセットを `OutputCallbackInfo` の `callback`/`playback` instant に合成する。

1. シグネチャに `sample_rate: u32` を追加（`info` の直後）
2. chunk ループ内で `frame > 0 && sample_rate > 0` のとき:
   - `offset_ns = u128::from(frame) * 1_000_000_000 / u128::from(sample_rate)` を `i64::try_from` で飽和
   - `MediaDuration::new(offset_ns, DEVICE_CLOCK_TIMEBASE)` を `info.timestamp.callback.checked_add(dur)` で加算 — `Err` 時は元 `callback` を保持
   - `playback` が `Some(p)` なら `p.checked_add(dur)` 同様。`Err` 時は元 `playback` を保持（`None` 化しない）
3. `frame == 0` または `sample_rate == 0` はオフセット 0 で既存 `info` をそのまま渡す
4. doc コメントに「合成値は chunk 先頭フレームのバッファ内位置に基づく推定 DAC/callback 時刻」を明記

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-59-chunk-timestamp.md](../plans/implement-issue-59-chunk-timestamp.md)
- [../designs/chunk-timestamp-design.md](../designs/chunk-timestamp-design.md)

## Dependencies

- 先行タスク: [add-sample-rate-to-callback-state.md](add-sample-rate-to-callback-state.md)
- ブロック対象: [add-chunk-timestamp-tests.md](add-chunk-timestamp-tests.md)

## Affected Files

- `crates/ruviform-audio-device/src/convert.rs`

## Implementation Notes

- 乗算は `u128` で行い `i64` 飽和を `try_from` で担保（`frame` が巨大でも panic しない）
- `MediaDuration::new` は `time.rs` の既存 API — `TimeBase::new(1, 1_000_000_000)` 等価で `Ok` が確定するが、安全側で `Result` を扱う（`unwrap_or` 元値フォールバック）
- RT 契約: 整数演算のみ・確保なし・`Copy` 値型

## Done Criteria

- [x] `write_interleaved` が `sample_rate` を受け取り per-chunk に instant を合成する
- [x] `checked_add` 失敗時に元 instant を渡す
- [x] `playback: None` でも panic しない
- [x] 単一 chunk 時に `info` が変化しない

## Verification

- [x] `mise run check`/`lint` パス
- [x] 追加テスト（T3）で振る舞いを検証

## Completion Notes

- `write_interleaved` に `sample_rate: u32` 引数を追加。chunk ループで `frame == 0 || sample_rate == 0` なら元 `info`、それ以外は `chunk_timestamp_info` ヘルパーで per-chunk 合成
- オフセット計算: `u128::try_from(frame_offset).unwrap_or(u128::MAX) * 1_000_000_000 / u128::from(sample_rate)` で整数除算（`u128::from(usize)` は非対応のため `try_from` + saturate）
- `i64` 変換失敗時・`MediaTime::checked_add` 失敗時は元の `OutputCallbackInfo`/instant を保持。`playback: None` は `None` のまま
- 整数演算のみ・無アロケート・有界計算で RT 制約を維持。合成値はバッファ位置由来の推定値である旨を doc コメントに明記
- `mise run check`/`lint` パス・T3 テストで振る舞い検証済み
