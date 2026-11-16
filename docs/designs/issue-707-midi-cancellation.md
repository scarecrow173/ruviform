---
id: "DESIGN-034YJiaD4GvnZzHb0ClYwr"
type: "design"
status: "approved"
title: "Issue #707 — MIDI input cancellation fault detailed design"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/designs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-close-out.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-707-midi-cancellation.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-707-midi-cancellation.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-707-midi-cancellation.md"
    - "docs/designs/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-707-midi-cancellation.md"
    - "docs/test-specs/issue-707-midi-cancellation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-707-midi-cancellation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-707-midi-cancellation.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #707 — MIDI input cancellation fault detailed design

## Context

#628 close-out（merge `0b510cf8`）で audio stream に
`StreamFault::Cancellation` を追加した: 当該 callback で data
callback を発火せず（in-flight delivery の中断）、error callback に
`DeviceError::Backend("cancelled")` を配送し、stream を終了する
（後続 pump は `StreamInvalidated`）。production 契約に cancel 概念が
ないため、既存 observable のみで表現する写像規約が確立済み。

MIDI input path は別構造を持つ: `MidiFeed`（純粋変換層）→
`MockPort`（`Arc<Mutex<PortState>>` 共有 queue）→ `MockInput::read`
（`DeviceResult<MidiReadStats>`）→ `InputPump::pump`（non-RT→RT
handoff）→ `InputStage::collect_block`（production 写像）。
`MidiInput::read` は `Err(DeviceError)` を返せるため、
`Backend("cancelled")` + 後続 `StreamInvalidated` を既存契約で
そのまま表現できる。

## Scope

### In Scope

- `crates/ruviform-audio-core/src/midi_input/mock.rs` の
  `MockPort`/`MockInput` への read-level fault injector 追加と、
  `MockInput::read` の fault 消費経路。
- `mock.rs` 内 unit test と `crates/ruviform-audio-test/tests/
  midi_faults.rs` の pipeline 観測 test。

### Out of Scope

- production trait（`MidiInput`/`MidiInputPort`/
  `MidiInputEnumerator`）・`DeviceError` の変更 — test-only 境界
  不変。
- `MidiFeed`/`MidiLeg`/`InputPump`/`InputStage` の変更 — 既存の
  `DeviceError` → `PipelineError::Device` 伝播で observable が
  完結するため不要（後述の設計判断）。
- audio stream 側・実 endpoint/実デバイス（#628 済・#629/#630 領域）。

## Components and Boundaries

### 採用案: (a) `MockPort` read-level fault injector

`MockPort` は「backend が観測可能にできること」の所有層であり、
既存の `fail_next_open`/`inject_dropped`/`inject_unsupported` と
同じ面に read-level fault を追加する。`MockPort` の consumer
（`ruviform-audio-plugin-bridge` の `midi_wiring` test、
plugin-test e2e、audio-test の `MidiLeg`）全てが同じ fault 面を
使える。

`PortState` への追加:

```rust
struct PortState {
    // …既存 field…
    /// Backend-side termination (cancel) — consumer `close` とは別
    /// 系統。両方が `read` を `StreamInvalidated` で失敗させる。
    terminated: bool,
    /// `read` が FIFO で消費する armed fault 列。
    read_faults: VecDeque<ReadFault>,
}

/// `read` が arm 順に消費する scripted fault（private）。
enum ReadFault {
    /// 1-shot read 失敗 — subscription は生存し、`out`・queue・
    /// dropped/unsupported counter のいずれも消費しない。
    Fail(DeviceError),
    /// cancelled read — `Backend("cancelled")` を 1 度返して
    /// subscription を終了させる。
    Cancel,
}
```

`MockPort` の新規 public handle method（`fail_next_open` と同型）:

- `fail_next_read(error: DeviceError)` — 次の `read` が `error` で
  失敗する 1-shot fault。subscription は生存する（負例のための
  「非終端 read error」面）。
- `inject_cancelled()` — 次の `read` が `Backend("cancelled")` で
  失敗し、何も配送せず subscription を終了させる。以後の `read`
  は全て `StreamInvalidated`。
- `is_terminated()` — backend 起因の終了を観測する test handle
  （`is_closed` は consumer `close` のみを報告するため区別する）。

`MockInput::read` の判定順序（既存 `closed` check の直後、drain
の前）:

1. `closed || terminated` → `Err(StreamInvalidated)`（dead
   subscription では armed fault を消費しない）。
2. `read_faults.pop_front()`:
   - `Fail(e)` → `Err(e)`（subscription 生存・queue/counter 不変）。
   - `Cancel` → `terminated = true` →
     `Err(Backend("cancelled"))`。
3. 通常 drain（既存経路）。

### 不採用案: (b) `MidiLeg`/`MidiFeed` 側の fault script

leg/pump 単位で `pump_inputs` が `PipelineError::Device` を返す
script を audio-test 側に持たせる案。以下の理由で不採用:

