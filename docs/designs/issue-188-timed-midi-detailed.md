---
id: "DESIGN-Gy2lun1o6N1NDRI7tfSArM"
type: "design"
status: "approved"
title: "Issue #188 Timed MIDI — detailed API and contracts"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source: []
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
    - "docs/designs/issue-188-timed-midi.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md"
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/discovery/issue-188-timed-midi-inventory.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-188-timed-midi.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/188"
  defers: []
  deferred-by: []
metadata:
  gate: "M3"
  issue: 188
---

# Issue #188 Timed MIDI — detailed API and contracts

Overview は `docs/designs/issue-188-timed-midi.md`。本書は PoC で実装する API 形状・不変条件・fault 分類を確定する。全て `Copy`/`#[non_exhaustive]`/validated-newtype の既存パターンに従う。コード例は契約の形を示すもので実装詳細の規定ではない。

## 1. `ruviform-media-core::midi` — MIDI 語彙

```rust
pub struct MidiChannel(u8);   // 0..=15, TryFrom<u8> / new returns Result
pub struct MidiGroup(u8);     // 0..=15
pub struct NoteKey(u8);       // 0..=127
pub struct MidiRangeError { /* kind + value */ }

#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MidiKind {
    NoteOn         { key: NoteKey, velocity: u32 },  // MSB-justified
    NoteOff        { key: NoteKey, velocity: u32 },
    ControlChange  { controller: u8, value: u32 },   // controller 0..=127
    PitchBend      { value: i32 },                   // 0=center, MSB-justified
    PolyPressure   { key: NoteKey, pressure: u32 },
    ChannelPressure{ pressure: u32 },
    ProgramChange  { program: u8 },                  // 0..=127
    // System/SysEx は将来 variant 拡張（wire tag は別途 version negotiation）
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MidiEvent { pub group: MidiGroup, pub channel: MidiChannel, pub kind: MidiKind }
```

### Canonical value 規約（MSB-justified）

| kind | MIDI1 source | UMP source | canonical |
| --- | --- | --- | --- |
| velocity/pressure | 7bit `v` | 16bit `v` | `u32` に `v << 25` / `v << 16`（最上位ビット側へ正規化） |
| pitch bend | 14bit `-8192..=8191` | 32bit | `i32` MSB-justified、0=center |
| controller/program | 7bit | — | `u8` |

`MidiValue` helper: `from_u7(v)->u32`/`from_u14(v)->u32`/`from_u16(v)->u32`/`as_u7(v)->u8`/`as_u14(v)->u16`/`as_u16(v)->u16` と bend 用 `i32` 版。`as_u7` 等の縮退変換は **rounding は adapter 責務**、helper は切り捨て/飽和の defined 規則のみ。

### `NoteLedger`（RT-safe、bounded）

```rust
pub struct NoteLedger { /* fixed [Option<LedgerEntry>; N] */ }
pub struct LedgerKey { pub group: MidiGroup, pub channel: MidiChannel, pub key: NoteKey }

impl NoteLedger {
    pub fn note_on(&mut self, e: &MidiEvent) -> NoteLedgerUpdate;
    pub fn note_off(&mut self, e: &MidiEvent) -> NoteLedgerUpdate;
    pub fn feed(&mut self, kind: &MidiKind, group, channel) -> NoteLedgerUpdate;
    pub fn is_active(&self, key: &LedgerKey) -> bool;
    pub fn active_count(&self) -> usize;
    /// offset 0 で配送すべき release event を scratch へ列挙。戻り値は書き込み数。
    pub fn release_all(&mut self, scratch: &mut [MidiEvent]) -> usize;
    pub fn take_overflow(&mut self) -> bool; // force-release が走ったか
}
```

