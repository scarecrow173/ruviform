---
id: "DESIGN-034XBGBV0qexwf6VzGt77S"
type: "design"
status: "approved"
title: "Issue #628 設計 — deterministic core（VirtualClock/PCM fixture/MIDI trace/独立 Oracle/run metadata）"
created: "2026-11-14"
updated: "2026-11-14"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-628-deterministic-test-runtime-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-628-deterministic-test-runtime.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 設計 — deterministic core（VirtualClock/PCM fixture/MIDI trace/独立 Oracle/run metadata）

## Context

Issue #628 first slice。test-only crate `crates/ruviform-audio-test` を新設し、
Ruviform の公開契約を fake ではなく**実際の production 型**で駆動する
deterministic core を提供する。本 slice の範囲は coordinator 確定の
「deterministic core のみ」であり、scripted backend・fault injector・
production 配線・CI lane・consumer oracle は後続 slice。

利用する production 契約（調査済み）:

- `ruviform_rt::MasterClock` — atomic tick + rational timebase、
  `publish_tick`(Release)/`tick`(Acquire) の wait-free read モデル。
- `ruviform_media_core::transport_schedule` — `TransportScheduler<P>` /
  `TransportPlanBuilder<P>` / `TransportPlan<P>` / `ScheduledEvent<P>` /
  `SpanSink<P>` / `TransportSpan` / `SpanContext` / `EngineFrame` /
  `ClockEpochId` / `TransportCommand` / `BlockContract` / `ScheduleBudget` /
  `DeliveryPolicy` / `AppliedReceipt` / `ScheduleFault`。
  `run_callback` は RT-suitable（allocation-free・bounded）、
  `SpanSink::span` が span 毎の event window を受け取る capture 点。
- `ruviform_media_core::midi` — `MidiEvent`/`MidiKind`/`MidiChannel`/
  `MidiGroup`/`NoteKey`/`MidiController`/`MidiProgram`/`MidiValue`/
  `NoteLedger`（canonical scale-to-max 値、`Copy` value type）。
- `ruviform_media_core::time` — `TimeBase`/`MediaTime`/`MediaDuration`/
  `MediaRange`（exact integer rational、`i128`/`u128` 中間値、f64 なし）。
- `ruviform_audio_core::buffer` — `AudioBufferView`/`AudioBufferViewMut`
  （planar channel-major、`try_new` で検証済み借用 view）。
- `crates/ruviform-audio-device/tests/mock_input_backend.rs` —
  `assert_no_alloc` + `rt_` prefix RT contract test の先例。
- `crates/ruviform-ui-test`・`crates/ruviform-io-e2e` — test-only crate
  宣言・依存記録・`[lints] workspace = true` の先例。

重要な設計上の発見:

- repo 内に production の MIDI **output** port は存在しない
  （`MidiOutput`/`midi_output` grep = 0 件）。本 runtime は outgoing trace を
  `SpanSink` として capture する設計とし、実 port への配線は後続 slice。
- `TransportScheduler::run_callback` + `SpanSink` は「span 境界で
  half-open `[start,end)` に分割し、position window の event を sink へ渡す」
  本番配送経路であり、MIDI trace の capture 契約としてそのまま使える。
  独自 scheduler を書くと production 契約を迂回するため採用しない。
- Rust の `f32::sin`/`f64::sin` 等の超越関数は platform libm 依存で
  bit-exact を保証しない。deterministic core の PCM 生成は
  **libm 非依存の決定的算術のみ**で構成する（後述 §3）。

## Scope

### In Scope

- crate 骨格: `Cargo.toml`（path dep 3 件 + dev-dep `assert_no_alloc`）、
  `src/lib.rs`（crate-level doc で test-only を宣言）、`README.md`。
- `clock` module: `VirtualClockConfig` / `VirtualClock` /
  `VirtualClockDriver`。
- `pcm` module: `PcmPattern` / `PcmFixture` / `PcmSegment` /
  `PcmScenario` / `PcmBlock`（block 生成と `fill`）。
