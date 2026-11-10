---
id: "ADR-C3k293rplIbL9VJJzCrFKh"
type: "adr"
status: "accepted"
title: "Select MIDI input backend（Windows = raw WinMM / 他 OS = Unsupported 境界）"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/select-audio-device-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-253-os-midi-backend.md"
    - "docs/specs/issue-253-os-midi-backend.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://learn.microsoft.com/en-us/windows/win32/api/mmeapi/"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
  issue: 253
---

# ADR: Select MIDI input backend

## Status

accepted

## Context and Problem Statement

Issue #253 の OS MIDI input backend において、Windows で MIDI input
port の列挙・open・timestamp 付き event 受信を実装する必要がある。
`ruviform-audio-core::midi_input` の port contract に対して、
backend library を選定しなければならない。macOS/Linux は本 slice
では実装せず `Unsupported` 境界とする。

選択肢:

1. `midir` crate（Rust の WinMM/CoreMIDI/ALSA wrapper）
2. raw WinMM（`windows-sys` 経由の直接 FFI）
3. WinRT/UWP `Windows.Devices.Midi` API
4. 独自実装（本 ADR の対象外 — 常に 1-3 のいずれか）

## Decision Drivers

- **喪失 signal の観測可能性**: `MidiReadStats`（`dropped`/本 slice
  追加の `unsupported`）で backend 側喪失を defined に報告する
  必要がある（spec R2/R5）。`MIM_MOREDATA`/`MIM_LONGERROR` は
  WinMM 固有の喪失通知であり、wrapper が握りつぶすと契約を満たせない。
- **callback 生 timestamp**: `dwParam2` の生値を `InputStamp` に
  そのまま載せる必要がある。wrapper が timestamp を捨てる/変換
  する場合は provenance が崩れる。
- **依存追加基準**（AGENTS.md）: メンテ状況・ライセンス・依存木
  サイズ。`windows-sys` は workspace で既に lock 内に存在する。
- **UIPI・packaging 制約**（task 指定）: WinRT/UWP API は desktop
  plugin/CLI 文脈で制約が重い。
- **unsafe ポリシー**: 局所許可で FFI は可能だが、wrapper で
  避けられるなら避けるのが望ましい（ただし本件では wrapper が
  要件を満たさないため FFI を選ぶ）。
- **テスト容易性**: device 不在環境で open 失敗パスを deterministic
  に検証できること。

## Considered Options

### Option 1: `midir` crate

- Pros: WinMM/CoreMIDI/ALSA を統一 API で wrap。実装量が少ない。
- Cons:
  - `MIDI_IO_STATUS` flag・`MIM_MOREDATA`・`MIM_LONGERROR` を
    公開せず、driver 側喪失を観測不能 → defined-drop 契約に反する。
  - callback timestamp を捨てる（`MidiInput::read` は
    `(Vec<u8>, u64)` を返すが timestamp の clock domain 情報が
    失われる/独自変換される）。
  - device 不在時の列挙はできるが、不正 port id での open 失敗を
    deterministic に作れない（内部で index に変換するため）。
  - 依存追加の正当性が薄い（使う機能が subset）。
- Verdict: **不採用** — 喪失 signal と timestamp provenance の
  要件を満たせない。

### Option 2: raw WinMM（`windows-sys`）

- Pros:
  - `MIM_DATA`/`MIM_MOREDATA`/`MIM_LONGDATA`/`MIM_LONGERROR`/
    `MIM_CLOSE` の全 message を直接観測可能。
  - `dwParam1`/`dwParam2` をそのまま `MidiEvent`/`InputStamp` に
    写像できる（provenance = Carried を正直に維持）。
  - `MIDI_IO_STATUS` flag で driver queue 溢れを通知させられる。
  - `windows-sys` は薄い binding で依存木増分が小さい
    （feature 追加のみ、crate 自体は既存）。
  - 不正 device id での open 失敗を deterministic に unit test 可能。
- Cons:
  - FFI・unsafe が必要（局所許可で対処）。
  - WinMM 固有の制約（mid-stream device-loss 通知なし、
    timestamp が per-stream ms）を自前で扱う必要がある。
- Verdict: **採用** — 要件を最も正直に満たす。

### Option 3: WinRT/UWP `Windows.Devices.Midi`

- Pros: modern API、`MidiMessageReceivedEventArgs` で timestamp
  付き message を受け取れる。
- Cons:
  - UIPI・packaging 制約（desktop app では AppContainer/packaged
    context が必要な場合がある）。
  - `windows` crate の feature が大きく依存木が膨らむ。
  - plugin/CLI のような非 packaged context での利用に不向き。
- Verdict: **不採用** — task 指定の優先順位（WinMM/midir 系優先）
  と packaging 制約に反する。

## Decision

**Windows backend は raw WinMM（`windows-sys` 経由の直接 FFI）を
採用する。** macOS/Linux は本 slice では実装せず、
`cfg` で分離した stub が `DeviceError::Unsupported` を返す。

実装の詳細（module 構成・callback・queue・timestamp domain・
port 型への additive 変更）は
`docs/designs/issue-253-os-midi-backend.md` を正本とする。

## Consequences

### Positive

- `MIM_MOREDATA`/`MIM_LONGERROR`/`MIM_LONGDATA` を defined-drop/
  unsupported として観測可能にできる。
- timestamp を `InputStamp` に `Carried` provenance で保持できる。
- `windows-sys` feature 追加のみで依存木の増分を最小化できる。
- open 失敗パスを deterministic に unit test できる。

### Negative / Trade-offs

- unsafe FFI が必要（局所許可 + SAFETY コメントで対処）。
- WinMM 固有の制約（mid-stream device-loss 通知なし、per-stream
  timestamp）を設計で明示的に扱う必要がある。
- macOS/Linux は Unsupported のまま（別 slice で実装）。

### Follow-up Tasks

- macOS CoreMIDI backend（別 slice）
- Linux ALSA/JACK/PipeWire backend（別 slice）
- opt-in 実デバイス lane（mise task）・実 DAW 往復（別 slice）
- `PumpStats`/`InputTx` 側で `MidiReadStats::unsupported` を
  透過する対応（bridge 側、別 slice）

## Implementation Plan

1. `crates/ruviform-audio-core` に additive 変更:
   `MidiPortOrigin::Unknown`・`MidiReadStats.unsupported`。
2. `crates/ruviform-midi-device` 新設:
   - `src/lib.rs`・`src/system.rs`（factory）
   - `src/midi.rs`（dwParam1/dwParam2 decode、pure）
   - `src/queue.rs`（bounded queue + stats、pure）
   - `src/os/windows.rs`（WinMM FFI）
   - `src/os/unsupported.rs`（非 Windows stub）
   - `tests/`・`examples/`
3. workspace member 追加・`crates/README.md` 更新・crate README。
4. `Cargo.lock` 再生成。

## Verification

- [ ] `mise run fmt-check` Pass
- [ ] `mise run check` Pass
- [ ] `mise run lint` Pass（unsafe 局所許可のみ）
- [ ] `mise run test` Pass（pure module 単体テスト + 結線テスト）
- [ ] `mise run test-rt` Pass
- [ ] 非 Windows target で `cargo check` Pass（Unsupported stub）
- [ ] 実 MIDI デバイスなし環境で全 lane Pass
- [ ] `docs/designs/issue-253-os-midi-backend.md` の制限事項を
      PR/IR に転記
