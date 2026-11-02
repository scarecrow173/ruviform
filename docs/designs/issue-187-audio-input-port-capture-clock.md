---
id: "DESIGN-034Qx35fKutOnHPc9IRWC7"
type: "design"
status: "approved"
title: "Issue #187 Audio Input Port: input/duplex contract, capture timestamp/clock-domain/latency, RT-safe handoff PoC"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/synvelis/issues/12"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
    - "docs/adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md"
  implemented-by: []
  depends-on:
    - "audio-device-boundary-and-poc.md"
    - "../adr/select-audio-device-backend.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
    - "issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
    - "docs/adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md"
    - "../discovery/issue-194-m0-baseline-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/59"
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/185"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/187"
  deferred-by: []
---

# Issue #187 Audio Input Port: input/duplex contract, capture timestamp/clock-domain/latency, RT-safe handoff PoC

## Context

既存 `ruviform_audio_core::device` の port は `DeviceEnumerator`(output 列挙) → `Device`(describe+open) → `OutputStream`(callback 駆動) の 3-trait 形状で、入力用の契約を持たない。Issue #187 は汎用 Audio Input/Duplex・Capture Clock/Latency 契約と headless PoC を要求する。確定済みの前提:

- port の形状・timestamp/latency 語彙・duplex 表現は [ADR](../adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md) で決定済み（`Device` 拡張・独立 stream + `clock_domain` 相関・`MediaTime`/`ClockDomainId`/`MediaTimestamp` 再利用・`StreamLatency` 報告）。
- stamp→frame 変換は `ruviform_media_core::timed_input`（`ClockDomainId`/`InputStamp`/`ClockAnchor`/`map_stamp_to_frame`）が唯一権威。
- RT 契約は output と同規則（realtime ADR: callback 内 alloc 0・非ブロッキング・I/O・無制限待機・panic・制御 API の再入を禁止）。
- `AudioBufferView<'a, f32>`（read view）には `try_new` がなく、adapter が capture scratch を view 化するために追加する。

## Scope

### In Scope

- `ruviform-audio-core` の `device` モジュールへの入力契約追加と `buffer.rs` の `AudioBufferView::try_new` 追加。
- `ruviform-audio-device`（cpal）の入力側非 RT 表面: input 列挙・format query mapping・`open_input_stream` → `Err(Unsupported)`。
- `crates/ruviform-audio-device/tests/mock_input_backend.rs` — deterministic capture→bounded SPSC handoff の RT 契約 PoC。
- 設計の文書化: InputStream vs DuplexStream 比較（ADR で確定・本書に契約として転記）、timestamp 意味論、latency 定義、fallback/rejection 規則、RT 契約。

### Out of Scope

- cpal `build_input_stream` plumbing・interleaved→planar unpack・実 timestamp mapping（`open_input_stream` は `Unsupported`）。
- `DuplexStream` 型、実機 HW latency 計測・実 capture 走行、他 backend、engine 統合、DAW 意味論、異 clock drift 補正。

## Components and Boundaries

### `ruviform_audio_core::device` — 入力契約（全て additive）

**`DeviceEnumerator`（変更・既存 trait へ method 追加・default なし）:**

| method | 契約 |
| --- | --- |
| `input_devices()` | input-capable device の `Vec<Box<dyn Device>>`。空 `Vec` = 存在しない（error でない）。非 RT・OS API・alloc。`Err` = 列挙失敗（host API 不可・permission denied 等）。 |
| `default_input_device()` | `Ok(None)` = default 未設定/不在（型で表現・error でない）。`Err` = query 失敗。 |

**`Device`（変更・既存 trait へ method 追加・default なし）:**

| method | 契約 |
| --- | --- |
| `default_input_format()` | 開ける `F32` `AudioFormat` を優先返却（output と同規則）。`Err(UnsupportedConfig)` = 入力対応するが F32-capable 範囲なし。`Err(Unsupported)` = 入力機能自体なし。`Err(他)` = query 失敗。 |
| `supported_input_formats()` | F32-capable range のみ返す `Vec<SupportedInputFormat>`。空 `Vec` = 使用可能 range なし（入力非対応を含む）。`Err` = query 失敗。 |
| `open_input_stream(config, data_callback, error_callback)` | `Err(UnsupportedConfig)` = supported 範囲外の config。`Err(Unsupported)` = device/backend が入力 stream を開けない。`Err(DeviceBusy)` = 排他占有。`Err(Backend)` = その他 backend 失敗。非 RT。 |

**`InputStream`（新規 trait・`Send`・`Box<dyn>`）:**