- `midi` module: `MidiCue` / `MidiScenario` / `MidiTraceConfig` /
  `run_midi_trace` / `MidiTrace` / `MidiTraceEntry`。
- `oracle` module: PCM 独立参照生成・比較、MIDI trace 期待列・不変条件、
  固定ベクトル。
- `metadata` module: `RunMetadata`（Case Schema v1 準拠の記録構造）。
- `tests/` 統合テスト + `rt_` prefix `assert_no_alloc` テスト。
- workspace 登録・docs chain・evidence/rf628。

### Out of Scope

- scripted backend / fault injector / production pipeline 配線 /
  CI lane / consumer oracle / 3 OS 実走 / Windows EXT-L3（spec 参照）。
- `TransportScheduler` 自体の改変（production 契約は変更しない）。
- MIDI clock（`MidiKind` に clock message は存在しない — channel voice
  vocabulary のみ）・SysEx・System message — production 型の範囲に従う。
- `NoteLedger` を使った chase/stuck-note 処理の harness 実装 — production
  型は存在するが、trace capture への結線は後続 slice。

## Components and Boundaries

### 1. `clock` — VirtualClock

`MasterClock` の atomic read モデルを拡張した deterministic virtual clock。
共有 read 側（`Arc<VirtualClock>` で RT thread へ渡せる `Sync` 型）と
single-writer の driver 側を分離する。

```rust
pub struct VirtualClockConfig {
    pub sample_rate: u32,        // 1/rate 秒/tick（MasterClock timebase = 1/rate）
    pub epoch: u64,              // clock epoch（ClockEpochId::new(epoch) へ写像可能）
    pub offset_ticks: i64,       // 決定的 offset（負値=pre-roll 許容、飽和 clamp）
    pub drift_ppm: i32,          // 決定的 linear drift（parts per million、負値可）
    pub jitter: JitterSpec,      // deterministic jitter
}

pub struct JitterSpec {
    pub seed: u64,
    pub period_frames: u64,      // 0 = jitter 無効
    pub amplitude_ticks: u32,    // period 内で定数の jitter 振幅
}

pub struct VirtualClock {
    clock: MasterClock,               // tick の atomic read/write（ruviform-rt 再利用）
    monotonic_nanos: AtomicU64,       // monotonic 時間（virtual elapsed 由来）
    config: VirtualClockConfig,
}

pub struct VirtualClockDriver {
    clock: Arc<VirtualClock>,
    elapsed_frames: u64,              // 仮想経過（pause 中は進まない）
    paused: bool,
}
```

決定的 tick 計算（`VirtualClockConfig::tick_at(elapsed_frames)`、
`const`/pure integer math、libm 不使用）:

```
tick = clamp_u64( offset_ticks
                + elapsed_frames
                + (elapsed_frames * drift_ppm) / 1_000_000
                + jitter_ticks(elapsed_frames) )

jitter_ticks(e) = 0                                   if period_frames == 0
                = map(hash64(seed, e / period_frames))
                  onto [-amplitude_ticks, +amplitude_ticks]  otherwise
```

- `hash64` は splitmix64 系の自前 u64 hash（pure integer、OS 非依存）。
  jitter は period 単位で定数 — callback 周期の jitter を模す。
- `monotonic_nanos_at(elapsed)` = `elapsed * 1e9 / sample_rate` を
  u128 中間値で計算（exact rational、f64 不使用）。
- `VirtualClock` read API（全て RT-safe、`MasterClock` 同様 wait-free）:
  `tick()` / `monotonic_nanos()` / `timebase()` / `epoch()` / `config()`。
- `VirtualClockDriver`（non-RT、single writer）:
  `advance_frames(n)`（pause 中は elapsed を進めない or 別途蓄積 —
  pause 中の advance は破棄せず `resume` 後に反映しない設計:
  `advance_frames` は `!paused` のときのみ elapsed を進める）、
  `pause()`/`resume()`/`is_paused()`、`sync()`（elapsed から
  `tick_at`/`monotonic_nanos_at` を計算して publish）、
  `seek_frames(n)`（time discontinuity — elapsed を直接設定）、
  `deadline_in(frames) -> u64`（現在 elapsed + frames の目標 tick を返す
  deadline helper。timeout 判定は `clock.tick() >= deadline` の比較で行う）。

