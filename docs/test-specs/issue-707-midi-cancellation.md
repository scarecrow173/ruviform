---
id: "TSPEC-034YJlkbKnctq9LE7na0TZ"
type: "test-spec"
status: "approved"
title: "Issue #707 — MIDI input cancellation fault test specification"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/designs/issue-707-midi-cancellation.md"
    - "docs/plans/issue-707-midi-cancellation.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-707-midi-cancellation.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/designs/issue-707-midi-cancellation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-707-midi-cancellation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-707-midi-cancellation.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-707-midi-cancellation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #707 — MIDI input cancellation fault test specification

## Purpose

Issue #707 slice が守るべき契約を固定する: `MockPort` の
read-level fault injector（`fail_next_read`/`inject_cancelled`）と、
`MidiLeg` 経由で観測される `Backend("cancelled")` + subscription
終了の Expected/Observed 一致。deterministic backend の Pass は
OS-L1 証拠であり、実 OS/実 MIDI デバイス path の証明ではない。

## Feature: MIDI input read-level fault injection

### Rule RD-1: `fail_next_read` は非終端の 1-shot read error
Given `port.fail_next_read(error)` を arm した subscription、
次の `read` は `Err(error)` を返すが subscription は生存する:
`out`・queue・`dropped`/`unsupported` counter のいずれも消費
されず、後続の `read` は正常に drain する。`is_terminated()` と
`is_closed()` は共に false のまま。

### Rule RD-2: `inject_cancelled` は cancelled read + 終了
Given `port.inject_cancelled()` を arm した subscription、
次の `read` は `Err(DeviceError::Backend("cancelled"))` を返し
`out` へ何も append しない。その read で subscription は
backend 側終了（`is_terminated() == true`、`is_closed() ==
false` — consumer は close していない）し、以後の全 `read` は
`Err(DeviceError::StreamInvalidated)` を返す。queue に残った
event は二度と配送されない。

### Rule RD-3: armed fault は FIFO 消費・dead subscription では不発火
複数の read fault を arm した場合、`read` は arm 順に 1 件ずつ
消費する（`fail_next_read(X)` → `inject_cancelled()` なら
`X` → `Backend("cancelled")` → `StreamInvalidated` の ordered
系列）。`closed`/`terminated` の subscription では armed fault を
消費せず即 `StreamInvalidated`。

### Rule RD-4: pipeline 観測（`MidiLeg`）
`pump_inputs`/`pump_next`/`pump_block` は cancelled read を
`Err(PipelineError::Device(DeviceError::Backend("cancelled")))`、
終了後を `Err(PipelineError::Device(DeviceError::StreamInvalidated))`
として返す。cancel された block で port queue に push 済みの cue
は stage へ届かない（`InputStage` counters・`pending_count` で
観測）。cancel 前に channel へ受理済みの event は `collect_block`
で通常通り配達される（遡及取消しない）。

### Rule RD-5: 負例 — error 値・loss counter は終了しない
`fail_next_read(DeviceError::Backend("cancelled"))`（同一 error
値の非終端 fault）では subscription が終了せず後続 block が
正常に配送される。`inject_dropped`/`inject_unsupported` も
subscription を終了させない（stats として観測されるのみ）。

### Rule RD-6: production 契約不変
`git diff` は `crates/ruviform-audio-core/src/midi_input/mock.rs`
と `crates/ruviform-audio-test/tests/`・docs/evidence のみ。
`MidiInput`/`MidiInputPort` trait・`DeviceError`・pipeline 配線
（`src/backend/midi.rs`/`src/pipeline/midi.rs`）に差分なし。

## Examples

- `inject_cancelled` を pre-arm した leg で `pump_next(BLOCK)`:
  due cue は port queue に push されるが `read` が
  `Backend("cancelled")` で失敗 → `pending_len()==1`・
  `feed_remaining()==0`・`counters().mapped==0`。次の
  `pump_next` は `StreamInvalidated`。
- block 0 正常配送後に `leg.port().inject_cancelled()`:
  block 1 の `pump_next` が `Backend("cancelled")` で失敗し、
  その block で due だった cue は未配送のまま滞留。以後の
  pump は `StreamInvalidated`。
- `fail_next_read(Backend("cancelled"))` を arm: pump が同 error
  で失敗するが `is_terminated()==false`、次の pump は成功し
  後続 cue が通常配送される。

## Guarantees

- cancelled read が in-flight delivery の中断として観測可能で
  あること（`out` 非 append + `Backend("cancelled")` + 終了）。
- `Backend("cancelled")` の error 値単体では終了しないこと
  （termination は cancel fault の semantics であり error 値では
  ない）。
- armed fault の消費順が決定的（FIFO）であること。

## Non-goals

- 実 MIDI デバイス・実 endpoint 上の fault 実 timing（EXT-L3・
  `scarecrow173/ruviform#629`/`#630` 領域 — NotRun 簿記のみ）。
- audio stream 側の cancellation（#628 close-out で完了済み）。
- cue 粒度の cancel（subscription-level cancel が内包 — spec の
  Deferred Design Concerns 参照）。
- `rt_` contract test の新規追加（変更は non-RT fixture と
  test のみ。既存 `rt_` 非回帰は `mise run test-rt` で確認）。
- receipt ledger（`run_midi_trace` の `MidiTrace::receipts`）との
 直接相互作用 — 本 slice は input path のみで transport scheduler
  を触らない。整合は既存 `tests/midi.rs` の receipt test 非回帰
  （`mise run test ruviform-audio-test` 収集）で確認する。

## Risk

- `read` が `Err` と stats を同時に返せない契約上、audio 側の
  「error callback + data callback」併存に相当する観測は
  `fail_next_read`（失敗 read）と `inject_dropped`/
  `inject_unsupported`（stats 付き成功 read）に分かれる — この
  分解が consumer 期待とずれた場合、Issue close 判定で指摘
  される可能性。design doc の Deferred Design Concerns に記録済み。