| method | RT | 契約 |
| --- | --- | --- |
| `start()`/`stop()` | 非 RT | 入力配信の開始/停止。`Drop` で配信停止（backend-defined・output と同契約）。`Err` = backend 拒否・stream invalidated。 |
| `format()` | RT 可 | open 時に確定した `AudioFormat` を返す（stored・negotiated intent）。`#[must_use]`。 |
| `buffer_size()` | 非 RT | backend 実測 query。`Err` = 報告不能。 |
| `now()` | 非 RT | capture clock の現在値（`MediaTime` @ `DEVICE_CLOCK_TIMEBASE`）。`Err` = clock 報告不能・overflow。 |
| `clock_domain()` | RT 可 | この stream の timestamp が属する `ClockDomainId`（stored）。`UNKNOWN` = backend が clock を識別不能。`#[must_use]`。 |
| `input_latency()` | 非 RT | `DeviceResult<Option<StreamLatency>>`。`Ok(None)` = backend が latency を報告不能（型で表現）。`Err` = query 失敗。 |

**`OutputStream`（変更・default method 追加のみ）:**

| method | 契約 |
| --- | --- |
| `clock_domain()` | default `ClockDomainId::UNKNOWN`。override で実 domain を報告。RT 可（stored）。 |
| `output_latency()` | default `Ok(None)`（「報告不能」を型で表現）。override で `StreamLatency` を返す。非 RT。 |

**値型・callback（新規）:**

| 型 | 形状 | 契約 |
| --- | --- | --- |
| `InputStreamConfig` | `{ format: AudioFormat, buffer_size: BufferSize }` | `Copy`。`OutputStreamConfig` と同形状の方向別 mirror。 |
| `SupportedInputFormat` | `{ channels, min_sample_rate, max_sample_rate, buffer_size, sample_format }` | `Clone`（非 `Copy`・拡張余地）。`SupportedOutputFormat` と同形状の方向別 mirror。 |
| `InputTimestamp` | `{ callback: MediaTime, capture: Option<MediaTimestamp> }` | `Copy`。`callback` = host 観測時刻（`OutputTimestamp::callback` と同約束）。`capture` = frame 0 の ADC 実時刻 + provenance。`None` = backend 未報告（`callback` で暗黙補完しない）。 |
| `InputCallbackInfo` | `{ timestamp: InputTimestamp, clock_domain: ClockDomainId, frame_position: Option<u64> }` | `Copy`。`capture_stamp()` helper で `InputStamp` を構成（後述）。 |
| `InputDataCallback` | `Box<dyn for<'a> FnMut(AudioBufferView<'a, f32>, InputCallbackInfo) + Send + 'static>` | HRTB。view は callback 返却までの借用（保持禁止）。capture data を読む read view。 |
| `InputErrorCallback` | `Box<dyn FnMut(DeviceError) + Send + 'static>` | RT-adjacent・no alloc・no block・panic 禁止（output と同規則）。 |
| `StreamLatency` | `{ frames: u64, coverage: LatencyCoverage, provenance: TimestampProvenance }` | `Copy`+`Eq`。frames は stream sample rate 基準。 |
| `LatencyCoverage` | `HardwareOnly` / `HardwareAndBuffering` / `Unknown` | `#[non_exhaustive]`。`Unknown` = 含有不明（加算禁止の trigger）。 |
| `DeviceError::ClockDiscontinuity` | 新規 variant（`#[non_exhaustive]` enum への追加） | frame position/timestamp の断絶・restart・reset・検証不能な jump を error callback で通知。`Copy`+`Display`。 |

**`AudioBufferView::try_new`（buffer.rs 追加）:**

```rust
pub fn try_new(data: &'a [S], channels: usize, frames: usize) -> Result<Self>
```

`AudioBufferViewMut::try_new` の immutable 対応版（`&'a [S]`）。`channels == 0`・`channels * frames` overflow・`data.len() != channels * frames` を `Error::InvalidValue` で拒否。capture scratch を view 化する adapter 用 constructor。RT 可（borrow only）。

### 時刻・Clock 契約