`MasterClock` を内包する理由: atomic read/write・timebase 検証
（zero denominator 拒否）を production 実装で再利用し、test-only 側に
独自 atomic を増やさない。`epoch`/`offset`/`drift`/`jitter` は
`VirtualClockConfig` が保持し、driver が tick を計算して
`publish_tick` する — read 側は publish 済み tick を読むだけ。

### 2. `pcm` — Deterministic PCM fixtures

pattern は **spec**（fixture version で固定）であり、生成は決定的。

```rust
pub enum PcmPattern {
    Silence,
    Constant { value: f32 },               // DC offset fixture
    Impulse { period_frames: u32, amplitude: f32 },
    Ramp { amplitude: f32, period_frames: u32 },    // sawtooth -amp..+amp
    Square { amplitude: f32, period_frames: u32 },  // 50% duty
    Sine { freq_hz: f64, amplitude: f32 }, // poly近似（libm 不使用）
    Sweep { f0_hz: f64, f1_hz: f64, duration_frames: u64, amplitude: f32 },
    Noise { seed: u64, amplitude: f32 },   // per-frame hash（位置独立）
}
// 全て #[non_exhaustive]
//
// channel 依存（multichannel を実際に区別する決定的規則）:
// - Noise: stream = hash(seed ^ channel) で channel 毎に独立
// - Sine/Sweep: phase offset = channel/16 turn（ch<16 で位相が異なる）
// - その他: channel 間で同一波形（valid な multichannel data）

pub struct PcmFixture {
    pub sample_rate: u32,
    pub channels: u16,
    pub pattern: PcmPattern,
}
// 構築は `PcmFixture::new(sample_rate, channels, pattern) -> Result<Self>`
// で行い、sample_rate>0・channels>0・pattern パラメータ（freq 範囲・
// period>0 等）を検証する。field は pub（spec として公開）。

pub struct PcmSegment {                       // format/channel/buffer 変更単位
    pub frame_start: u64,                    // stream 先頭からの絶対 frame
    pub frame_len: u64,
    pub fixture: PcmFixture,
}

pub struct PcmScenario { pub segments: Vec<PcmSegment> }
// 欠落 frame = segment 列の frame range に gap を置く（連続性は consumer が検証）
```

生成 API:

- `PcmFixture::fill(&self, start_frame: u64, frames: u32, out: &mut [f32]) -> Result<(), Error>`
  — planar `[ch][frame]` 前提、`out.len() == channels * frames` を検証。
  **alloc-free・RT-suitable**（`rt_` テスト対象）。
- `PcmFixture::block(&self, start_frame: u64, frames: u32) -> PcmBlock`
  — owned `Vec<f32>` 生成（non-RT）。
- `PcmScenario::segments()` — format/channel/buffer 変更を跨ぐ列挙。
  `PcmBlock { start_frame, frames, channels, sample_rate, data: Vec<f32> }`。

決定性の構成（全て **absolute frame の純関数** — `fill(start_frame)` を
任意位置から呼んでも同一 frame に同一値を返す位置独立性を持つ）:

- **PRNG/hash**: splitmix64 系の自前 u64 hash（pure integer、OS 非依存）。
  noise は `hash64(seed, channel, absolute_frame)` の **stateless per-frame
  hash** — sequential state を持たず任意 start_frame から再現可能。
  u64 → f32 は上位 bit の exact 変換で写像。
- **phase（Sine/Ramp/Square）**: u64 fixed-point。`step32 =
  round(freq_hz / sample_rate * 2^32)`（f64 で 1 回だけ計算 — f64 の
  `+ - * /` は IEEE-754 exact）。`phase32(frame) = (frame * step32) mod
  2^32`（u64 wraparound は mod 2^32 の準同型で正確）。`freq_hz >= 0`・
  `freq_hz < sample_rate` を `PcmFixture::new` で検証。
