---
id: "DESIGN-RF628sBscripted0000002"
type: "design"
status: "approved"
title: "Issue #628 設計 — scripted port-trait backend（device 境界 fault/clock/PCM/MIDI 結線）"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-628-scripted-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-scripted-backend.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-628-scripted-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 設計 — scripted port-trait backend

## Context

Issue #628 2nd slice。first slice の deterministic core
（`VirtualClock`/`PcmScenario`/`run_midi_trace`/oracle/`RunMetadata`）を
`ruviform-audio-core::device` の port 契約へ結線する scripted backend を
`crates/ruviform-audio-test` に追加する。

利用する production 契約（調査済み）:

- `ruviform_audio_core::device` — `DeviceEnumerator`（object-safe、
  Send/Sync 非要求）/ `Device: Send` / `OutputStream: Send` /
  `InputStream: Send`。callback は `OutputDataCallback`/
  `InputDataCallback`（`for<'a> FnMut(view, info) + Send + 'static`）、
  error は `InputErrorCallback`/`OutputErrorCallback`
  （`FnMut(DeviceError) + Send + 'static`）。唯一の error 型は
  `DeviceError`（`Copy`、`#[non_exhaustive]`）。close は `stop()`+Drop
  のみ（明示 close API なし）。
- timestamp 型: `OutputTimestamp{callback, playback}` /
  `InputTimestamp{callback, capture: Option<MediaTimestamp>}` /
  `InputCallbackInfo{timestamp, clock_domain, frame_position}` /
  `OutputCallbackInfo{timestamp}` / `StreamLatency{frames, coverage,
  provenance}` / `LatencyCoverage`。全て `DEVICE_CLOCK_TIMEBASE`
  （nanos `TimeBase(1,1e9)`）の `MediaTime`。
- `AudioBufferView`/`AudioBufferViewMut::try_new(data, channels, frames)`
  — planar channel-major、alloc-free の借用 view。
- `ruviform_audio_core::midi_input::mock::MockBackend`/`MockPort` —
  `TimedMidiInput` 投入・`inject_dropped`/`inject_unsupported`/
  `fail_next_open` の先例。
- first slice: `VirtualClock`/`VirtualClockDriver`（atomic wait-free
  read + single-writer publish）、`PcmScenario`/`PcmFixture::
  fill_channel`（alloc-free）、`MidiScenario`/`MidiCue`。
- 先例 test: `mock_backend.rs`/`mock_input_backend.rs`（manual pump、
  `Arc<Mutex<…>>` 共有状態、`assert_no_alloc`+`rt_`）。

## Scope

### In Scope

- `src/backend.rs`（module root・公開 API）+ `src/backend/` 配下の
  実装分割（script 型・stream 実装・MIDI feed）。
- enumerate/open/describe/stream 制御面 fault + callback index/frame
  指定の stream fault。
- `tests/backend.rs`/`tests/backend_faults.rs`/`tests/rt_contract.rs`
  拡張。crate README・`src/lib.rs` doc 更新。docs chain・evidence。

### Out of Scope

spec Out of Scope と同じ（pipeline 配線・consumer oracle・CI lane・
3 OS 実走・generic FaultInjector・rate change mid-stream）。

## Components and Boundaries

### 1. Script 型（`backend::script`）

