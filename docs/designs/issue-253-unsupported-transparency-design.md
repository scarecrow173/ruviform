---
id: "DESIGN-034VEQoHhDL68K8qLl8w8r"
type: "design"
status: "approved"
title: "Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane scaffold"
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
  implements:
    - "docs/specs/issue-253-unsupported-transparency.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-253-os-midi-backend.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
    - "docs/impl/ir/rf-253-l1-report-verb.md"
    - "docs/impl/ir/issue-253-os-midi-input-backend-adapter-windows-winmm-os-unsupported.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-unsupported-transparency.md"
    - "docs/designs/issue-253-os-midi-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# 設計: Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane scaffold

上流: `docs/specs/issue-253-unsupported-transparency.md`。前提実装:
PR #561 の `ruviform-midi-device`（`MidiReadStats.unsupported` を
報告する唯一の実 backend）と `midi_input::mock` の
`inject_dropped` 注入面、`InputTx::pump`/`InputStage::read` の
`dropped` 透過経路（Issue #188）。

## Context

`MidiReadStats` は 2 つの since-last-read 計数を運ぶ:

- `dropped`: backend が event を受信する前に失った数（queue
  overflow・`MIM_MOREDATA` 等の driver-level loss）— 過負荷
  signal。
- `unsupported`: backend が受信したが declared channel-voice 語彙
  （`MidiEvent`）へ翻訳不能だった数（SysEx・system common/
  realtime・malformed bytes）— 語彙設計上の除外 traffic。

`ruviform-midi-device` の `queue.rs::Shared::drain` は両者を
`MidiReadStats::with_unsupported` で返す。しかし bridge 側では:

- `InputTx::pump`（`crates/ruviform-audio-plugin-bridge/src/input.rs`）
  は `stats.dropped` だけを `dropped_backend` に累積し、
  `stats.unsupported` を読み捨てる。
- `InputStage::read`（同上）も同様に `stats.unsupported` を
  読み捨てる。
- `midi_input::mock` の `MockInput::read` は常に `unsupported=0`
  を返すため、透過欠落を deterministic に観測する注入手段が無い。

破られた不変条件: 「adapter の observable loss/untranslatable
計数は engine の control/status surface まで silent でなく届く」
（`midi_input.rs` module doc・`queue.rs` module doc・spec R1）。

## Scope

### In Scope

- bridge カウンタ透過（`input.rs`・`status.rs`）、mock 注入
  （`mock.rs`）、deterministic test（bridge tests）、`#[ignore]`
  実機 E2E（midi-device tests + bridge tests）、evidence 記録、
  dev-dep 追加。

### Out of Scope

- `mise.toml` canonical lane 登録（別 Worker 占有のため本設計の
  対象外 — 後続 slice `chore/253-midi-real-device-lane` で
  `test-midi-real-device` として登録済み）、macOS/Linux
  backend、実 DAW、`Verified` 昇格、V9-V13、video verb、MIDI2、
  consumer 消費実装、Issue Close。

## Components and Boundaries

### 1. `ruviform_audio_core::midi_input::mock`（注入面）

`PortState` に `unsupported: u64` field を追加（`dropped` と同じ
since-last-read 消費意味論）。

```rust
struct PortState {
    info: MidiInputInfo,
    pending: VecDeque<TimedMidiInput>,
    dropped: u64,
    unsupported: u64,          // NEW
    opened: bool,
    closed: bool,
    open_error: Option<DeviceError>,
}
```

- `MockPort::inject_unsupported(&self, n: u64)`: `lock().unsupported
  += n`。`inject_dropped` と同じ形状・doc 規約。
- `MockInput::read`: `MidiReadStats::with_unsupported(
  take(dropped), take(unsupported))` を返す。`read` の既存意味論
  （`closed`→`StreamInvalidated`、bounded drain）は不変。

### 2. `InputTx`（non-RT producer 半 — channel 経路の計数owner）

`MidiInput::read` を呼ぶのは `InputTx::pump` なので、channel 経路
（`InputTx`→SPSC→`InputStage::pump`）では `InputTx` が
`unsupported` の計数 owner である。`dropped_backend` と同じ形状で:

```rust
pub struct PumpStats {
    pub received: usize,
    pub pushed: usize,
    pub deferred: usize,
    pub backend_dropped: u64,
    /// NEW: 今回の read が報告した語彙外（翻訳不能）計数。
    pub backend_unsupported: u64,
}

pub struct InputTx {
    tx: Producer<TimedMidiInput>,
    staging: Vec<TimedMidiInput>,
    dropped_backend: u64,
    unsupported_backend: u64,  // NEW
}
```