- **sine/sweep**: f64 の固定多項式近似（libm の `sin`/`cos`/`powf` は
  不使用）。位相を `[0,1)` → `[-π,π]` に写像し、Taylor 展開
  （`x - x³/3! + x⁵/5! - … ± x¹⁷/17!`）を f64 で評価。sweep は
  `freq(t)` 線形補間の位相積分を f64 closed-form で frame 毎に計算
  （duration 超過後は f1 で継続）。f64→f32 変換は IEEE
  round-to-nearest で deterministic。
- **Impulse/Ramp/Square/Constant/Silence**: 有理数・整数のみ、完全 exact。
- **drift/jitter**: §1 の pure integer math。

`f32::sin` 等を使わない理由と、oracle 側が libm `f64::sin` を独立実装として
許容する理由（比較は tolerance）を design notes に記録する — generator
自身の出力は libm 非依存で bit-deterministic、oracle は「異なる算法経路」
で独立性を担保する。

### 3. `midi` — Deterministic MIDI scenario/trace

```rust
pub struct MidiCue {
    pub position: MediaTime,        // canonical absolute position（frame base ticks）
    pub source_sequence: u64,       // causal ordering key
    pub event: MidiEvent,           // production MIDI 型をそのまま payload に
}

pub struct MidiScenario {
    pub frame_base: TimeBase,       // 1 tick = 1 engine frame（PoC 慣行）
    pub cues: Vec<MidiCue>,
}

pub struct MidiTraceConfig {
    pub block_frames: u32,          // 1 callback の frames
    pub contract: BlockContract,    // BlockContract::new(min, max, in_block)
    pub policy: DeliveryPolicy,     // Exact / Degraded
    pub budget: ScheduleBudget,
    pub commands: Vec<(u64, TransportCommand)>,  // due frame → command
}

pub fn run_midi_trace(
    scenario: &MidiScenario,
    config: &MidiTraceConfig,
) -> Result<MidiTrace, Error>
```

流れ（全て production 契約経由）:

1. `scenario.compile(generation: PlanGeneration) -> Result<Arc<TransportPlan<MidiEvent>>>`
   — `TransportPlanBuilder::new(frame_base, generation)` に cue を
   `builder.event(position, source_sequence, event)` で投入し `build()`。
   position が frame base に exact 変換できない場合は `Error::TimeOverflow`
   （production の reject 経路をそのまま使う）。
2. `TransportScheduler::<MidiEvent>::new(frame_base, contract, policy, budget)`。
3. driver loop: `blocks = total_frames / block_frames` 回、各 block で
   - due ∈ `[block_start, block_end)` の `commands` を `scheduler.request(cmd, EngineFrame::new(epoch, due))` で投入
     （`install_plan` で plan を due=block 0 に install、`Play` を due=0 に request）
   - `scheduler.run_callback(EngineFrame::new(epoch, block_start), block_frames, &mut trace_sink)`
4. `TraceSink`（`SpanSink<MidiEvent>` 実装）が span 毎に
   `event_engine_frame(span, event)` で deliverable 判定し、
   `MidiTraceEntry { frame: EngineFrame, position_ticks: i64, occurrence: LoopOccurrence,
   source_sequence: u64, event: MidiEvent }` を収集。
   `deliverable == false` の span は sink が event を deliver しない
   （contract 上 sink は deliverable span のみ処理する責務 — 本実装は
   `span.deliverable` を記録し、false なら event を採用しない）。
5. 結果 `MidiTrace { entries, receipts, faults, fault_overflow, blocks_run }`。

MIDI **output port が存在しない**ため、trace は `SpanSink` capture が
outgoing の記録点となる — design notes に記録（spec Out of Scope 参照）。

`commands` は `TransportCommand`（Play/Pause/Stop/Seek/SetLoop/ClearLoop）
をそのまま渡せる — seek/loop/pause の scenario を production scheduler の
receipt/fault ledger で観測できる。chase/queue overflow/panic は
production port 不在のため本 slice 対象外（NoteLedger は存在するが
harness 結線は後続）。