```rust
pub struct DeviceSpec {
    pub name: String,
    pub id: Option<String>,                    // None → id() = Ok(None)
    pub clock: Option<Arc<VirtualClock>>,      // None → backend 既定 clock
    pub output: Option<OutputSpec>,
    pub input: Option<InputSpec>,
}

pub struct OutputSpec {
    pub default_format: AudioFormat,           // F32 必須（add_device で検証）
    pub supported_formats: Vec<SupportedOutputFormat>, // 非空・F32 のみ
    pub stream: StreamScript,
    pub record_output: bool,                   // callback 後 scratch を capture
}

pub struct InputSpec {
    pub default_format: AudioFormat,
    pub supported_formats: Vec<SupportedInputFormat>,
    pub pcm: PcmScenario,                      // capture 源
    pub stream: StreamScript,
    pub capture_reporting: bool,               // false → capture: None 常時
}

pub struct StreamScript {
    pub default_frames: u32,                   // config Default 時の frames/callback
    pub frames_cycle: Vec<u32>,                // 空=固定。非空=callback 毎に巡回（可変 buffer）
    pub latency: Option<StreamLatency>,        // *_latency() の報告値
    pub clock_domain: Option<ClockDomainId>,   // None → ClockDomainId::new(clock.epoch())
    pub faults: Vec<StreamFault>,              // callback index 指定の注入
    pub open_faults: OpenFaults,               // open 時エラー
    pub start_faults: VecDeque<DeviceError>,   // start() 毎に先頭を消費
    pub stop_faults: VecDeque<DeviceError>,
}

pub struct OpenFaults {
    pub persistent: Option<DeviceError>,       // 設定中は全 open が失敗
    pub once: VecDeque<DeviceError>,           // open 毎に 1 つ消費（一過性→成功）
}

#[non_exhaustive]
pub enum StreamFault {
    Error { at_callback: u64, error: DeviceError },   // data callback 後に error callback へ
    FramePosition { at_callback: u64, position: u64 },// その callback で内部 position を設定（持続）
    CaptureNone { at_callback: u64 },                 // input のみ: capture: None に
    Frames { at_callback: u64, frames: u32 },         // その callback の frames を差替え
    Invalidate { at_callback: u64 },                  // StreamInvalidated 通知+stream 死亡
}

#[non_exhaustive]
pub enum StreamDirection { Output, Input }
```

`add_device` は軽量検証を行う: `default_format.sample_format == F32`、
`supported_formats` 非空かつ全 entry が F32・`channels>0`・`min<=max`・
`min>=1`、`frames_cycle`/`default_frames` が `>0`。違反は
`Error::InvalidValue`（crate 既存の `ruviform_media_core::Error`）。

### 2. Backend/Device/Handle（`backend::device`）

```rust
pub struct ScriptedBackend { /* DeviceEnumerator impl */ }
impl ScriptedBackend {
    pub fn new(clock: Arc<VirtualClock>) -> Self;
    pub fn add_device(&mut self, spec: DeviceSpec) -> Result<ScriptedDeviceHandle, Error>;
    pub fn set_enumerate_error(&mut self, error: Option<DeviceError>);
    pub fn set_default_output(&mut self, index: Option<usize>);
    pub fn set_default_input(&mut self, index: Option<usize>);
    pub fn device_count(&self) -> usize;
}
```

- `enumerate_error` 設定中は 4 メソッド全て `Err`（permission denial
  等の列挙失敗を `Backend("permission_denied")`/`DeviceBusy` で正直に
  写像）。
- 列挙は登録順・`vanish` 中の device を除外。default は明示 index →
  無ければ direction 対応の先頭 present device → 無ければ `Ok(None)`。

```rust
#[derive(Clone)]
pub struct ScriptedDeviceHandle { /* Arc<Mutex<DeviceShared>> */ }
impl ScriptedDeviceHandle {
    pub fn vanish(&self);            // device 消失（open→DeviceNotFound、列挙から除外）
    pub fn reappear(&self);
    pub fn is_present(&self) -> bool;
    pub fn fail_next_open_output(&self, error: DeviceError);  // once へ push
    pub fn fail_next_open_input(&self, error: DeviceError);
    pub fn set_open_output_error(&self, error: Option<DeviceError>); // persistent
    pub fn set_open_input_error(&self, error: Option<DeviceError>);
    pub fn inject_output_fault(&self, fault: StreamFault);
    pub fn inject_input_fault(&self, fault: StreamFault);
    pub fn is_output_open(&self) -> bool;
    pub fn is_input_open(&self) -> bool;
    pub fn pump_output_next(&self) -> DeviceResult<PumpReport>;
    pub fn pump_input_next(&self) -> DeviceResult<PumpReport>;
    pub fn captured_output(&self) -> Vec<PcmBlock>;   // record_output の収集（clone）
    pub fn clear_output_capture(&self);
}
```