- 容量 `NOTE_LEDGER_CAPACITY`（定数、PoC 128）。超過時は **最古 entry を force-release し overflow flag**（drop しない — release が安全側）。
- MIDI1 duplicate-pitch: 同一 LedgerKey の重複 Note-On は count 管理（`note_on` で +1、`note_off` で -1、0 で inactive）。voice 個別識別は不可能であり、これは MIDI1 仕様の制約として doc 明記。`Note-Off priority` 規約で同一 frame の on/off 競合を制御。

## 2. `ruviform-media-core::timed_input` — stamp / clock domain / mapper

```rust
pub struct ClockDomainId(pub u64);            // opaque domain tag
pub enum ClockDomain { Sample, Monotonic, Unknown } // 定義済み domain の語彙
#[non_exhaustive] pub enum TimestampProvenance { Carried, Derived, Estimated } // 既存再利用

#[derive(Copy, Clone, Debug)]
pub struct InputStamp {
    pub ticks: u64,
    pub base: MediaTimeBase,        // RationalTimeBase
    pub domain: ClockDomainId,
    pub provenance: TimestampProvenance,
}

#[derive(Copy, Clone, Debug)]
pub struct TimedInputEvent<P> { pub stamp: InputStamp, pub event: P }

#[derive(Copy, Clone, Debug)]
pub struct ClockAnchor {
    pub domain: ClockDomainId,
    pub stamp_ticks: u64,
    pub stamp_base: MediaTimeBase,
    pub frame: EngineFrame,          // anchor 時点の EngineFrame
    pub calibration: TimestampProvenance,
}

#[non_exhaustive]
pub enum MapRejection {
    UnmappedDomain,     // anchor.domain != stamp.domain または anchor 不在
    EpochMismatch,      // anchor.frame.epoch != current epoch
    BeforeAnchor,       // stamp < anchor（負 delta）
}

pub struct MappedInput<P> { pub event: P, pub frame: EngineFrame, pub provenance: TimestampProvenance }

/// 単一権威写像。anchor delta (stamp.ticks - anchor.stamp_ticks) を stamp_base→frame_base へ
/// convert し anchor.frame へ加算。丸めは単回 NearestEven。
pub fn map_stamp_to_frame<P:Copy>(
    stamp: &InputStamp, anchor: &ClockAnchor, frame_base: MediaTimeBase, now: EngineFrame,
) -> Result<MappedInput<P>, MapRejection>;
```

- `ClockDomainId` は opaque（生成/登録は非RT所有者）。定義済み domain 語彙は `ClockDomain` で提供するが `ClockDomainId` 自体は自由値。
- 信頼度継承: `MappedInput.provenance = min(stamp.provenance, anchor.calibration)`（Carried>Derived>Estimated の弱い方）。
- epoch 跨ぎ禁止: `anchor.frame.epoch != now.epoch` → `EpochMismatch`（`ClockEpochId` 不一致は減算しない、#190 規約）。

## 3. `ruviform-plugin-core` — EventKind 拡張・wire・merge

```rust
// event.rs
#[non_exhaustive]
pub enum EventKind {
    ParamValue(ParamEvent),
    Midi(MidiEvent),            // NEW
}
```

- `Event`/`EventList`/`reserved`/`sort_events_by_frame_offset` 不変。`MidiEvent` は `Copy` で wire 形に収まる（`Event` サイズ増なし）。
- wire.rs: `EVENT_KIND_MIDI: u16 = 1`（`EVENT_KIND_PARAM_VALUE: u16 = 0` 不変）。未知 tag は既存 `Malformed`。
- `merge_call_events`（`transport_bridge.rs` に配置、`build_call_event_list` と同じ権威パターン）:

```rust
pub fn merge_call_events(
    plan: &[Event], live: &[Event], scratch: &mut [Event],
) -> Result<&mut [Event], MergeOverflow>;
```