- **時刻基準**: 全 device timestamp は `MediaTime` @ `DEVICE_CLOCK_TIMEBASE`（1e9 分母のナノ秒・epoch は backend-defined・差分のみ意味を持つ）。input の `callback`/`capture` は `OutputTimestamp` と同約束。
- **`capture` 意味論**: backend が報告する ADC 実時刻（hardware latency 込み）。`MediaTimestamp.provenance`: `Carried` = backend 報告値、`Derived` = 宣言パラメータからの計算、`Estimated` = adapter/backend の推定。`capture: None` を埋めない。
- **`clock_domain`**: `InputCallbackInfo` は callback ごとに domain を運び（`TimedInputEvent.stamp.domain` と同 model）、`InputStream::clock_domain()` は open 後の照会用。`OutputStream::clock_domain()` も同値を返す。**同一 clock 判定規則**: 値が一致し **かつ** `UNKNOWN` でないときのみ「同一 clock」。`UNKNOWN` 同士は「同一」とみなさない（同一性を証明できないため）。drift 補正は domain 一致時のみ意味を持ち、異 domain の加算補正は consumer の明示 rejection。
- **`frame_position`**: capture clock 上の `input` frame 0 の位置。epoch 内で非減少、無損失時は callback あたり `frames` 進行。backend 未報告は `None`（callback 回数からの合成禁止 — 真の断絶を隠す）。断絶は `DeviceError::ClockDiscontinuity` で通知し、前後の position/timestamp を接続しない。
- **stamp→frame**: `InputCallbackInfo::capture_stamp()` が `capture`（`Some` の場合）から `InputStamp { ticks: u64::try_from(capture.time.value()).ok()?, base: DEVICE_CLOCK_TIMEBASE, domain: clock_domain, provenance: capture.provenance }` を構成して `Some` を返す。`capture: None` または tick が `u64` に収まらない場合は `None`。consumer は `ClockAnchor` を stream 開始時（非 RT）に calibrate し、`map_stamp_to_frame` で engine frame へ写像する（engine 責務・本 PoC では写像可能性の検証のみ）。

### Latency 契約

- `StreamLatency.frames` は stream sample rate の frame 単位。coverage: `HardwareOnly`（converter path のみ・buffer 除外）、`HardwareAndBuffering`（converter+backend buffer の全体）、`Unknown`（含有不明 — consumer は他成分との無条件加算をしない）。
- provenance: `Carried`（backend 報告値）/ `Derived`（宣言値からの計算）/ `Estimated`（推定）。
- **二重計上防止**（Synvelis #12 §16.5）: `capture` が `Carried`/`Derived` で得られる場合、consumer は `callback - capture` の差分で latency を導出でき、`StreamLatency` を再加算しない。`playback` timestamp（output）が得られる場合も同規則。`coverage: Unknown` の latency を別成分と無条件加算しない。

### RT 契約

- `InputDataCallback` 内: alloc/dealloc・ブロッキング・file/network I/O・無制限待機/リトライ・panic・`InputStream` 制御 API（`start`/`stop`/`Drop`）の再入を禁止。view は借用 — 保持せず、必要なら bounded copy で pool へ取り込む。
- `InputErrorCallback` 内: no alloc・no block・panic 禁止（`DeviceError` が `Copy` なのはこのため）。
- `InputStream`: `format()`/`clock_domain()` は RT 可（stored value）、`start`/`stop`/`buffer_size`/`now`/`input_latency`/`Drop` は非 RT。
- handoff（engine 責務・mock が検証）: callback 内で view → pre-allocated `CaptureBlock`（pool）へ bounded copy → `ruviform_rt::channel` `try_push` で非 RT へ配送。満杯は `try_push` 失敗 → overflow 計数（block/panic しない）。drain 済み block は非 RT で free へ recycle（#185 の chunk-pool モデルと同型）。

### `ruviform-audio-device` — cpal 入力側表面

- `Host::input_devices`/`default_input_device` → `cpal::HostTrait::input_devices`/`default_input_device` を `CpalDevice` へ wrap（output と同型）。
- `CpalDevice::default_input_format` → `inner.default_input_config()` を `f32_format`/`synthesize_default_format` 経由で mapping（output と同規則: backend default が F32-openable なら fast path、非 F32/unmappable layout なら F32 合成）。
- `CpalDevice::supported_input_formats` → `inner.supported_input_configs()` を `map_supported_input_range`（`map_supported_range` と同 filter 規則を `SupportedInputFormat` へ写像）で mapping。空 `Vec` = 使用可能 range なし。
- `CpalDevice::open_input_stream` → `Err(DeviceError::Unsupported)`（実 plumbing + 実機検証は後続 Issue。capture を偽装しない）。
- cpal `InputCallbackInfo::timestamp()` は `callback`+`capture` instant を持つ（実装時に `OutputStreamTimestamp` と同 `instant_to_media_time` + epoch fallback を再利用する設計 — 本 PoC では未実装）。

### mock input backend（tests/mock_input_backend.rs）

`mock_backend.rs` と同構造の scripted fake:

- `FakeInputEnumerator`/`FakeInputDevice`/`FakeInputStream` が新 trait を実装。`start()` で script 化した capture block（frames・sample pattern・`callback`/`capture` instant・`frame_position`）を per-block heap scratch（test fake につき許容 — real backend は自前の capture buffer を借りる）から `AudioBufferView` 経由で callback へ配送、scripted error を `error_callback` へ配送。
- `Shared` observation sink で firings/errors/start/stop/now を記録（test thread で lock — RT でない）。
- handoff: callback で `AudioBufferView` → pool `CaptureBlock`（`Box`・`free` channel から `try_pop`）へ copy → `filled` channel へ `try_push`。非 RT drain で内容・`frame_position` 連続性・timestamp を検査し block を `free` へ recycle。
- duplex: 同一 `Shared` の `clock_domain` を持つ mock output stream と input stream を両方開き、`clock_domain()` 一致で同一 clock 判定を検証。

## Data and Control Flow

- 入力: `Host::input_devices()` → `Device::default_input_format`/`supported_input_formats` → `open_input_stream(config, cb, ecb)` → `InputStream::start()` → backend RT thread が `InputDataCallback(AudioBufferView, InputCallbackInfo)` を駆動 → engine が view から pool block へ copy → SPSC → 非 RT collector → recycle。`InputStream::stop()`/`Drop` で停止。
- 時刻: backend instant → adapter（epoch-fallback `instant_to_media_time`）→ `MediaTime` → `capture_stamp()` → `InputStamp` → `ClockAnchor`（非 RT calibrate）→ `map_stamp_to_frame` → engine frame。
- エラー: backend error → `map_error` → `DeviceError` → `InputErrorCallback`（RT-adjacent）。`ClockDiscontinuity` は adapter が backend restart/jump を検出したとき通知。

## Non-Functional Constraints

- additive のみ（`OutputStream` への追加は default method）。`unsafe` 不使用。`#[non_exhaustive]` を全公開 enum へ。`**RT contract:**` doc ラベルを API 単位で付与。
- cpal 型・エラー・feature を公開 API に露出しない。`ruviform-rt` は audio-device の **dev-dep のみ**（mock handoff 用・normal 依存に現れないことを `cargo tree` で確認）。
- `AudioBufferView::try_new` は `try_new` 系の既存検証規則（`channels == 0`・overflow・len 不一致 → `Error::InvalidValue`）を踏襲。

## Risks and Trade-offs

- **片方向 device で必ず失敗する method**: `Device` が方向別 method を両持ちするため、input 非対応 device で `open_input_stream` は `Err(Unsupported)`、output 非対応で `open_output_stream` は `Err(UnsupportedConfig)`/`Unsupported` — 失敗面を契約で定義済みにして緩和。
- **`clock_domain` は stream open 後にしか比較不能**: open 前の同一 clock 事前判定を提供しない（backend が証明できない claim を作らない）。
- **`AudioBufferView::try_new` 追加は output 側にも公開**: immutable view の constructor を増やす additive — 既存挙動不変。
- **mock は cpal 実装を証明しない**: mock は port shape と handoff pattern の実装可能性のみ証明。cpal input stream・HW latency・実同一 clock は未検証として記録。

## Deferred Design Concerns

| 論点 | 本設計での扱い | 移管先 | リスク |
| --- | --- | --- | --- |
| `DuplexStream` | 独立 stream + domain 相関 | backend capability 実証後の実装 Issue | 過早な抽象 |
| cpal input plumbing | `Unsupported` | #187 後続実装 Issue | 未検証の偽装 |
| cpal `clock_domain`/`frame_position`/`input_latency`/`output_latency` の実値 | `UNKNOWN`/`None`/`Ok(None)` 維持 | cpal adapter の実機調査 Issue | 値捏造 |
| engine session/graph 統合 | port 検証のみ | ruviform-audio engine Issue | 二重設計 |
| 異 clock drift 補正 | domain 識別のみ | engine drift 設計 Issue | 誤加算 |
| `list_devices` example 入力列挙 | 任意（本 PoC では未追加） | 実機検証 lane | — |

## Verification Notes

- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` exit 0（既存 output 回帰含む）
- [ ] `mock_input_backend` が deterministic capture→handoff・同一 clock 判定・error 注入を検証し、`rt_` 契約テストで callback→handoff alloc 0
- [ ] `CpalDevice::open_input_stream` → `Err(DeviceError::Unsupported)`、`input_devices`/`default_input_device`/`supported_input_formats` が compile+mapping 検証
- [ ] `AudioBufferView::try_new` が view contract を持ち、doctest で検証
- [ ] `cargo tree -p ruviform-audio-device --edges normal` に `ruviform-rt` が現れない