- `pump` 内: `self.unsupported_backend += stats.unsupported;`、
  `PumpStats { ..., backend_unsupported: stats.unsupported }`。
- `pub const fn unsupported_backend(&self) -> u64` getter を
  `dropped_backend()` の直後に追加。
- `PumpStats` は既に `#[non_exhaustive]` — field 追加は additive
  で破壊的変更なし。

### 3. `InputStage`（RT 側 — direct-read 経路の計数 owner）

`InputStage::read` は channel を介さず `MidiInput` を直接 drain
する経路（engine が channel 分割を取らない構成用）。ここでも
`stats.unsupported` を stage カウンタに累積する:

```rust
pub struct InputStageCounters {
    pub mapped: u64,
    pub delivered: u64,
    pub late: u64,
    pub rejected: u64,
    pub dropped_backend: u64,
    /// NEW: backend が受信したが語彙へ翻訳不能だった累積計数。
    pub unsupported_backend: u64,
    pub dropped_capacity: u64,
    pub released: u64,
    pub ledger_overflows: u64,
}
```

- `InputStage::read` 内: `self.counters.unsupported_backend +=
  stats.unsupported;`（`dropped_backend` 累積の直後）。
- `InputStageCounters` も既に `#[non_exhaustive]`。
- `InputStage::pump`（channel→pending の RT 側）は `MidiReadStats`
  を見ない — channel 経路の `unsupported` は `InputTx` 側にのみ
  存在する（責務境界: `MidiInput::read` を呼ぶ側が計数 owner）。
  これは `dropped` と同一の既存分割であり、本設計はその形状を
  そのまま踏襲する。

### 4. `NodeStatus` mirror（`status.rs`）

`input_*` atomic 群に `input_unsupported_backend: AtomicU64` を
追加（`input_dropped_backend` の直後）。`set_input` で
`c.unsupported_backend` を absolute store、`snapshot()` で
`input.unsupported_backend` として load。`Counter` enum には
追加しない — この atomic は `bump` 対象ではなく mirror 専用
（既存 `input_*` 群と同じ）。

### 5. `#[ignore]` 実機 E2E

**`ruviform-midi-device/tests/ports.rs`** — 既存の
`real_device_open_read_close` を全経路へ拡張:

1. `backend().input_ports()` → Err(Unsupported) なら記録して
   return（非対応 OS の honest NotRun）。ports が空なら skip。
2. 先頭 port の `info()` を記録（id/name/origin/capabilities）。
3. `open()` → `Box<dyn MidiInput>`。
4. 複数回 `read`（短い dwell で実 event の到達機会を作る）。
   per-read `stats.dropped`/`stats.unsupported` と受信 event の
   `stamp.domain`/`stamp.base`/`stamp.provenance`/`stamp.ticks`
   を記録。assertion: event の `stamp.domain` が port 宣言の
   `clock_domain` と一致すること（adapter の「declared domain で
   stamp する」契約）、provenance が `Carried`、ticks が単調非減。
5. `close()` → 後続 `read` が `Err(StreamInvalidated)`。

**bridge tests（`midi_wiring.rs` 末尾）** — `SystemBackend`→
`InputTx::pump`→`InputStage`→`NodeStatus` mirror の実機 E2E:

1. `build_rig(128, true)` で input leg 付き rig。
2. `ruviform_midi_device::backend().input_ports()` → 同上の
   skip/Unsupported 処理。port の宣言 domain/base から
   `ClockAnchor` を構築して `control.set_clock_anchor`。
3. `open()` → 複数回 `input_tx.pump(&mut *input)`（dwell 付き）。
   `PumpStats.backend_unsupported`/`backend_dropped` の合計が
   `input_tx.unsupported_backend()`/`dropped_backend()` 累積と
   一致することを assert（計数透過の実機検証 — 本 slice の主
   目的）。
4. `feed`+`rt.process` → `control.snapshot().input` の mirror を
   記録（stage 側計数は direct-read 経路のもの — channel 経路で
   は 0 のまま。これは設計上の境界であり assert もその形で固定）。
5. `input.close()` → `pump` が `Err(StreamInvalidated)`。

