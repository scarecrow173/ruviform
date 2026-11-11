---
id: "DESIGN-5Zp45sCRwv11kYwJ4MGJBX"
type: "design"
status: "approved"
title: "Issue #565 WASAPI-native input path 設計 — per-packet device_position を frame_position に写す"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/565"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-565-wasapi-input-frame-position.md"
  implemented-by: []
  depends-on:
    - "docs/designs/audio-device-boundary-and-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/select-audio-device-backend.md"
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
---

# Issue #565 design: WASAPI-native input path

## なぜ native path か（代替案の棄却）

| 案 | 棄却理由 |
| --- | --- |
| timestamp→frame 合成 | port 契約で禁止。合成 position は不連続を隠し、QPC≠audio 発振器でドリフトする |
| 同一 endpoint の第 2 `IAudioClient` | stream epoch が別で相関不能 |
| cpal internals 経由 | `StreamInner`/`audio_clock` は private で到達不能 |
| forked cpal | 外部保守コスト・self-contained でない |

cpal の WASAPI input 経路（`host/wasapi/stream.rs`）は `GetBuffer` の `device_position` を discontinuity 検査にのみ使い、`clock_position()` の戻り値 position も破棄する。cpal の公開 `InputCallbackInfo` には frame position が存在しない。したがって adapter 内に WASAPI-native の input stream を実装するのが唯一の正直な経路。

## 「第 2 backend は別 crate」原則の例外

crate README / lib.rs docs は「第 2 backend は別 crate」を掲げるが、本件は **同一 backend（WASAPI）内の platform gap-fill** であり backend 追加ではない。adapter→adapter 依存は禁止のため別 crate に切り出すと `CpalDevice` の cpal 内部状態（`as_inner()` の `IMMDevice` 解決）に到達できず解決不能。よって同一 crate 内の `#[cfg(windows)]` モジュールとして実装し、本 design note を例外の記録とする。

## アーキテクチャ

### ルーティング

`CpalDevice` に `host_name: &'static str`（`clock_profile` 解決に使ったのと同じ値）と `default_input: bool`（`Host::default_input_device()` 由来のみ true）を保持する。`open_input_stream` は negotiation・F32 検証・scratch 確保の後、

```rust
#[cfg(windows)]
if self.host_name == "WASAPI" {
    return wasapi_input::open(...);
}
```

で native path に分岐する。それ以外は従来の `build_input_stream::<f32>` cpal 経路を維持する。`clock_profile` の計算は `CpalDevice::new(inner, host_name, default_input)` 内に集約し、host.rs の 4 箇所の構築を更新する。

### `wasapi_input.rs` の構成（cpal `host/wasapi` の写し + frame_position 追加）

- **COM**: thread_local の `ComGuard`（`CoInitializeEx(STA)`、`RPC_E_CHANGED_MODE` 許容、RAII `CoUninitialize`）— cpal `host/com.rs` と同型。control path（open/activate/monitor drop）で使用。worker thread は WASAPI client オブジェクトが free-threaded のため cpal 同様 init しない。
- **endpoint 解決**:
  - `default_input == true` → `Com::StringFromIID(&DEVINTERFACE_AUDIO_CAPTURE)` + `ActivateAudioInterfaceAsync`（同期完了 handler + mpsc）で reroute 可能な `IAudioClient` を取得。`DefaultDeviceMonitor` 相当（`IMMNotificationClient` + event + `pending_device_changed` flag）を常設し、default 変更を error callback に通知する。
  - `default_input == false` → `cpal::Device::as_inner()` → `DeviceInner::Wasapi(d)` → `d.immdevice()` で `IMMDevice` を取得し `Activate(CLSCTX_ALL)`（pinned endpoint、monitor なし — cpal の `DeviceHandle::Specific` と同じ扱い）。
- **初期化**（cpal `build_input_stream_raw_inner` の写し）: `WAVEFORMATEXTENSIBLE` F32 を構築 → `Initialize(SHARED, EVENTCALLBACK, buffer_duration, 0, &wfx.Format, None)` → `GetBufferSize` → `GetDevicePeriod`→`shared_mode_period_frames` 相当 → `CreateEventW` → `SetEventHandle` → `GetService::<IAudioCaptureClient>` + `GetService::<IAudioClock>` → `GetStreamLatency`。
- **worker thread**: `std::sync::mpsc` command channel（PlayStream/PauseStream/Terminate → `audio_client.Start()/Stop()`）+ `WaitForMultipleObjectsEx` で `[pending_scheduled_event, audio_event, monitor_event?]` を待機。cpal `run_input`/`process_commands_and_await_signal` と同型。
- **packet loop**（`process_input` の写し + position 供給）: `GetNextPacketSize` → `GetBuffer(&mut buf, &mut frames, &mut flags, Some(&mut device_position), Some(&mut qpc_position))` → `AUDCLNT_S_BUFFER_EMPTY` は continue → flags/position で discontinuity 判定 → `IAudioClock::GetPosition` で callback instant → `InputCallbackInfo` 構築 → `&[f32]` 化して `read_interleaved` → `ReleaseBuffer`。

### `frame_position` の供給規則

