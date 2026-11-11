---
id: "SPEC-4LwDFLWjvbGeZQSqW0Y93X"
type: "spec"
status: "approved"
title: "Issue #565 WASAPI input frame_position: IAudioClock::GetPosition の device position を port に乗せる"
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
    - "https://github.com/scarecrow173/ruviform/issues/565"
  implemented-by: []
  depends-on:
    - "docs/designs/audio-device-boundary-and-poc.md"
    - "docs/adr/select-audio-device-backend.md"
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "crates/ruviform-audio-core/src/device.rs"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
---

# Issue #565: WASAPI input `frame_position` 供給

## 背景と問題

`ruviform-audio-core` の `InputCallbackInfo::frame_position` は「callback block の frame 0 の capture-clock フレーム位置」を表す port 契約フィールドであり、WASAPI backend はデバイスが実際に報告する per-packet `device_position` を持っているにもかかわらず、現行 adapter は常に `None` を返している。

Issue 本文の前提（adapter が position を破棄している）は正確ではない。検証済みの実態:

- 破棄しているのは **cpal 0.18.2 内部**である。`cpal::host::wasapi::stream::process_input` は `IAudioCaptureClient::GetBuffer` の `device_position` out-param を discontinuity 検査にのみ使い、cpal の公開 `InputCallbackInfo` は `timestamp()` のみで frame position フィールド自体を持たない。`IAudioClock::GetPosition` の返り値も `let (callback, _position)` で破棄される。`StreamInner` の `audio_clock` は worker `RunContext` に隔離され、adapter からは到達不能。
- ruviform adapter 側は `InputCallbackState::handle` で `frame_position: None` を構築している（`src/stream.rs`）。これは cpal が値を提供しないための正直な `None` であり、adapter 自体の欠陥ではない。

## 決定（Coordinator 承認済み）

in-repo で受入条件を満たす唯一の正直な経路は、**WASAPI-native 入力パスを `ruviform-audio-device` 内に実装する**こと。これは crate README の「第 2 backend は別 crate」原則に対する明示的な例外であり、adapter→adapter 依存禁止のため別 crate では解決不能である（design note に記録）。

- **replace-by-default**: Windows かつ host=WASAPI の input stream のみ native path を既定とする。WASAPI 以外の Windows host（ASIO 等）・非 Windows・output stream は従来どおり cpal 経路で不変。
- **依存**: `windows` crate（0.62 系、cpal と同一バージョンで Cargo.lock 追加パッケージゼロ）。features は WASAPI/COM/スレッド系に限定。
- **unsafe**: この crate の src で初。module-level `#![expect(unsafe_code)]` + 各 unsafe block に SAFETY コメント（先例: `ruviform-midi-device/src/os/windows.rs`）。

## 要件

1. WASAPI input stream の data callback で `InputCallbackInfo::frame_position` が `Some` を返し、値が device の capture frame 進行と整合する（連続 block で frame 数分だけ増加）。headless では `InputCallbackInfo` 構築部の単体 test で伝播を検証し、実機 lane が実行できた場合のみ実測証拠を添える。
2. `frame_position` を報告できない backend では従来どおり `None`（合成しない）。timestamp→frame 合成は port 契約で禁止。
3. `frame_position` は PCM を届ける同一 stream の per-packet `device_position` のみから供給する。`IAudioCaptureClient::GetBuffer` の `devicePosition`（packet 先頭 frame の device frame position）をそのまま使い、合成・補正しない。
4. `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` や position jump（期待位置との不一致）は `DeviceError::ClockDiscontinuity` に正直に写像する。cpal が `Xrun` に混同するのとは異なり、port が設計した variant を使う。
5. first packet `device_position == 0` の edge を扱う（cpal 同様、先頭 packet では flag が未定義であり得る）。
6. 既存の `capture_stamp()`/timestamp 経路の契約は不変。`clock_profile` の主張（WASAPI = `MONOTONIC`/`Carried`）は native path でも真のまま。
7. RT 制約: position を載せる callback 経路は allocation-free・bounded・lock-free を維持し、`rt_` 系 `assert_no_alloc` test の対象にする。
8. cpal parity で写す項目: `AUDCLNT_S_BUFFER_EMPTY` 処理、`DefaultInput` device の default-device monitor 相当、`GetStreamLatency` 扱い、scratch サイジング、`now()`/`buffer_size()`。

## 受入条件（Issue #565 本 slice）

- [ ] WASAPI input stream の data callback で `frame_position` が `Some` を返し、連続 block で frame 数分だけ増加する（headless 単体 test + 実機 lane が動けば実測）。
- [ ] 供給不能 backend は従来どおり `None`。
- [ ] `capture_stamp()`/timestamp 経路の契約は不変。
- [ ] 実機 gate 行（`test-audio-real-input` 実走・latency parity・default-device change 再経路・非 Windows 影響なし）は実行できた分だけ Passed、未実行は理由付き NotRun。

## スコープ外

- `crates/ruviform-audio-core` の port 契約（`InputCallbackInfo`/`InputTimestamp`/`DeviceError` の型・意味論）は不変。
- output stream、非 WASAPI host、非 Windows platform は変更しない。
- cpal への upstream 提案・fork は行わない。
