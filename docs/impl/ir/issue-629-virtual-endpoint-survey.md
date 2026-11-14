---
id: "IMPL-034XCe9Yszn1tqayGEjYYk"
type: "impl"
status: "completed"
title: "Issue #629 virtual endpoint probe implementation record"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/tasks/issue-629-virtual-endpoint-survey.md"
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
  changes:
    added:
      - type: "code"
        path: "crates/ruviform-audio-device/examples/endpoint_probe.rs"
      - type: "code"
        path: "crates/ruviform-midi-device/examples/midi_endpoint_probe.rs"
      - type: "doc"
        path: "docs/discovery/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/specs/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/designs/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/plans/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/tasks/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/test-specs/issue-629-virtual-endpoint-survey.md"
      - type: "evidence"
        path: "evidence/rf629/endpoint_probe-windows-local.txt"
      - type: "evidence"
        path: "evidence/rf629/midi_endpoint_probe-windows-local.txt"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-629-virtual-endpoint-survey.md"
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
    - "docs/impl/exp/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-629-virtual-endpoint-survey.md"
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  derived-by:
    - "docs/impl/exp/issue-629-virtual-endpoint-survey.md"
  verifies:
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
  verified-by: []
  references:
    - "evidence/rf629/endpoint_probe-windows-local.txt"
    - "evidence/rf629/midi_endpoint_probe-windows-local.txt"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #629 virtual endpoint probe implementation record

## Summary

Issue #629 first slice（investigation + capability-probe）を実装した。仮想 audio/MIDI endpoint 候補の調査を `docs/discovery/` に記録し、Windows 実 OS API 面を読む手動 capability probe を `ruviform-audio-device` / `ruviform-midi-device` の `examples/` に追加した。両 probe は未 provisioning 環境を `NotRun`/`NOT_PROVISIONED` で fail-closed に報告し、`virtual-os-endpoint` evidence を主張しない。runtime-driven controller・Backend 配線・CI lane・driver 実装は対象外のまま。

## Context

Task `TASK-034XCkcQ3OJUyxLkwM19xs` → Plan `PLAN-034XCkcPRJrWqtaLLfUlAS` → Spec `SPEC-034XCi8anHlk3T9rYA7ysb` / Design `DESIGN-034XCdHjAQMtFJ2ymLlI2M` / TestSpec `TSPEC-034XCdLA9CKhCyJHi0wtK9`。`scarecrow173/ruviform#627` の Case schema / `environment_kind` 語彙と `POLICIES.md` の仮想優先方針に従う。

## Implementation

### Changes

- `crates/ruviform-audio-device/examples/endpoint_probe.rs`（新規）: `IMMDeviceEnumerator` で render/capture endpoint を全 state 列挙。`GetId`/`GetState`/friendly-name join（cpal `wasapi:<id>` Debug 表記との substring match）・`Activate<IAudioClient>`・`GetMixFormat`・`IsFormatSupported`・`GetDevicePeriod`・`GetStreamLatency` を query し、virtual candidate name heuristic と provisioning verdict を出力。非 Windows は `NotRun`。
- `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs`（新規）: WinMM `midiInGetNumDevs`/`midiOutGetNumDevs` + `*GetDevCapsW` で port 列挙、input 側 `DRV_QUERYDEVICEINTERFACE*` で stable id、bidirectional name match で loopback candidate 検出、production `backend().input_ports()` view と突合。非 Windows は `NotRun`。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`（新規）: VB-CABLE/Hi-Fi・VAC・loopMIDI/teVirtualMIDI・Windows MIDI Services・SysVAD の比較、capability matrix、コードギャップ（`midiOut*` 不在・`MidiPortOrigin::Unknown`）、provisioning runbook 方針。
- doc chain 一式 + `evidence/rf629/*-windows-local.txt`。

### Decisions During Implementation

- **audio probe は cpal ではなく `windows` crate の COM/WASAPI 面を直接使用**: endpoint state（not-present 含む全 state）と生 endpoint id は cpal port surface に出ないため。cpal view は friendly-name join 補助として併用。依存・feature 追加なし（`wasapi_input.rs` と同じ enabled feature）。
- **`IAudioClient::Initialize`/`Start` を呼ばない**: streaming session を作らず、副作用最小の query-only probe とした。`GetStreamLatency` は Initialize 前 `AUDCLNT_E_NOT_INITIALIZED` を `NotRun` として報告。
- **`IsFormatSupported` の `ppclosestmatch` に有効な out-pointer を必須化**: `NULL` は本ホスト build で `E_POINTER` を返す実測 quirk。closest format は `CoTaskMemFree` で解放。
- **`midiOutMessage` の `DRV_QUERYDEVICEINTERFACE*` を呼ばない**: 本ホスト（Windows 11 10.0.26300.9974、Microsoft GS Wavetable Synth）で size=2 応答後の fetch が WinMM 内で segfault。input 側のみ query し、output は `interface_id: None` + `NotRun` note。
- **virtual/loopback 判定は name heuristic / bidirectional match に限定し「candidate (unconfirmed)」表記**: API が仮想性を証明しないため provisioning 決定の根拠にしない。
- **verdict は常に仮想 endpoint case を `NotRun`**: 「列挙できた ≠ 使える」を固定し、`virtual-os-endpoint` evidence を主張しない文言を出力に埋め込んだ。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-629-virtual-endpoint-survey.md` — midiOutMessage crash・IsFormatSupported quirk・not-present activate 拒否の実測記録。

### Rejected

- （なし）

## Validation

- `cargo check -p ruviform-audio-device --example endpoint_probe` — Pass（Windows x86_64、補助証拠）
- `cargo check -p ruviform-midi-device --example midi_endpoint_probe` — Pass（同上）
- `cargo run -p ruviform-audio-device --example endpoint_probe` — exit 0、61 endpoint 列挙、virtual candidate 0、`NOT_PROVISIONED`、出力を `evidence/rf629/endpoint_probe-windows-local.txt` に保存
- `cargo run -p ruviform-midi-device --example midi_endpoint_probe` — exit 0、input 0/output 1 port、loopback candidate 0、`NOT_PROVISIONED`、出力を `evidence/rf629/midi_endpoint_probe-windows-local.txt` に保存
- canonical（`mise run fmt-check` / `check|lint|test` 両 crate / `prek run --all-files`）— 実施結果は PR 本文に記録

## Risks

- name heuristic / bidirectional match の識別精度は未 provisioning 環境では未検証。candidate 表記は unconfirmed に留める。
- `midiOutMessage` crash は driver 依存であり、他 driver で安全かは未検証 — output query は当面 `NotRun`。
- 非 Windows lane の compile/`NotRun` 出力はローカル未検証（3OS CI で確認）。

## Follow-ups

- 後続 slice: Test Runtime (#628) 統合の runtime-driven controller、本番 Backend 配線、`midiOut*` 送信経路、`MidiPortOrigin` 識別、専用 Windows runner での provisioning 実測、Synvelis/AudioLens consumer 照合。
- 採用 endpoint 確定は provisioning 実測後の ADR/decision として記録。
