---
id: "IMPL-034VxYJLKGGs9nLvyaDgHT"
type: "impl"
status: "completed"
title: "Issue #565 WASAPI-native input frame_position 供給"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-device/src/wasapi_input.rs"
      - type: "file"
        path: "docs/specs/issue-565-wasapi-input-frame-position.md"
      - type: "file"
        path: "docs/designs/issue-565-wasapi-input-frame-position-design.md"
      - type: "file"
        path: "docs/plans/issue-565-wasapi-input-frame-position-plan.md"
      - type: "file"
        path: "docs/tasks/issue-565-task1-wasapi-input-frame-position.md"
      - type: "file"
        path: "docs/test-specs/issue-565-wasapi-input-frame-position.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-device/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio-device/README.md"
      - type: "file"
        path: "crates/ruviform-audio-device/src/convert.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/device.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/host.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/tests/real_input_e2e.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/issue-565-task1-wasapi-input-frame-position.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-565-task1-wasapi-input-frame-position.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-565-wasapi-input-frame-position.md"
    - "docs/designs/issue-565-wasapi-input-frame-position-design.md"
    - "docs/plans/issue-565-wasapi-input-frame-position-plan.md"
    - "docs/test-specs/issue-565-wasapi-input-frame-position.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/565"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #565 WASAPI-native input frame_position 供給

## Summary

Windows/WASAPI input stream で `InputCallbackInfo.frame_position` が常に `None` だった問題を、adapter 内に WASAPI-native 入力パス (`src/wasapi_input.rs`) を追加して解消した。cpal の WASAPI host は `IAudioCaptureClient::GetBuffer` が返す `device_position` を discontinuity 検査にのみ使い捨て、公開 `InputCallbackInfo` に frame position フィールドを持たないため、in-repo で受入条件を満たす唯一の正直な経路として `#[cfg(windows)]` かつ host=WASAPI の input stream のみ cpal をバイパスする native 経路を実装した。`frame_position` は packet の `device_position` のみから供給し、timestamp→frame 合成は行わない。`AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` および position jump は `DeviceError::ClockDiscontinuity` に写像する。

## Context

- 実施タスク: docs/tasks/issue-565-task1-wasapi-input-frame-position.md
- 上流: docs/specs/issue-565-wasapi-input-frame-position.md (approved) / docs/designs/issue-565-wasapi-input-frame-position-design.md / docs/plans/issue-565-wasapi-input-frame-position-plan.md / docs/test-specs/issue-565-wasapi-input-frame-position.md
- Issue: https://github.com/scarecrow173/ruviform/issues/565
- 前提検証: cpal 0.18.2 `host/wasapi/stream.rs` の `process_input` は `GetBuffer` の `device_position` を `device_position != 0 && DATA_DISCONTINUITY flag` の Xrun 判定にのみ使用し、公開 `InputCallbackInfo` は `timestamp()` のみ。`StreamInner.audio_clock` は worker `RunContext` に隔離され外部から到達不能。timestamp→frame 合成は port 契約で禁止(不連続を隠す + QPC≠audio 発振器でドリフト)。したがって WASAPI-native 入力パスが唯一の正直な経路と判断された(Coordinator 承認済みの設計判断: replace-by-default、crate 内 platform gap-fill、`windows 0.62` 依存、局所 `unsafe` 許可)。

## Implementation

### Changes

