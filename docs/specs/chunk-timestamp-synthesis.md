---
id: "SPEC-034JOoOaExdxXgKrQ80l7x"
type: "spec"
status: "approved"
title: "chunk 分割時の OutputCallbackInfo timestamp 合成"
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
  implemented-by:
    - "../plans/implement-issue-59-chunk-timestamp.md"
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
  depends-on:
    - "../designs/audio-device-boundary-and-poc.md"
    - "../adr/select-audio-device-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/audio-device-boundary-and-poc.md"
    - "../adr/select-audio-device-backend.md"
  derived-by:
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
    - "../designs/chunk-timestamp-design.md"
    - "../plans/implement-issue-59-chunk-timestamp.md"
  verifies: []
  verified-by:
    - "../test-specs/chunk-timestamp-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/pull/47"
    - "https://github.com/scarecrow173/ruviform/issues/25"
  defers: []
  deferred-by: []
---

# chunk 分割時の OutputCallbackInfo timestamp 合成

## What Should Be Built

`write_interleaved` が `frames_capacity` 超過時に分割する各 chunk へ、frame オフセットを反映した per-chunk `OutputCallbackInfo` を合成して渡す。各 chunk の `timestamp.callback`/`timestamp.playback` instant は「chunk の先頭フレームがバッファ内で占める時刻オフセット」分だけ進んだ値を持つ。

## Why Now

PR #47 レビュー指摘（Issue #59）。現行は全 chunk に同一 instant を渡す近似であり、後続 chunk の `playback` instant が実際より早く、callback 間隔・ジッター計測が潰れる。Issue #25（engine 統合）の `MasterClock` 配線前に契約を確定する必要がある。設計書 `audio-device-boundary-and-poc.md` の「PoC ではこの近似を明記し、厳密な per-chunk timestamp 合成は将来検討」の保留項を解消する。

## Users and Value

- engine 実装者（Issue #25）: `MasterClock` が per-chunk の正確な instant を受け取れる
- adapter 利用者: callback 間隔・ジッター計測が chunk 分割の有無に依らず一貫する

## Scope

### In Scope

- `DataCallbackState` に `sample_rate`（`u32`、Hz）を保持（open 時 `negotiated.stream.sample_rate` から取得・非 RT 経路のみ）
- `write_interleaved` の per-chunk `OutputCallbackInfo` 合成: `callback`/`playback` 各 instant に `chunk_start_frame * 1e9 / sample_rate` ナノ秒（`MediaTime` at `DEVICE_CLOCK_TIMEBASE` 整数 tick）のオフセットを `checked_add` で加算
- 合成不能時（`checked_add` 失敗）のフォールバック: その instant にはオフセットを適用せず元の値を渡す（`playback` の `Option` は既に「report 不能」を表現済み）
- per-chunk timestamp の単体テスト（オフセット量・フォールバック・単一 chunk 時の無変化）

### Out of Scope

- `MasterClock` への実配線（Issue #25）
- backend からの追加 timestamp 報告の活用（cpal が 1 instant/callback のみ提供する前提は維持）
- `DataCallbackState` の構造変更以外の open/config 経路変更

## Requirements

- chunk k（先頭フレーム `f_k`）の instant オフセットは `f_k / sample_rate` 秒を `DEVICE_CLOCK_TIMEBASE`（ナノ秒）整数 tick に変換した値 — 半端フレーム以下の誤差は tick 粒度で丸める（`MediaDuration` → `checked_add` 経路に集約）
- オフセットは `callback`・`playback` の両方に同量適用（chunk 先頭フレームの「打鍵時刻」と「DAC 到達時刻」は同量だけ遅れる）
- 合成不能（`checked_add` が `Err`）時はその instant を未合成のまま渡す — timestamp が失われるより元値が残る方が悪くない
- 単一 chunk（`n <= max_chunk`）ではオフセット 0 — 既存テストの挙動を変えない
- RT 契約維持: 合成は整数演算のみ・確保なし・`Copy` 値型のまま
- `sample_rate` 保持は `DataCallbackState` のフィールド追加（`u32` — `negotiated.stream.sample_rate` は cpal `SampleRate(u32)`）

## Acceptance Criteria

- [ ] chunk 分割時（`n > max_chunk`）、k 番目 chunk の `callback`/`playback` instant が元 instant + `f_k * 1e9 / sample_rate` ナノ秒に一致する（単体テスト）
- [ ] `playback: None` の場合も `callback` のみオブセット合成され、panic しない
- [ ] `checked_add` 失敗時に元 instant がそのまま渡る（フォールバックテスト）
- [ ] 単一 chunk 時（`n <= max_chunk`）は instant が変化しない（既存テストで担保）
- [ ] `mise run test`・`mise run test-rt` 全パス・新テスト追加

## Deferred Design Concerns

- backend 側が chunk 分割を起こす頻度・実害は実機依存。cpal が per-callback 単一 timestamp のみ提供する限り、合成値は「バッファ内位置に基づく推定」であり真の DAC 時刻ではない — 精度限界は契約 doc に明記 | engine 統合時に実機で観測値を検証 | Issue #25 | 合成値の精度が engine の前提を満たさない可能性

## Sources and Relations

- Issue [#59](https://github.com/scarecrow173/ruviform/issues/59) — PR #47 レビュー指摘の追跡
- [../designs/audio-device-boundary-and-poc.md](../designs/audio-device-boundary-and-poc.md)「コールバック配管」— 近似の明記と「厳密な per-chunk timestamp 合成は将来検討」の保留項
- [../adr/select-audio-device-backend.md](../adr/select-audio-device-backend.md) — cpal 採否・timestamp 提供範囲の前提