Merge 規約（中立・明文化）:
1. 両入力は非減少 `frame_offset`（各 source の causal order は merge で保持）。
2. 同一 offset の cross-source tie は **plan-before-live**。
3. 同一 offset・同一 `{group,channel,key}` の `Midi` events は **NoteOff を NoteOn より先**（Note-Off priority）。これは kind 内の局所規則で、他 kind の causal order を変えない。
4. 出力非減少・`offset<block_frames` 不変（入力が規約を満たす前提で merge は保持）。
5. scratch 不足 → `MergeOverflow`（呼び出し側が `EventScratchOverflow` 相当の report を発行）。

## 4. `ruviform-audio-core::midi` — input port trait

```rust
pub enum MidiPortKind { External, Virtual, Loopback, Unknown }
pub enum MidiSupport { Unknown, Unsupported, Supported }

#[derive(Copy, Clone, Debug)]
pub struct MidiInputCaps {
    pub timestamps: MidiSupport,
    pub sysex: MidiSupport,
    pub ump: MidiSupport,
    pub virtual_ports: MidiSupport,
}

pub trait MidiInputEnumerator: Send {
    fn input_ports(&self) -> Vec<Box<dyn MidiInputPort>>;
    fn default_input_port(&self) -> Option<Box<dyn MidiInputPort>>;
}

pub trait MidiInputPort: Send {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn kind(&self) -> MidiPortKind;
    fn capabilities(&self) -> MidiInputCaps;
    /// callback は backend input thread で RT contract（no alloc/block/panic/re-entry）。
    /// events は呼び出し中のみ有効な &[TimedInputEvent<MidiEvent>]。
    fn open(&self, cb: Box<dyn FnMut(&[TimedInputEvent<MidiEvent>]) + Send>)
        -> Result<Box<dyn MidiInputConnection>, MidiInputError>;
}

pub trait MidiInputConnection: Send {
    fn close(self: Box<Self>);
}

pub enum MidiInputError { /* DeviceError に準ずる normalized error */ }
```

`MockMidiEnumerator`/`MockMidiInput`: `push(TimedInputEvent<MidiEvent>)`（内部 queue へ、open 済 connection の callback 経由で drain）。port kind/caps は宣言可。`#[cfg(any(test, feature = "..."))]` または常時公開のいずれかは実装時に決める（PoC では bridge テストから使える必要がある → `midi::mock` module として公開、`#[doc(hidden)]` または feature で隔離）。

## 5. `ruviform-audio-plugin-bridge` — InputStage・merge・ledger 配線

```rust
// input.rs
pub const INPUT_RING_CAPACITY: usize = 1024;
pub const INPUT_DRAIN_MAX: usize = 64;

pub struct InputStage { /* rx: spsc::Receiver<TimedInputEvent<MidiEvent>>, anchors: SnapshotReader<ClockAnchor>, mapped: [..; INPUT_DRAIN_MAX], counters */ }

pub struct InputConfig { pub capacity: usize /* <= INPUT_RING_CAPACITY */ }
```

`node.rs`/`CallSink` への配線（`process` 内）:
1. `CTRL_DRAIN_MAX` で `NodeControl` drain（新 `SetClockAnchor` 適用）。
2. scheduler `run_callback` → span ごとに `build_call_event_list` → plan scratch。
3. `InputStage::drain(block_start, frames)` → mapped live scratch（anchor 不在/domain 不一致/負 delta は `InputUnmapped`/`InputBeforeAnchor`/`EpochMismatch` counter + defined drop、**silent drop しない**）。
4. transport 不連続検出（`last_transport_generation`/epoch/status 変化）→ `NoteLedger::release_all` を offset 0 scratch へ。
5. `merge_call_events(plan ++ release ++ live)` → merged scratch → `EventList` → `PluginEndpoint::process`。
6. `NoteLedger.feed(events)`（merged の Midi kind を feed）。
7. counter/report を `NodeStatus`/`NodeReport` へ flush。

`control.rs`: `NodeControl::SetClockAnchor{anchor: ClockAnchor}`（`#[non_exhaustive]` additive）。`attach.rs`: `AttachConfig::input: Option<InputConfig>`、`Attached::input_tx: Option<InputTx>`。

