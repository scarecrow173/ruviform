---
id: "TASK-034JOul8zCtHMR49d9c78V"
type: "task"
status: "todo"
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

- [ ] `write_interleaved` が `sample_rate` を受け取り per-chunk に instant を合成する
- [ ] `checked_add` 失敗時に元 instant を渡す
- [ ] `playback: None` でも panic しない
- [ ] 単一 chunk 時に `info` が変化しない

## Verification

- [ ] `mise run check`/`lint` パス
- [ ] 追加テスト（T3）で振る舞いを検証

## Completion Notes

<!-- Record outcome, deviations, and evidence after implementation. -->
