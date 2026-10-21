---
id: "ADR-034JOpkx5tdDPVGzdlb0bb"
type: "adr"
status: "accepted"
title: "chunk 分割時の OutputCallbackInfo timestamp 合成"
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
  implements:
    - "../specs/chunk-timestamp-synthesis.md"
  implemented-by: []
  depends-on:
    - "../designs/audio-device-boundary-and-poc.md"
    - "select-audio-device-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/chunk-timestamp-synthesis.md"
    - "../designs/audio-device-boundary-and-poc.md"
    - "select-audio-device-backend.md"
  derived-by:
    - "../designs/chunk-timestamp-design.md"
    - "../plans/implement-issue-59-chunk-timestamp.md"
    - "../tasks/synthesize-per-chunk-timestamps.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/pull/47"
    - "https://github.com/scarecrow173/ruviform/issues/25"
  defers: []
  deferred-by: []
---

# chunk 分割時の OutputCallbackInfo timestamp 合成

## Context and Problem Statement

`ruviform-audio-device` の `write_interleaved` は、backend コールバックが `frames_capacity` を超えるフレーム数を届けた場合、bounded chunk に分割して `data_callback` を複数回呼ぶ。現行実装は全 chunk に同一 `OutputCallbackInfo`（同一 `callback`/`playback` instant）を渡す近似を採用しており、設計書も「厳密な per-chunk timestamp 合成は将来検討」と記録済み（PR #47 レビュー指摘 → Issue #59）。Issue #25 の `MasterClock` 配線前に契約を確定する必要がある。

## Decision Drivers

* `playback` instant が後続 chunk で実際より早いと、`MasterClock` のレイテンシ推定・同期判断が歪む
* `callback` instant が chunk 間で同一だと、コールバック間隔・ジッター計測が潰れる
* RT 契約: 合成は整数演算・確保なしで完結する必要がある
* `MediaTime` は `DEVICE_CLOCK_TIMEBASE`（1/1e9 ナノ秒）整数 tick — フレーム→ナノ秒の変換は有理数として定義可能
* 契約明文化のみ（代替案2）でも動くが、engine 側が `out.frames()` から時刻を再構成する義務を負う — 契約をシンプルに保つ方が利用側の負担を減らす

## Considered Options

* 案1: per-chunk offset 合成 — `frame_offset / sample_rate` ナノ秒を `checked_add` で `callback`/`playback` に加算
* 案2: 「chunk は同一 instant を共有」を port 契約として明文化 — engine 側がフレーム進行から時刻を再構成
* 案3: chunk 分割をやめて容量超過時に truncate/エラー — 契約を狭めるが受信データを捨てる

## Decision Outcome

Chosen option: "案1: per-chunk offset 合成", because `sample_rate` を state に保持するだけで RT-safe・整数演算のみで実現でき、engine 側に時刻再構成の義務を負わせずに正確な instant を提供できる。案2 は実装を手抜かない代わりに利用側契約を複雑化する。案3 はデータ損失を伴い既存の bounded 分割の目的を損なう。

### Consequences

* Good, because `MasterClock` が chunk 分割の有無に依らず一貫した instant 系列を受け取れる
* Good, because `sample_rate` 保持のみで実装可能（`negotiated.stream.sample_rate` は open 時に確定済み）
* Bad, because 合成値は「バッファ内位置に基づく推定」であり真の DAC 時刻ではない — ただし近似より厳密であり、cpal が per-callback 単一 timestamp のみ提供する現実では最良の近似
* Bad, because `checked_add` のオーバーフロー経路が増える — フォールバック（元 instant をそのまま渡す）を明文化して対処

### Confirmation

単体テストで per-chunk オフセット値・`playback: None`・フォールバック・単一 chunk 無変化を検証。`mise run test`/`test-rt` 全パス。

## Implementation Plan

* Affected paths: `crates/ruviform-audio-device/src/stream.rs`（`DataCallbackState` に `sample_rate` フィールド追加・`handle` で `write_interleaved` に渡す）、`crates/ruviform-audio-device/src/convert.rs`（`write_interleaved` に `sample_rate` 引数追加・chunk ループ内でオフセット合成）、`crates/ruviform-audio-device/src/device.rs`（`DataCallbackState::new` 呼び出し時に `negotiated.stream.sample_rate` を渡す）
* Patterns to follow: `MediaDuration` 経由の `checked_add`（time.rs の既存 API）・`DEVICE_CLOCK_TIMEBASE`・`#[non_exhaustive]`・`**RT contract:**` doc ラベル
* Patterns to avoid: callback 内での確保・浮動小数点演算・`unwrap`（`checked_add` 失敗は元値フォールバック）
* Dependencies or configuration: なし（`u32` フィールド追加のみ）

## Verification

* [x] chunk 分割時に k 番目 chunk の instant が `f_k / sample_rate` オフセットされる単体テスト
* [x] `playback: None`・`checked_add` 失敗・単一 chunk の3フォールバックテスト
* [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス

## Pros and Cons of the Options

### Option 1: per-chunk offset 合成

* Good, because engine に正確な instant を渡せる・利用側契約がシンプル
* Good, because RT-safe（整数演算・`Copy` 値型・確保なし）を維持
* Bad, because 合成値は推定値（真の DAC 時刻ではない）— 精度限界を doc に明記する必要

### Option 2: 同一 instant 共有を契約明文化

* Good, because 実装変更なし・adapter の責務最小
* Bad, because engine が `out.frames()` からフレーム進行を再構成する義務を負い、契約が複雑化
* Bad, because `callback`/`playback` の同一性を利用側が「分割の結果」と解釈する必要があり、API の誤読リスク

### Option 3: truncate/エラー化

* Good, because timestamp 契約が最も単純（分割自体がなくなる）
* Bad, because `frames_capacity` を超える backend 配信でデータを捨てる — PoC で確認済みの「容量超過を chunk 分割で吸収する」設計意図を損なう

## More Information

- Issue [#59](https://github.com/scarecrow173/ruviform/issues/59) / PR #47 レビュー
- 設計書 [../designs/audio-device-boundary-and-poc.md](../designs/audio-device-boundary-and-poc.md)「コールバック配管」— 近似の記録と将来検討項
- `MediaTime::checked_add` / `MediaDuration` — time.rs の既存整数演算 API