`DeviceShared` は `Arc<Mutex<…>>` で spec・`present: Arc<AtomicBool>`・
open stream slot（`Option<Arc<StreamSlot>>`、direction 毎に 1 つ）を保持。
`StreamSlot { core: Mutex<StreamCore>, dead: AtomicBool }` — invalidate/
drop で `dead` を立て、device は direction を再利用可能にする
（reconnect）。2 つ目の同 direction open は `DeviceBusy`。

### 3. Stream 実装（`backend::stream`）

`ScriptedOutputStream`/`ScriptedInputStream` は `OutputStream`/`InputStream`
を実装し、`Arc<StreamSlot>` を共有。RT-suitable な `format()`/
`clock_domain()` は Mutex 外の stored value を返す（lock 不要）。

open 時の検証順序（両 direction 共通）:

1. device `!present` → `DeviceNotFound`
2. direction spec `None` → `Unsupported`
3. `open_faults.persistent` → その error、空なら `once.pop_front()`
4. 同 direction slot が `!dead` で存在 → `DeviceBusy`
5. `config.format.sample_format != F32` → `UnsupportedConfig`
   （port の callback view は f32 のみ — integer/F64 要求は正直に拒否）
6. supported space に適合しない（channels/rate/buffer range 外）→
   `UnsupportedConfig`
7. `BufferSize::Fixed(0)` または `usize→u32` overflow → `UnsupportedConfig`

`default_frames` 解決: `config.buffer_size == Default` →
`script.default_frames`、`Fixed(n)` → `n`（range 検証済み）。
`frames_cycle` 非空なら callback 毎に巡回値を使う（可変 frame count —
`BufferSize::Default` 相当の backend 判断として正直に報告）。

pump（`pump_*_next`、test driver が手動で呼ぶ non-RT 経路）:

1. device `!present` → 初回のみ `StreamInvalidated` を error callback
   に通知し `dead`/`invalidated` 化、`Err(StreamInvalidated)`。
2. `!started` → `Err(Backend("not_started"))`。
3. `invalidated` → `Err(StreamInvalidated)`。
4. 当該 `callback_index` に一致する `StreamFault` を適用
   （`Frames`→frames 差替え、`FramePosition`→内部 position 設定、
   `CaptureNone`→capture 無効化）。
5. scratch（open 時に `channels * max_frames` で確保、縮小はしない）
   の当該領域を zero 化 → input は scenario から fill、output は
   zero のまま callback へ。
6. timestamp 構築: `callback = MediaTime(clock.monotonic_nanos(),
   DEVICE_CLOCK_TIMEBASE)`。input `capture = capture_reporting && !fault
   ? Some(MediaTimestamp{ callback - latency_nanos, Derived }) : None`、
   `frame_position = position`。output `playback = latency.map(
   callback + latency_nanos)`。`latency_nanos = frames * 1e9 /
   sample_rate`（u128 整数 math）。
7. `data_callback(view, info)` を呼ぶ。callback 中は consumer の
   RT 契約（alloc/lock/wait/IO 禁止）が適用される — scripted backend
   側は view 構築・timestamp 構築・bounded memset のみで alloc しない。
8. output かつ `record_output` → scratch を `PcmBlock` として
   `captured` に追加（non-RT、alloc 許容）。
9. `Error`/`Invalidate` fault を error callback へ通知（data callback
   の後）。`Invalidate` は通知後に stream を死亡させる。
10. input で当該 window に scenario 未カバー frame（gap）があれば
    `DeviceError::Xrun` を error callback へ通知（loss の正直な観測）。
11. `callback_index += 1`、`position += frames`。`PumpReport` を返す。

