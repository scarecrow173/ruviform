---
id: "TESTSPEC-RF628sBscripted0005"
type: "test-spec"
status: "approved"
title: "Issue #628 テスト仕様 — scripted port-trait backend"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
    - "docs/knowledge/docs-operations-and-agent-workflow.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-628-scripted-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-scripted-backend.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-628-scripted-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-628-scripted-backend.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 テスト仕様 — scripted port-trait backend

## Feature: Enumerate / Describe

### Rule: 列挙と default 選択が script 通りに動く

- Given `ScriptedBackend` に output-capable / input-capable /
  bidirectional の device を登録
- When `output_devices`/`input_devices`/`default_*` を呼ぶ
- Then 非対応 direction は `Ok(vec![])`/`Ok(None)`/`Err(Unsupported)`、
  `vanish` 中の device は列挙から除外、default は明示 index → 先頭
  capable → `Ok(None)` の順で解決される。

### Rule: enumerate fault は全メソッドに適用される

- Given `set_enumerate_error(Some(DeviceError::Backend("permission_denied")))`
- When いずれの列挙メソッドを呼ぶ
- Then `Err(Backend("permission_denied"))` が返る（permission denial の
  honest mapping）。

### Rule: describe/open fault の正負

- Given `fail_next_open_*`（once）/ `set_open_*_error`（persistent）/
  vanished device / 非対応 direction / 非 F32 format / range 外
  rate/channels/buffer
- When `open_*_stream` を呼ぶ
- Then script の `DeviceError` または `DeviceNotFound`/`Unsupported`/
  `UnsupportedConfig` が返り、once は 1 回で消費される。

### Rule: 同 direction の多重 open は DeviceBusy

- Given open 中の output stream が存在
- When 同 device で再度 `open_output_stream`
- Then `Err(DeviceBusy)`。`stop()`+drop 後は再 open 可能（reconnect）。

## Feature: Stream ライフサイクルと報告値

### Rule: start/stop/pump の状態機械

- Given stream を open
- When `pump_*_next` を `start()` 前に呼ぶ
- Then `Err`（not started）。`start()` 後は pump が `PumpReport` を返し、
  `stop()` 後は再び失敗。`start`/`stop` は冪等。

### Rule: format/buffer/latency/clock_domain/now の正直な報告

- Given script で latency・clock_domain・buffer を指定
- When `format()`/`buffer_size()`/`*_latency()`/`clock_domain()`/`now()`
  を呼ぶ
- Then script・VirtualClock 通りの値が返る。`buffer_size()` は固定時
  `Fixed(n)`、可変 cycle 時 `Default`。

## Feature: PCM 経路

### Rule: input は PcmScenario を callback へ再生する

- Given `InputSpec.pcm` に既知 pattern の scenario（gap 含む）
- When `start()`+pump を実行し consumer の `InputDataCallback` が
  view を記録
- Then 各 window の sample が `pcm_reference_block` oracle と一致し、
  gap window では silence + `Xrun` が error callback へ通知される。

### Rule: output は callback 書込みを capture する

- Given consumer が `OutputDataCallback` 内で fixture を view へ書く
- When pump を実行
- Then `captured_output()` の `PcmBlock` が書込み内容と一致し、
  `pcm_reference_block` oracle で照合できる。

### Rule: timestamp/frame_position/capture/playback が正しい

- Given VirtualClockDriver を pump 間で advance
- When pump する
- Then `callback_nanos` が monotonic_nanos に一致、input `capture` =
  callback − latency（provenance `Derived`）、output `playback` =
  callback + latency、`frame_position` が frames 毎に増加、
  `capture_stamp()` が device clock timebase を検証できる。

## Feature: Fault / Oracle

### Rule: StreamFault が index 指定で発火する

- Given `StreamFault::Error/FramePosition/CaptureNone/Frames/
  Invalidate` を `at_callback` 指定で注入
- When pump を順に実行
- Then 指定 index の callback のみで期待される error/position/
  capture/frames が観測され、`PumpReport.errors_fired`/
  `invalidated`/`reported_frame_position`/`gap_frames` が一致する
  （Expected vs Observed）。

### Rule: Invalidate → reconnect

- Given `Invalidate` fault または vanish
- When pump する
- Then `StreamInvalidated` が通知され stream が死亡、以降の pump は
  `Err(StreamInvalidated)`。drop/再 open（必要なら reappear）で
  新 stream が正常に pump できる。

### Rule: ClockDiscontinuity + position jump

- Given `FramePosition{position: 小さい値}` + `Error{ClockDiscontinuity}`
- When pump する
- Then consumer が非単調/巻き戻った `frame_position` と
  `ClockDiscontinuity` を観測する。

## Feature: RT 境界

### Rule: callback 発火経路は alloc-free

- Given `record_output = false` の output stream / 通常の input stream
- When `pump_*_next` を `assert_no_alloc` window 内で実行
- Then allocation/deallocation が発生しない（`rt_` prefix テスト、
  `mise run test-rt` で収集・実行されることをテスト名で確認）。

## Feature: MIDI 薄 adapter

### Rule: MidiScenario → TimedMidiInput feed

- Given `MidiScenario` と `ClockDomainId`
- When `MidiFeed::new` + `drain_due(upto)`/`pump_into_port`
- Then `(position_ticks, source_sequence)` 順に `TimedMidiInput` が
  emit され、負値・変換不能 cue は `skipped` に計上される。

### Rule: MockPort の fault が観測できる

- Given `MockPort` に feed を pump
- When `inject_dropped`/`fail_next_open` を使う
- Then `MidiReadStats.dropped`/open error が engine 側 `read`/`open`
  経路で観測される。
