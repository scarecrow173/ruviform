---
id: "TSPEC-TnJfu6sTEzaQgA5YPfDd7e"
type: "test-spec"
status: "approved"
title: "Issue #565 WASAPI input frame_position test spec"
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
    - "docs/designs/issue-565-wasapi-input-frame-position-design.md"
  implemented-by: []
  depends-on: []
  blocks: []
  related:
    - "docs/test-specs/issue-187-input-port-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
---

# Issue #565 test spec: WASAPI input `frame_position`

## Feature: per-packet device_position → `InputCallbackInfo::frame_position`

### Rule T1-1: frame_position は packet 先頭 frame の device position

`IAudioCaptureClient::GetBuffer` の `devicePosition`（packet 先頭 frame の device frame position）を `Some(device_position)` として `InputCallbackInfo::frame_position` に載せる。値は PCM を届ける同一 stream・同一 packet の backend 報告値であり、合成・補正しない。

- **Example**: `GetBuffer` が `device_position=4096, frames=480` を返した packet は `frame_position=Some(4096)` を持つ。

### Rule T1-2: 連続 packet は frame 数分だけ増加

loss-free な連続 packet では `device_position` が前 packet の `device_position + frames` に一致する。`PositionTracker` はこの不変条件を検査し、不一致を discontinuity として検出する。

- **Example**: `expected=Some(4576)` に対し `device_position=4800`（480 frames 欠落）→ `ClockDiscontinuity` を error callback に通知し、tracker は `4800+frames` に再同期する。

### Rule T1-3: discontinuity は `DeviceError::ClockDiscontinuity`

`AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY`（`device_position != 0` ガード付き、first packet の未定義 flag 対策）または position jump は `DeviceError::ClockDiscontinuity` に写像する。cpal が `Xrun` に混同するのとは異なり port が設計した variant を使う。packet 自体は実位置つきで配送を継続する。

### Rule T1-4: first packet / stop-start の epoch 境界

`expected == None`（stream open 後最初の packet、または stop→start 後の再初期化が必要な場合）は検査を skip して `expected = device_position + frames` に初期化する。first packet `device_position == 0` は正常値として扱う。

- **注**: stop/start 後に WASAPI が position を reset する場合、contiguity 検査が `ClockDiscontinuity` を報告する — consumer 起点の境界でも position epoch が切れた事実を正直に通知する設計。

### Rule T1-5: timestamp 契約は不変

`callback` は `IAudioClock::GetPosition` の qpc（100ns→ns→MediaTime）、`capture` は `GetBuffer` の `qpc_position`（`0` は未報告 sentinel → `None`）、`clock_domain` は host profile（WASAPI=`MONOTONIC`）、capture provenance は `Carried`。chunk 分割時は既存 `chunk_input_timestamp_info` が `frame_position` を `saturating_add(frame_offset)` で進める。

### Rule T1-6: 非 WASAPI / 供給不能 backend は `None`

`frame_position` を報告できない backend（非 Windows、非 WASAPI host、将来の cpal 経路）は従来どおり `None` — 合成しない。

## 検証方法

| 層 | 方法 |
| --- | --- |
| headless unit | `wasapi_input.rs` `#[cfg(test)]`: `PositionTracker` 境界、packet→`InputCallbackInfo` 構築（`frame_position=Some`、capture `Carried`）、`map_hresult` の AUDCLNT→`DeviceError` 写像、`WAVEFORMATEXTENSIBLE` 構築 |
| RT | `rt_wasapi_packet_path_allocates_nothing` — `assert_no_alloc` で packet→info→`read_interleaved` が非 alloc（Windows 限定 compile のため `test-rt` ubuntu lane ではなく local Windows 実行を証拠にする） |
| contract | `tests/mock_input_backend.rs` 既存の `Some(position)+gap` port-level 契約 test は変更なしで green を維持 |
| 実機 | `tests/real_input_e2e.rs`: WASAPI host で `FRAME_POSITION_SOME > 0` かつ position が `pos+frames` で連続増加することを assert（`test-audio-real-input`、実機ある場合のみ） |