input の PCM 再生: window `[position, position+frames)` と重なる各
`PcmSegment` について、channel 毎に `fixture.fill_channel(ch,
abs_frame, dst_subslice)` で書き込む。segment fixture の
`sample_rate`/`channels` は open 時に negotiated format と全 segment
で一致することを検証（不一致 → `UnsupportedConfig`）。format/channel
変更は stream の close + 別 config での再 open で表現する（port 契約に
mid-stream format change は存在しない — rate change は
`ClockDiscontinuity` + re-open で代替）。

`now()` = `MediaTime(clock.monotonic_nanos(), DEVICE_CLOCK_TIMEBASE)`。
`clock_domain()`/`InputCallbackInfo.clock_domain` は script 値
（`None` → `ClockDomainId::new(clock.epoch())`）。`same_clock_domain`
での比較が意味を持つよう、同一 clock を共有する device は同一 domain
を報告する。

### 4. PumpReport

```rust
#[derive(Copy, Clone, Debug)]
pub struct PumpReport {
    pub direction: StreamDirection,
    pub callback_index: u64,
    pub frames: u32,
    pub position: u64,                    // frame 0 の stream 位置（fault 適用後）
    pub reported_frame_position: Option<u64>, // input のみ
    pub clock_domain: ClockDomainId,
    pub callback_nanos: i64,
    pub capture_nanos: Option<i64>,       // input のみ
    pub playback_nanos: Option<i64>,      // output のみ
    pub gap_frames: u64,                  // input: 未カバー frame 数
    pub errors_fired: u32,
    pub invalidated: bool,
}
```

`Copy` value type — alloc なし。error の実体は test 登録の error
callback が観測する（backend 独自の error log は持たない）。

### 5. MIDI 薄 adapter（`backend::midi`）

```rust
pub struct MidiFeed {
    events: Vec<TimedMidiInput>,   // (position_ticks, source_sequence) sort 済み
    cursor: usize,
    domain: ClockDomainId,
    skipped: u64,                  // 変換不能 cue 数
}
impl MidiFeed {
    pub fn new(scenario: &MidiScenario, clock_domain: ClockDomainId) -> Self;
    pub fn skipped(&self) -> u64;
    /// stamp.ticks <= upto_frame の event を順に out へ drain（戻り値=件数）。
    pub fn drain_due(&mut self, upto_frame: u64, out: &mut Vec<TimedMidiInput>) -> usize;
    /// MockPort へ直接 pump する便宜メソッド。
    pub fn pump_into_port(&mut self, port: &MockPort, upto_frame: u64) -> usize;
}
```

- `stamp.ticks` = cue position を `frame_base` へ `convert_exact` した
  tick 数（`u64`、負値・変換不能は `skipped` へ計上して emit しない）。
  `stamp.base = frame_base`、`stamp.domain = clock_domain`、
  `provenance = Carried`（scenario が時刻を宣言する fixture の事実）。
- test driver は `VirtualClockDriver::advance_frames(block)` の後
  `driver.elapsed_frames()` を `upto_frame` として pump する —
  `frame_base = TimeBase::sample_rate(rate)` なら engine frame と一致。
- `MockPort::inject_dropped`/`inject_unsupported`/`fail_next_open` は
  そのまま利用（本 adapter は queue を持たない薄い変換層）。

### 6. RT 契約と正直な制限

- **alloc-free 証明対象**: pump の callback 発火区間 — scratch 再利用・
  `try_new` view・timestamp 構築・`data_callback`/`error_callback`
  起動・counter 更新。`record_output` capture・`PumpReport` 返却・
  `Vec`/`String` を伴う API は non-RT（test driver 側）。
- **lock について**: scripted backend の pump は test driver スレッドで
  動く non-RT 経路であり、内部状態は `Mutex` で保護する。callback 発火中も
  backend 側 lock を保持する — consumer の RT 契約（callback 内で
  alloc/lock/wait/IO 禁止）は consumer 側コードに適用される。本 backend
  は実 RT thread を持たないため「backend が RT thread で lock しない」
  ことは主張しない。`rt_` テストが証明するのは発火経路の
  **allocation-free** である（`assert_no_alloc` の計測範囲）。