- `crates/ruviform-audio-device/src/wasapi_input.rs` (新規, Windows-only): WASAPI-native input stream 実装。
  - endpoint 解決: pinned device は `Device::id()` の endpoint ID → `IMMDeviceEnumerator::GetDevice` → `IAudioClient` activate。default input device は `ActivateAudioInterfaceAsync` + `IMMNotificationClient` で default 変更を監視し再経路。
  - `IAudioClient::Initialize` (shared mode, event-driven, F32 `WAVEFORMATEX`), `GetService(IAudioCaptureClient)` + `GetService(IAudioClock)`。
  - worker thread: `WaitForSingleObject` で capture event 待ち → `GetNextPacketSize`/`GetBuffer` ループ → `device_position`/`qpc_position`/flags 取得 → 既存 `InputCallbackState::handle` に `Some(device_position)` を渡して `InputCallbackInfo` 構築 → `ReleaseBuffer`。command channel (start/stop/terminate) は cpal WASAPI stream と同型。
  - `PositionTracker`: `prev_position + prev_frames` との連続性を検査し、jump/reset を検出して resync しつつ `DeviceError::ClockDiscontinuity` を error callback へ報告。first packet `device_position == 0` を正常系として扱う。
  - packet が scratch capacity を超える場合は複数 callback に分割し、`pos_next = pos_current + frames_delivered` で frame position を進める (convert.rs の chunk 機構と整合)。
  - QPC position → `StreamInstant`/media time 変換は既存 `instant_to_media_time` を crate-visible にして再利用。`clock_profile` の主張 (WASAPI = MONOTONIC/Carried) は不変。
  - `GetDevicePeriod`/`GetBufferSize`/`GetStreamLatency` を cpal parity で `buffer_size()`/`input_latency()` に反映。HRESULT → `DeviceError` 写像は cpal host の写像に準拠。
  - callback 経路は allocation-free (`assert_no_alloc` で検証)。
- `crates/ruviform-audio-device/src/device.rs`: `CpalDevice` に `host_name: &'static str` と `default_input: bool` を追加。`open_input_stream` で `#[cfg(windows)] && host_name == "WASAPI"` の場合のみ `wasapi_input::open_input_stream` に route。それ以外 (非 Windows, WASAPI 以外の Windows host, output stream) は従来どおり cpal 経路。cpal callback は `state.handle(data, info, None)` のまま。
- `crates/ruviform-audio-device/src/host.rs`: 4 箇所の `CpalDevice::new` 呼出に host 名と default-input フラグを伝搬。default input device のみ `default_input = true` (reroutable default-device activation + monitor の対象)。
- `crates/ruviform-audio-device/src/stream.rs`: `InputCallbackState::handle` が `frame_position: Option<u64>` を受け取り `InputCallbackInfo` に載せる。`instant_to_media_time` を `pub(crate)` 化。既存 RT alloc test の callsite を `None` 渡しに更新。
- `crates/ruviform-audio-device/src/convert.rs`: `chunk_input_timestamp_info` の doc を「adapter は WASAPI-native 経路で `Some` を供給し得る」旨に更新 (実装は既に chunk 単位の `frame_position` 進行をサポート済み)。
- `crates/ruviform-audio-device/src/lib.rs`: `#[cfg(windows)] mod wasapi_input;` 追加。
- `crates/ruviform-audio-device/Cargo.toml`: Windows 限定で `windows = "0.62"` (WASAPI/COM/Threading/Foundation/Performance feature) と `windows-core = "0.62"` (`implement` macro の展開先として直接依存が必要) を追加。新規 package 追加なし (cpal の dep として Cargo.lock に既存)。
- `crates/ruviform-audio-device/tests/real_input_e2e.rs`: WASAPI host では全 callback が `frame_position: Some` であること、position gap 数 ≤ `ClockDiscontinuity` error event 数であることを assert。非 WASAPI backend は `None` を許容。
- `crates/ruviform-audio-device/README.md`: crate 内 native path 例外 (「第 2 backend は別 crate」原則の例外、理由: adapter→adapter 依存禁止のため別 crate では解決不能) を記録。
- `Cargo.lock`: windows/windows-core 依存解決の反映。

### Decisions During Implementation