### 4. `oracle` — 独立 Oracle

期待値を被試験コードから生成しないため、oracle は **別実装**を持つ。

```rust
// PCM: fixture spec から独立に sample を生成（libm 許容・tolerance 比較）
pub fn pcm_reference_block(
    sample_rate: u32, channels: u16, pattern: &PcmPattern,
    start_frame: u64, frames: u32,
) -> Vec<f32>

// 比較: Exact（bit 比較、to_bits）/ AbsEps（f32 差の最大値）
pub enum PcmCompare { Exact, AbsEpsilon(f32) }
pub fn pcm_max_abs_diff(a: &[f32], b: &[f32]) -> f32
pub fn pcm_bits_eq(a: &[f32], b: &[f32]) -> bool

// MIDI: play-through（command なし or Play@0 のみ）scenario の期待 trace を
// cue の独立 sort + position→frame 写像で生成
pub fn midi_expected_playthrough(
    frame_base: TimeBase, cues: &[MidiCue],
) -> Vec<MidiTraceEntry>
// 不変条件: entries は (frame, source_sequence) で非減少、frame は
// half-open span で重複しない、各 entry の event は plan に存在
pub fn midi_trace_invariants_hold(trace: &MidiTrace) -> bool
```

oracle の独立性:

- PCM: oracle は fixture spec を**独立コードで再実装**する
  （generator の関数を呼ばない）。sine/sweep は libm `f64::sin` +
  別位相進行で `AbsEpsilon`（例: 1e-5）比較、noise は同一 spec の
  独立実装で `Exact`（bit 比較）、silence/impulse/ramp/square/constant
  は `Exact` で検証。
- MIDI: oracle は cue を `(position_ticks, source_sequence)` で独自 sort
  し、play-from-0 の単純経路で `frame == position_ticks` の期待 entry を
  生成。`TransportPlanBuilder`/`TransportScheduler`/`SpanSink` を呼ばない。
- 固定ベクトル: `tests/` に小さい既知入力の literal 期待値
  （silence+impulse の byte 列、2 cue の MIDI trace）を埋め込み、
  oracle 実装自身の正しさを検証。

### 5. `metadata` — Run metadata

#627 Case Schema v1 の語彙に従う記録構造（deterministic backend slice
で実際に埋まる項目のみ）:

```rust
pub struct RunMetadata {
    pub schema: &'static str,            // "rf-vtest-run-metadata.v1"
    pub environment_kind: EnvironmentKind, // DeterministicBackend
    pub case_id: String,
    pub feature: String,
    pub level: VerificationLevel,        // OsL1
    pub os: &'static str,                // std::env::consts::OS
    pub arch: &'static str,              // std::env::consts::ARCH
    pub repo_sha: String,                // 環境変数 or 引数で注入
    pub ruviform_resolved_sha: String,   // Ruviform 自身では sha と同一
    pub scenario_seed: u64,
    pub fixture_version: u32,
    pub clock: VirtualClockConfig,
    pub oracle_id: String,
    pub run_id: String,
}
```

`EnvironmentKind`/`VerificationLevel`/`Requirement`/`Result` は
kebab-case 文字列または enum で #627 契約の値集合を写像
（`deterministic-backend`/`os-virtual-endpoint`/`test-plugin`/
`real-window`/`physical-device`、`OS-L1`/`GUI-L2`/`EXT-L3`、
`Required`/`Deferred`/`NotApplicable`、`Pass`/`Fail`/`NotRun`/`Unsupported`）。
本 slice は `DeterministicBackend` + `OsL1` の記録のみを生成し、
Pass/Fail 判定は consumer/CI 側の責務。

## Data and Control Flow

- PCM: `PcmScenario` → `PcmSegment`/`PcmFixture` → `fill`/`block` →
  `&mut [f32]` or `Vec<f32>` → consumer（device callback / engine / oracle）。
- MIDI: `MidiScenario` → `compile` → `Arc<TransportPlan<MidiEvent>>` →
  `TransportScheduler` + `run_callback` → `TraceSink` → `MidiTrace` →
  oracle 比較。