- **thread 不使用**: callback は manual pump のみ。`start()` は
  delivery を「開始済み」にするだけで firing は行わない
  （先例 test の start()-firing 型ではなく、pump 型を採用 —
  input/output の交互駆動・fault 挿入位置を制御できるようにするため）。
- **rate change mid-stream は不可**: port 契約上 stream format は
  open 時固定。`ClockDiscontinuity` error + stream 死亡 + 別 config
  での再 open が代替経路（`Invalidate` fault + `reappear`/`open` で
  試験可能）。
- **permission denial の honest mapping**: OS permission は
  `Backend("permission_denied")`（enumerate/open）または `DeviceBusy`
  で表現する — 専用 variant は port に存在しないため捏造しない。
- **queue-full**: audio 側は `Backend("queue_full")`/`Xrun` の scripted
  error、MIDI 側は `MockPort::inject_dropped`/`inject_unsupported` の
  `MidiReadStats` で観測する。
- **実 OS path の証明ではない**: 本 backend の Pass は
  `environment_kind = deterministic-backend` の OS-L1 証拠。

## Data and Control Flow

- PCM input: `PcmScenario` → `InputSpec` → `InputCore` → pump が
  window 毎に `fill_channel` → `AudioBufferView` → `InputDataCallback`。
- PCM output: consumer `OutputDataCallback` が `AudioBufferViewMut`
  に書込 → pump が scratch を `PcmBlock` capture → `captured_output()`。
- Clock: test driver が `VirtualClockDriver::advance_frames` →
  `clock.monotonic_nanos()` publish → pump/`now()` が `MediaTime`
  (DEVICE_CLOCK_TIMEBASE) として読む。
- MIDI: `MidiScenario` → `MidiFeed` → `MockPort` → engine の
  `MidiInput::read`（production port 経路）。

## Non-Functional Constraints

- 依存追加なし（`ruviform-audio-core`/`ruviform-media-core`/
  `ruviform-rt` path dep + dev-dep `assert_no_alloc` のみ）。
- `unsafe` 不使用。公開 enum 全て `#[non_exhaustive]`。
- crate 境界維持: production crate は本クレートに依存しない。
- `Send`: `Device`/stream/handle/callback は `Send` を満たす
  （`Arc<Mutex<…>>` + `Send` callback）。

## Risks and Trade-offs

| Risk | Mitigation |
| --- | --- |
| `Mutex` を callback 発火中に保持することへの誤解 | design/README に「pump は non-RT test driver 経路」と明記。RT 証明は alloc-free のみ |
| scenario format と open format の不一致 | open 時に全 segment 検証 → `UnsupportedConfig`。format 変更は再 open で表現 |
| `ClockDomainId::new(epoch)` が well-known id（1/2）と衝突 | `clock_domain` を `Option` で script 可能にし、衝突時は明示値を推奨（doc 記載） |
| gap の自動 `Xrun` が過剰/不足 | window 内の未カバー frame >0 で 1 回通知。gap を fault にしない明示選択肢は設けない（欠落は常に loss） |
| `PumpReport` が direction 固有 field を持つ | `Option` field で方向別に `None`/`Some` — `Copy` を維持 |

## Deferred Design Concerns

- pipeline 配線・consumer oracle・CI lane・3 OS 実走・Windows EXT-L3 —
  後続 slice / 別 Issue。
- variable frame count の更に複雑な schedule（非周期的 jitter 等）—
  `frames_cycle` の巡回で十分な範囲に留める。
- 多重 stream（同 direction 複数同時 open）— port 利用形態上稀なため
  `DeviceBusy` で正直に拒否。

## Verification Notes

- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が
  `ruviform-audio-test` で Pass。
- [ ] `rt_` 新規テストが `assert_no_alloc` で callback 発火経路の
  alloc-free を証明し、`test-rt` で実際に収集・実行される。
- [ ] 正負テストで Fault と Expected/Observed が一致。