- 「subscription が終了した」という状態を leg 側 flag で模倣すると、
  `MockPort`/`MockInput` の observable（`read` の `Err` 系列・
  `is_closed`/`is_terminated`）と pipeline の observable が乖離する。
  本物の subscription 死を再現するには結局 `MidiInput` の wrapper
  を自作することになり、fixture の責務を test crate 側へ複製する。
- `MockPort` の他 consumer（plugin-bridge `midi_wiring`、plugin-test
  e2e）が同じ fault を使えない。
- `fail_next_open`/`inject_dropped`/`inject_unsupported` の既存 fault
  面と責務が分散し、「backend が何を観測可能にできるか」の
  authoritative な実装が 2 箇所に分かれる。

### `MidiFeed`/`MidiLeg` は無変更

`pump_inputs` は `input.read` の `DeviceError` を
`PipelineError::Device` へそのまま写像済みで、`port()` が
`&MockPort` を公開済み — injector の arm・観測に必要な面は全て
既存。production 経路への差分ゼロが本 slice の境界条件。

### cue-level cancel の判定

`MidiFeed` の「位置 X 以降の cue を配送しない」粒度の cancel は
要求しない: subscription-level cancel が内包する（cancel 後に due
となった cue は `feed.pump_into_port` で port queue に push されるが
`read` が死んでいるため stage へ届かない — `pending_len`/
`feed_remaining`/`counters` で観測可能）。cue 未配送は scenario
内容であり fault ではない。

## Data and Control Flow

- Arm: test が `port.fail_next_read(e)` / `port.inject_cancelled()`
  を呼ぶ（open 前の pre-script、または `leg.port()` 経由の
  mid-stream inject の両方が可能 — `PortState` は open をまたいで
  生存）。
- Consume: `MockInput::read` が `read_faults` を FIFO で 1 件消費。
  `Fail` は `Err(e)` を返して終了。`Cancel` は `terminated` を立てて
  `Err(Backend("cancelled"))` を返す。
- Termination: 以後の `read` は `closed || terminated` で
  `Err(StreamInvalidated)`。`close()` との違いは `is_closed()` が
  false のままである点のみ（consumer は close していない）。
- Pipeline 観測: `pump_inputs`/`pump_block`/`pump_next` が
  `Err(PipelineError::Device(Backend("cancelled")))` →
  `Err(PipelineError::Device(StreamInvalidated))` を返す。
  `collect_block` は channel/pending のみを drain する RT 面であり、
  subscription 死の影響を受けない（cancel 前に channel へ受理済みの
  event は配達される — in-flight の遡及取消はしない）。

## Non-Functional Constraints

- `MockPort` は non-RT fixture — 全 method が lock/alloc 可能。
  RT 契約経路（`InputStage::collect_block` 等）へ変更を持ち込まない。
- `unsafe` なし。`ReadFault` は crate-private enum で
  `#[non_exhaustive]` 対象外。`MockPort` の public API 追加は
  additive で既存 consumer を壊さない。
- armed fault は FIFO 消費 — `fail_next_read(X)` →
  `inject_cancelled()` の順で arm すれば `X` → `cancelled` →
  `StreamInvalidated` の ordered error 系列になる。

## Risks and Trade-offs

| Risk | Mitigation |
| --- | --- |
| `Backend("cancelled")` の解釈が audio 側とずれる | 同じ分類名・同じ lifecycle（terminate → `StreamInvalidated`）を rustdoc・design・test-spec に明記 |
| read error が event を消費しない semantics が consumer 期待とずれる | `Fail`/`Cancel` とも drain 前に return — `out`/queue/counter 不変を doc と unit test で pin |
| armed fault が dead subscription で誤発火 | `closed || terminated` を fault 消費より先に判定 — unit test で pin |
| `is_terminated` 追加が過剰 | `is_closed`（consumer close のみ）との区別を観測する test handle として最小限の accessor のみ追加 |

## Deferred Design Concerns

- cue-level cancel（spec の Deferred Design Concerns と同じ）。
- `MockPort` の re-open（cancel 後の port 再利用）— 現行 fixture は
  `opened` latch で 1 port = 1 subscription。reconnect 試験が必要に
  なれば別 slice で検討。
- audio stream 側の cancel が error callback に
  `StreamInvalidated` を併配送するのに対し、MIDI の `read` は
  `Err` を 1 件しか返せない — port 契約に error callback がない
  ため構造的に 1 error/read。写像の差異として記録。

## Verification Notes

- [ ] `mise run fmt-check` / `check`/`lint`/`test
  ruviform-audio-core`・`ruviform-audio-test` / `test-rt` /
  `prek run --all-files` が pass。
- [ ] `git diff` が許可範囲内のみ（production trait・pipeline
  配線・audio 側 fault 実装に差分なし）。
- [ ] docs index は Coordinator が統合時に再生成（Worker は
  編集しない）。