`status.rs`/`NodeStatus` counters: `input_received`/`input_mapped`/`input_queue_full`/`input_unmapped`/`input_before_anchor`/`input_epoch_mismatch`/`input_ledger_overflow`/`input_merge_overflow`/`input_released`（Counter 追加は additive）。`NodeReport`: `InputOverflow{count}`/`InputUnmapped{domain}`/`InputBeforeAnchor`/`InputLedgerOverflow`/`InputMergeOverflow`（`#[non_exhaustive]` additive、`EventScratchOverflow` 先例）。

## 6. Adapter — cap.rs・CLAP translation・VST3 honesty

`cap.rs` の `AdapterCapability` 追加 field（全 `MidiSupport` 三値）:

| field | L0 | L1 | CLAP | VST3 |
| --- | --- | --- | --- | --- |
| `note_events` | Supported | Declared | Supported | Unavailable |
| `channel_events` | Supported | Declared | Supported | Unavailable |
| `note_expression` | Unavailable | Unavailable | Declared（未接続） | Unavailable |
| `sysex` | Unavailable | Unavailable | Unavailable | Unavailable |

CLAP `push_events`（host.rs）: `match event.kind()` — `ParamValue`→既存 `ParamValueEvent`、`Midi`→`MidiKind` ごとに `NoteOnEvent`/`NoteOffEvent`（velocity f64、Pckn{port_index=0,channel,key,note_id=-1}）または `MidiEvent`（CC/Bend/Pressure/PC が MIDI1 7bit で表現可能なとき `data[3]`）。表現不能精度 → `midi_skipped` counter（`fill_result` へ統合）。non-ParamValue の `let-else` skip を `match` + count へ置換（**defined drop**、silently 消さない）。

VST3 `fill_param_queues`（host.rs）/`block.rs`: `EventKind::Midi` → counted `midi_skipped` + `NodeReport` 相当。`IEventList` 実装は分割 Issue（cap で Unavailable 宣言）。

## 7. Invariants / fault 分類

| invariant | 保証するもの | fault 検出 |
| --- | --- | --- |
| `EventList` wire 形 | 非減少 offset・`offset<block_frames`・0-frame flush | merge scratch 不足 → `InputMergeOverflow` |
| stamp 保持 | `TimedInputEvent` は native stamp を破壊しない | — |
| clock domain | domain/epoch 不一致は減算しない | `UnmappedDomain`/`EpochMismatch` counter+report |
| merge 安定性 | causal order 保持・plan-before-live・NoteOff priority | —（規約） |
| bounded | `INPUT_RING_CAPACITY`/`INPUT_DRAIN_MAX`/scratch | `InputQueueFull`/各 counter |
| note safety | ledger 追跡・不連続 release | `InputLedgerOverflow`（force-release 時） |
| adapter honesty | 未対応 kind/精度は defined drop | `midi_skipped` count |

## 8. Split Issue 提案（本 Issue では分割案のみ）

1. `ruviform-audio-midi` adapter crate — midir/CoreMIDI/ALSA/WinMM 比較→実装（audio-core port 実装、timestamp provenance の正直宣言）。
2. `PortFormat::TimedEvent` typed payload 化 + graph 配線（M3 後続）。
3. VST3 `IEventList`/`NoteOnEvent`/`LegacyMIDICCOutEvent` + note-expression。
4. CLAP `NoteExpressionEvent`/`Midi2Event`/SysEx・plugin→host output events。
5. L1 wire `EVENT_KIND_MIDI` verb + version negotiation + capability verb。
6. engine 層の録音 tap（`TimedInputEvent` を consumer へ投影する観測 API）。
7. `NoteLedger` controller chase 拡張（CC64/全 CC reset・preset restore 時の state re-send）。
8. SysEx/System message 語彙 + wire 化。