- `frame_position = Some(device_position)` — `GetBuffer` が返す「packet 先頭 frame の device frame position」。pcm を届ける同一 stream・同一 packet の値であり、合成しない。
- `PositionTracker { expected: Option<u64> }`: packet 毎に `device_position == expected`（初回は `None` → 無条件 accept）を検査し、`expected = device_position + frames` に更新する。`device_position != expected` は position jump → `ClockDiscontinuity`。stop/start による position reset もここで自然に検出される（消費者起点の境界でも正直に通知）。
- `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY`: cpal 同様 `device_position != 0` ガード付きで flag を見て `ClockDiscontinuity`（cpal は `Xrun` に写すが、port は `ClockDiscontinuity` を設計済み — spec 要件 4）。
- discontinuity 検出時も packet は実位置つきで配送する（consumer が境界で join を止める契約。cpal も error 通知後に配送を続ける）。
- first packet `device_position == 0`: `expected == None` で検査 skip、`expected = 0 + frames` に初期化 — cpal の「先頭 packet で flag 未定義」edge を tracker が自然に処理。

### timestamp 写像

- `callback`: `IAudioClock::GetPosition(&mut pos, Some(&mut qpc))` の `qpc`（100ns 単位）→ ns → `StreamInstant` → `instant_to_media_time`（epoch fallback 共用のため `pub(crate)` 化）。
- `capture`: `GetBuffer` の `qpc_position`（100ns → ns）。`qpc_position == 0` は「未報告」 sentinel として `None`（cpal の `StreamInstant::ZERO` 扱いと同等）。provenance は `TimestampProvenance::Carried`（clock_profile 由来）。
- `clock_domain`: `clock_profile.domain`（WASAPI = `MONOTONIC`）。
- `now()`: `QueryPerformanceCounter` → 100ns 換算 → `StreamInstant` → `instant_to_media_time`（cpal `Stream::now` と同じ 100ns グリッド）。
- `buffer_size()`: `BufferSize::Fixed(period_frames)`（cpal `StreamTrait::buffer_size` と同値）。
- `input_latency()`: `GetStreamLatency`（hns）→ frames 換算し `StreamLatency { frames, coverage: LatencyCoverage::Unknown, provenance: Carried }` を `Some` で報告。`Unknown` coverage は port が「構成要素不明・無条件加算禁止」と定義した正直なラベル。cpal は `Ok(None)` だが、backend が報告する実値を捨てる理由がない（port は `None` = unsupported と `Some(Unknown)` を区別する）。

### `DeviceError` 写像（HRESULT → DeviceError）

cpal `From<windows::core::Error> for cpal::Error` の AUDCLNT 表を踏襲し、`map_error` の ErrorCtx 規則に載せる:

| HRESULT | ErrorKind 相当 | Control | ErrorCallback |
| --- | --- | --- | --- |
| `AUDCLNT_E_SERVICE_NOT_RUNNING` | HostUnavailable | `Backend("host_unavailable")` | 同左 |
| `AUDCLNT_E_DEVICE_INVALIDATED` / `AUDCLNT_E_ENDPOINT_CREATE_FAILED` | DeviceNotAvailable | `DeviceNotFound` | `StreamInvalidated` |
| `AUDCLNT_E_DEVICE_IN_USE` | DeviceBusy | `DeviceBusy` | `DeviceBusy` |
| `AUDCLNT_E_RESOURCES_INVALIDATED` | StreamInvalidated | `StreamInvalidated` | `StreamInvalidated` |
| `AUDCLNT_E_UNSUPPORTED_FORMAT` 他 format/buffer 系 | UnsupportedConfig | `UnsupportedConfig` | `UnsupportedConfig` |
| `AUDCLNT_E_WRONG_ENDPOINT_TYPE` 他 state 系 | UnsupportedOperation | `Unsupported` | `Unsupported` |
| その他 / COM 失敗 | BackendError | `Backend("backend")` | `Backend("backend")` |

`WaitForMultipleObjectsEx` の `WAIT_FAILED` → `StreamInvalidated`（cpal 同様）。thread spawn 失敗 → `Backend("resource_exhausted")`。

### RT / unsafe / 依存

- packet loop は allocation-free: scratch は open 時確保済み、`&[f32]` は `GetBuffer` ポインタの借用、`InputCallbackInfo` は stack 構築、error callback は `FnMut(DeviceError)` で `Copy` payload。
- `unsafe` は `wasapi_input.rs` に局所化: `#![expect(unsafe_code)]` + 各 block に SAFETY コメント。対象は COM 呼出・`GetBuffer` ポインタ→slice・`WaitForMultipleObjectsEx`・event handle・`#[implement]` vtable。
- `windows` crate: `version = "0.62"`（cpal と同一、lockfile 追加パッケージゼロ）、`[target.'cfg(windows)'.dependencies]`、features は `Win32_Media`/`Win32_Media_Audio`/`Win32_Media_KernelStreaming`/`Win32_Media_Multimedia`/`Win32_Foundation`/`Win32_System_Com`/`Win32_System_Com_StructuredStorage`/`Win32_System_Threading`/`Win32_System_Performance`/`Win32_System_SystemServices`/`Win32_System_Variant` 程度（compile で不足分を補う）。

## リスク

- `cpal::Device::as_inner()` は `#[doc(hidden)]` API — cpal 0.18 pin のため許容。pin 更新時に再監査が必要。
- `ActivateAudioInterfaceAsync` の reroute semantics は cpal と同一機構を使うため parity 維持。
- `GetStreamLatency` を `input_latency` で `Some` 報告するのは cpal path（`None`）との差分 — `Unknown` coverage で誤用を防ぐ設計上の正直な選択として記録。