- Clock: `VirtualClockDriver::advance_frames` → `tick_at`/`monotonic_nanos_at`
  → `publish_tick`/`publish_nanos` → RT reader `tick()`/`monotonic_nanos()`。
- Metadata: scenario 実行時に `RunMetadata` を構築し、trace/fixture と
  同じ seed/version/clock で保存（JSON 化は consumer 側 or 後続 —
  本 slice は構造体 + `to_json` 的な最小 serialize を serde なしで
  提供するか検討。依存追加を避けるため `Display`/field 公開で十分とする）。

## Non-Functional Constraints

- **RT contract**: `clock.tick()`/`monotonic_nanos()`、`PcmFixture::fill`、
  `run_callback`+`TraceSink` は alloc-free・non-blocking・I/O なし。
  `assert_no_alloc` の `rt_` テストで検証。
- **決定性**: libm 非使用・HashMap 非使用・f64 の IEEE 四則のみ・
  pure integer PRNG/phase。`Vec`/`String` は non-RT 側のみ。
- **依存方向**: `ruviform-audio-test` は利用側層の test-shared infra。
  production crate（audio-core/media-core/rt/audio-device/engine/plugin）は
  本クレートに依存しない。path dep は `audio-core`/`media-core`/`rt` のみ、
  `assert_no_alloc` は dev-dep。
- **公開 enum**: 全て `#[non_exhaustive]`。
- **unsafe**: 不要（workspace lint で実質 deny）。

## Risks and Trade-offs

| Risk | Mitigation |
| --- | --- |
| f64 poly sine と libm sin の差で byte 比較が落ちる | generator は poly 固定・oracle は libm+tolerance。integer-domain pattern は Exact 比較。transcendental の 3 OS byte-exact は本 slice で主張しない（後続実走証拠） |
| `TransportScheduler` の `run_callback` が複雑で trace が production 契約を正しく反映しない | production の `event_engine_frame`/`span_event_window` をそのまま使い、sink は deliverable 判定のみ。oracle は独立 sort で play-through を検証 |
| VirtualClock の jitter/ drift が複雑化 | period 単位の定数 jitter + linear ppm drift に限定。複雑な jitter model は後続 |
| run metadata が Case Schema v1 の全項目を埋められない | deterministic backend で実際に埋まる項目のみ。`endpoint_id`/`plugin_*`/`sdk_version` 等は後続 slice で拡張 |
| `assert_no_alloc` が `Arc` clone 等を検出 | `run_callback` 内で `Arc::clone` は plan install 時のみ（non-RT）。RT 経路では `Arc` refcount bump を避ける設計を確認（`plan()` accessor は `&Arc` 返却で clone 不要） |

## Deferred Design Concerns

- scripted backend / fault injector / production 配線 / CI lane /
  consumer oracle / 3 OS 実走 / Windows EXT-L3 — 後続 slice・別 Issue。
- MIDI clock message・SysEx・chase/stuck-note harness — production
  vocabulary/port の範囲確定後。
- run metadata の JSON serialize — consumer 側 or 後続で serde 依存を
  検討（本 slice は構造体のみ）。
- `environment_kind` の値集合拡張 — #627 契約の改訂として扱う。

## Verification Notes

- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が
  `ruviform-audio-test` で Pass。
- [ ] `rt_` prefix テストが `assert_no_alloc` で clock-read /
  generator-fill の alloc-free を証明し、`test-rt` で実際に収集・実行
  される（テスト名で確認）。
- [ ] PCM: silence/impulse/ramp/square/noise/constant は oracle Exact
  比較、sine/sweep は oracle libm+tolerance で独立検証。
- [ ] MIDI: play-through scenario の trace が oracle 期待列と一致、
  seek/loop scenario で receipt/fault ledger が production 契約通りに
  記録される。
- [ ] run metadata が `environment_kind=deterministic-backend`・
  `level=OS-L1`・seed/fixture_version/clock を保持。