- **native path を crate 内に置く**: 「第 2 backend は別 crate」原則の例外として `wasapi_input.rs` を `ruviform-audio-device` 内に実装。adapter→adapter 依存禁止のため別 crate では解決不能であり、design note/README に理由を記録。
- **`windows` crate feature を必要最小限に絞る**: `Win32_Foundation`/`Win32_Media`/`Win32_Media_Audio`/`Win32_Media_KernelStreaming`/`Win32_Media_Multimedia`/`Win32_Security`/`Win32_System_Com`/`Win32_System_Com_StructuredStorage`/`Win32_System_Performance`/`Win32_System_SystemServices`/`Win32_System_Threading`/`Win32_System_Variant`。
- **module-level `#[expect]` で `unsafe_code`/`clippy::multiple_unsafe_ops_per_block`/`clippy::missing_safety_doc` を局所許可**: `windows::core::implement` macro 展開 span には item-level `#[expect]` が効かないため module level に集約 (file 内の `implement` は notification/async handler の 2 箇所のみ)。各 unsafe block に SAFETY コメント付き (`ruviform-midi-device/src/os/windows.rs` の先例に準拠)。
- **`windows-core` を直接依存に追加**: `windows::core::implement` が `windows_core::...` path に展開されるため。`windows` crate 自体は cpal の dep として lockfile に既存。
- **cpal `DeviceInner` の exhaustive match**: Windows build では `Wasapi` variant のみのため wildcard arm を除去。
- **default device の `IMMNotificationClient` + `ActivateAudioInterfaceAsync`**: cpal `DefaultDeviceMonitor` 相当を native path にも実装し、default input 変更時の再経路を保証。

## Related Experiments

### Adopted

なし (実機 probe は一時的な example で検証後に削除。恒久的な experiment log は作成していない)。

### Rejected

なし。

## Validation

- `cargo check -p ruviform-audio-device` (Windows): Pass
- `cargo clippy -p ruviform-audio-device --all-targets`: Pass (warning 0)
- `cargo test -p ruviform-audio-device` (lib tests): 75 tests Pass — `handle_packet_supplies_frame_position_and_carried_capture` / `handle_packet_reports_discontinuity_and_still_delivers` / `handle_packet_chunked_delivery_advances_frame_position` / `rt_wasapi_packet_path_allocates_nothing` / `PositionTracker` 系 / wave-format 系を含む
- `cargo test -p ruviform-audio-device --features real-device --test real_input_e2e -- --ignored --test-threads=1` (実機, Windows 11, Audient iD4): **2 tests Pass**
  - `real_input_stream_captures_planar_audio`: callbacks=194, `frame_position: some=194 gaps=0 first=0 last=85113`, `capture timestamp: some=194`, `clock discontinuities: 0`, clock domain 一致, channel mismatch 0, timestamp regression 0
  - `real_duplex_streams_agree_on_clock_domain`: input/output clock domain 一致 (`ClockDomainId(2)`)
- 直接 probe (一時 example, 検証後削除): pinned 2 endpoint + default endpoint で `frame_position=Some` が全 callback に付与されること、位置が 441 frames/callback で単調進行すること、`DATA_DISCONTINUITY` flag → `ClockDiscontinuity` error が発火することを実機確認
- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `test-msrv` / `build` / `lint-audio-real-input`: 本 IR 作成時点で順次実行 (結果は PR 本文に記録)

## Risks

- WASAPI-native path は Windows/WASAPI input のみに限定されるが、shared mode event-driven capture の挙動はデバイス依存。`DATA_DISCONTINUITY` flag を立てるが position が連続するデバイス (例: USB audio interface) では `ClockDiscontinuity` が過剰報告され得る — ただし cpal も同一条件で Xrun を報告しており parity は保たれている。
- `GetStreamLatency` が 0 を返すデバイスでは `input_latency().frames = 0` となる (cpal と同じ passthrough 動作)。
- 非 Windows / 非 WASAPI / output stream は cpal 経路のまま変更なし。`frame_position` は引き続き `None`。

## Follow-ups

- WASAPI output stream の `frame_position` 対応は本 Issue の範囲外 (output 側の device position 供給は別 Issue で検討)。
- default-device 変更時の stream 再経路は実装済みだが、実機での default-device 切替シナリオは e2e lane の対象外。必要なら別途実機検証タスクを起票。