実機依存の値（event 有無・unsupported/dropped の非零）は assert
せず記録のみ — hardware traffic を Pass 条件にしない（実機の
physical 操作なしに SysEx 等を発生させる手段は lane 内に無い）。
assert するのは「計数の一貫性」と「契約上不変の性質」のみ。

### 6. dev-dependency

`ruviform-audio-plugin-bridge/Cargo.toml` の
`[dev-dependencies]` に `ruviform-midi-device = { path =
"../ruviform-midi-device" }` を追加。dev-dep のみで public
surface・downstream への影響なし。依存方向 bridge→adapter は
adapter→adapter/adapter→engine 禁止規約に抵触しない
（`ruviform-midi-device` は bridge に依存しないため cycle なし）。
windows-sys は midi-device の `[target.'cfg(windows)']` 依存なので
非 Windows での解決も問題なし。

## Data and Control Flow

```text
adapter (WinMM callback) ──push_unsupported──▶ Shared.unsupported
        │ MidiInput::read ──▶ MidiReadStats.unsupported
        ▼
[channel 経路]                          [direct-read 経路]
InputTx::pump                            InputStage::read
  ├─ PumpStats.backend_unsupported         └─ InputStageCounters
  └─ InputTx.unsupported_backend  (累積)       .unsupported_backend (累積)
                                                     │ set_input (callback 末尾)
                                                     ▼
                                          NodeStatus.input_unsupported_backend
                                                     │ snapshot()
                                                     ▼
                                          NodeStatusSnapshot.input
                                            .unsupported_backend
```

channel 経路で `InputStageCounters::unsupported_backend` が 0 に
留まるのは設計上の責務境界（`MidiInput::read` の呼出側が計数
owner）。`dropped_backend`/`dropped_capacity` と同一の分割。

## Alternatives

- **`InputTx` の計数を `NodeStatus` にも mirror する**: `pump` は
  node の callback ではなく consumer の control thread から呼ば
  れるため、`InputTx` は `NodeStatus` を持たない。`dropped_backend`
  も同じ構造（getter のみ・status mirror 無し）であり、本 slice
  はその既存形状を踏襲する。status mirror 統合は deferred
  concern として spec に記録。
- **`unsupported` を `dropped` に fold する**: 却下。port 契約が
  両者を意味的に区別する（overload loss vs 語彙外 traffic）ため、
  bridge で混ぜると adapter の honest 分類を壊す。
- **canonical lane（mise task）を先に登録する**: `mise.toml` が
  別 Worker 占有のため不可。暫定入口（`cargo test -p
  ruviform-midi-device -- --ignored` / `cargo test -p
  ruviform-audio-plugin-bridge -- --ignored`）を docs に記録し、
  follow-up 行として残す（→ 実施済み: `chore/253-midi-real-device-lane`
  slice で `test-midi-real-device` として登録済み）。

## Test Strategy

`docs/test-specs/issue-253-unsupported-transparency.md` が正本:

- mock unit test: `inject_unsupported` が since-last-read で一度
  だけ surface し `dropped` と独立であること。
- `InputStage::read` integration test（`midi_delivery.rs`）:
  scripted `unsupported` が `counters().unsupported_backend` に
  累積されること、`dropped_backend` と独立であること。
- `InputTx::pump` integration test（`midi_wiring.rs`）:
  `MockBackend` 経由で `PumpStats.backend_unsupported`・
  `InputTx::unsupported_backend()` が観測できること。
- `NodeStatus` mirror: `set_input`→`snapshot` で
  `input.unsupported_backend` が mirror されること。
- `#[ignore]` 実機 E2E（`ports.rs` + `midi_wiring.rs` 末尾）:
  上記 §5 の経路。

## Risks

- `#[non_exhaustive]` field 追加による downstream 影響: なし
  （additive・public API 互換）。`InputStageCounters` の struct
  literal 構築は同一クレート外では `#[non_exhaustive]` により元々
  不可能 — `status.rs::snapshot` 内の literal 構築のみ更新。
- `#[ignore]` 実機 E2E が MIDI port 無し環境で pass する:
  意図通り（skip = honest）。port 有無・unsupported 非零は
  evidence に実測記録。
- bridge dev-dep による feature unify: midi-device は feature を
  持たない・dep tree は audio-core/media-core のみ — 無害。
- `assert_no_alloc` global allocator（`midi_wiring.rs`）との干渉:
  実機 E2E は `assert_no_alloc` guard 内に入れないため影響なし
  （既存 test と同じ）。
