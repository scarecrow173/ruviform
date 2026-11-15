---
id: "DESIGN-034XCdHjAQMtFJ2ymLlI2M"
type: "design"
status: "approved"
title: "Issue #629 virtual endpoint probe design"
created: "2026-11-14"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-629-virtual-endpoint-survey.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks:
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
    - "https://github.com/scarecrow173/ruviform/issues/629"
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by:
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
  references:
    - "evidence/rf629/endpoint_probe-windows-local.txt"
    - "evidence/rf629/midi_endpoint_probe-windows-local.txt"
  defers: []
  deferred-by: []
---

# Issue #629 virtual endpoint probe design

## Context

Issue #629 first slice の設計。目的は「仮想 endpoint 候補の調査記録」と「provisioning 判定を行う capability probe PoC」であり、runtime-driven controller・本番 Backend 配線・driver 実装は対象外。

境界の前提:

- probe は **control-path の手動 example** であり、CI では実行しない。先例は `crates/ruviform-audio-device/examples/list_devices.rs`・`play_sine.rs`・`crates/ruviform-midi-device/examples/list_midi_ports.rs`。
- probe は **公開 adapter 境界と enabled windows feature のみ**を使う。`Cargo.toml`/feature 追加なし。OS 固有型は example 内の `#[cfg(windows)]` module に閉じ、公開 API を変更しない。
- probe は **`virtual-os-endpoint` evidence を主張しない**。列挙・format query は streaming 証明ではない（#627 boundary-A 規則）。未 provisioning 環境は `NotRun` で fail-closed。
- `unsafe` は workspace lint 方針どおり module 内 `#![expect(unsafe_code)]` + 各 SAFETY コメント（`src/wasapi_input.rs`・`src/os/windows.rs` の先例）。

## Scope

### In Scope

- `endpoint_probe.rs`（audio）: `IMMDeviceEnumerator` による全 state endpoint 列挙、endpoint id/state、cpal port view との friendly-name join、`IAudioClient` の `GetMixFormat`/`IsFormatSupported`/`GetDevicePeriod`/`GetStreamLatency` query、virtual candidate name heuristic、provisioning verdict。
- `midi_endpoint_probe.rs`（MIDI）: WinMM `midiInGetNumDevs`/`midiOutGetNumDevs` + caps 列挙、input 側 `DRV_QUERYDEVICEINTERFACE`、bidirectional name match による loopback candidate 検出、production `backend().input_ports()`/`output_ports()` view（3rd slice で双方向化）、Windows MIDI Services availability 検出（3rd slice 追加）、provisioning verdict。
- discovery doc（候補比較・capability matrix・コードギャップ・provisioning runbook 方針）。
- doc chain（spec/design/test-spec/plan/task/IR/exp）と `evidence/rf629/` の実行証拠。

### Out of Scope

- `IAudioClient::Initialize`/`Start`・stream open・callback 駆動・signal/MIDI 生成（次 slice の runtime controller）
- ~~`midiOutOpen`/`midiOutShortMsg`/`midiOutLongMsg` 送信経路（crate に存在しない — ギャップとして記録のみ）~~ → **2nd slice `scarecrow173/ruviform#658` で解消**（本 design の Out of Scope は 1st slice 時点の記録として残す。3rd slice の loopback verifier は `docs/designs/issue-629-midi-loopback.md`）
- driver install/uninstall・provisioning 自動化・CI lane・workflow/mise 変更
- SetupAPI/driver 署名の機械的検証（enabled feature 外 — gap として記録）
- `src/` の変更・公開 API 追加・`Cargo.toml`/feature 変更

## Components and Boundaries

### `endpoint_probe.rs`（`ruviform-audio-device` example）

- 入口 `main()` → `print_environment()`（os/arch/family + `cmd /c ver` で Windows build 文字列）→ `#[cfg(windows)] win::run()`。
- `win::run()`: `enumerate_all()`（COM 初期化 → `MMDeviceEnumerator` 生成 → render/capture を `DEVICE_STATEMASK_ALL` で列挙 → 各 endpoint を `describe_endpoint`）→ 行ごと print → `print_verdict` → `print_port_view`（cpal port surface の再列挙）。
- `describe_endpoint`: `GetId`（`CoTaskMem` PWSTR → `ComString` RAII で exactly-once free）・`GetState`・friendly-name join（cpal `DeviceId` Debug 表記 `wasapi:<endpoint-id>` の substring match）・`Activate<IAudioClient>`（失敗は note に記録して継続）・mix format/`IsFormatSupported`/`GetDevicePeriod`/`GetStreamLatency`。
- `IsFormatSupported` は **`ppclosestmatch` に有効な out-pointer を渡す**（`NULL` だと本ホスト build で `E_POINTER` — 実測 quirk）。closest format は `CoTaskMemFree` で解放。
- virtual candidate 判定は **name heuristic**（`VIRTUAL_NAME_HINTS`: vb-cable/vb audio/cable input/cable output/hi-fi cable/virtual audio cable/voicemeeter 等の lowercase substring）。「candidate (unconfirmed)」と表示し provisioning 決定に使わない。
- verdict: endpoint 数・candidate 数・`provisioning: NOT_PROVISIONED|CANDIDATES_OBSERVED`・capability ごとの `observed`/`NotRun`・「audio virtual-endpoint cases: NotRun — enumeration is not `virtual-os-endpoint` evidence」。

### `midi_endpoint_probe.rs`（`ruviform-midi-device` example）

- 同構成。`enumerate_all()` が `midiIn*`/`midiOut*` の caps を列挙し、`pname`（packed struct の `szPname` copy → UTF-16 decode）、`interface_id_in`（`midiInMessage` + `DRV_QUERYDEVICEINTERFACE*`、adapter 先例と同一機構）、bidirectional name match で loopback candidate を立てる。
- **`midiOutMessage` の `DRV_QUERYDEVICEINTERFACE*` は呼ばない** — 本ホストで size=2 + crash を実測（GS Wavetable Synth）。output 側は `interface_id: None` + note で `NotRun` を明示。
- `print_port_view`: `backend().input_ports()` の `info()`（id/name/origin/capabilities）を表示 — `origin=Unknown` がそのまま見える（ギャップの可視化）。**3rd slice refresh**: `backend().output_ports()` の同形式 view を追加（#658 の output enumerator — `provides_timestamps=false`/`clock_domain=None` の正直な宣言が input との対比で見える）。
- **3rd slice 追加**: `print_wms_availability()` — `powershell.exe -NoProfile -NonInteractive -Command` で `Get-Command New-MidiLoopback` と `Get-Service midisrv` を照会し stdout を verbatim 記録。spawn 失敗・非ゼロ終了は `NotRun` note で probe を落とさない（`cmd /c ver` 先例、`#[cfg(windows)]` 内・新規 windows feature なし）。
- verdict: port 数・candidate 数・`provisioning`・「midi virtual-endpoint cases: NotRun — WinMM cannot prove a port is virtual」。**3rd slice refresh**: 「send path は 2nd slice で実装済み、残 gap は provisioned loopback pair（verifier: `tests/loopback.rs`）」に更新 — 1st slice の「crate に send path がない」判定は stale だった。

### OS API 面の使い分け（private adapter 境界の記録）

- Audio probe は cpal ではなく **`windows` crate の COM/WASAPI 面を直接**使う（`wasapi_input.rs` と同じ依存・同じ feature）。理由: endpoint state（not-present/unplugged を含む全 state）と `IMMDevice::GetId` の生 endpoint id は cpal port surface に出ないため。cpal view は join 用の補助。
- MIDI probe は windows-sys の `midiIn*`/`midiOut*` caps/message を直接使う（adapter `os/windows.rs` と同じ機構）。~~`midiOut*` の open/send は行わない（crate に経路なし + 今回の crash 実測を踏まえ query のみに限定）~~ → 2nd slice で open/send 経路が実装され、3rd slice の verifier（`tests/loopback.rs`）が production port API 経由で使用する。probe 自体は引き続き query のみ（open/send しない）。
- いずれも example 内に閉じ、`src/` の公開面・依存方向を変えない。後続 slice で provisioning/controller を実装する際は、これらの OS API 知見を private adapter（例: `ruviform-audio-device` 内の provisioning module、または別 private crate）へ移す判断を行う — 本 slice はその境界を「example = probe、src = production」のまま維持する。

## Data and Control Flow

1. `main` → OS facts 出力（os/arch/family/build）。
2. audio: COM init → enumerator → render/capture endpoint 列挙（全 state）→ per-endpoint describe（id/state/name-join/activate/format queries）→ verdict → port view。
3. MIDI: `midiInGetNumDevs`/`midiOutGetNumDevs` → caps + input interface-id → bidirectional pair 検出 → production port view（input+output）→ WMS availability → verdict（3rd slice で順序更新）。
4. 出力は stdout テキスト。`evidence/rf629/` に保存して discovery/IR/exp から参照。

## Non-Functional Constraints

- **RT 無関係**: probe は control-path のみ。callback・stream・割り込み処理を持たない。
- **fail-closed**: 未 provisioning・未検出・未対応は `NotRun`/`Unsupported`/`NOT_PROVISIONED` であり、Pass を出力する経路が存在しない。
- **副作用最小**: audio は `Activate<IAudioClient>` までで `Initialize` を呼ばない（audio session を作らない）。MIDI は caps/message query のみで open しない。driver install・設定変更・レジストリ書換を行わない。
- **3OS lane**: `#[cfg(windows)]` で全 Win32 を隔離。非 Windows は `NotRun` 出力のみで compile 通過。
- **lint**: workspace clippy（pedantic 含む `-D warnings`）・`unsafe_code` warn → module 内 expect。`clippy::cast_possible_truncation` は caps size 変換箇所に `#[expect]`（adapter 先例）。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| `midiOutMessage` interface-id query が driver 依存で crash し得る | probe 実行自体が落ちる | input-path のみ query に限定し、output は `NotRun` + note で記録（実測済み） |
| `IsFormatSupported` の `ppclosestmatch` 必須挙動が build 依存か不明 | format query が一部環境で `E_POINTER` | closest out-param を常に渡す設計にし、結果をそのまま報告（quirk 自体を evidence 化） |
| name heuristic が仮想 endpoint を誤検出/見逃す | candidate 数が不正確 | 「candidate (unconfirmed)」表記に留め、provisioning 判定の根拠にしない。最終識別は専用 runner での人間確認 + 後続の interface-id/metadata 識別 |
| not-present stale endpoint が大量に出る | 出力が冗長 | 全 state 列挙は hotplug 観測のために必要。stale 行は activate 失敗 note で区別される |
| probe が「列挙できた」ことを過大評価される | false green の種 | verdict が常に `NotRun` を明示し、`virtual-os-endpoint` evidence を主張しない文言を固定 |

## Deferred Design Concerns

| 後送り項目 | 理由 | 再開条件 | 無視した場合のリスク |
| --- | --- | --- | --- |
| runtime-driven controller（provisioning 判定 → stream 駆動 → Expected/Observed 照合） | 本 slice 対象外。#628 Test Runtime の fixture と統合が前提 | #628 完了 + 専用 runner provisioning | probe が手動 PoC のまま残る |
| `midiOut*` 送信経路の adapter 追加 | 公開 API 変更を伴う。loopback send→receive に必須 | 後続 slice で adapter 設計 | MIDI 仮想 endpoint の入出力検証が不能 |
| `MidiPortOrigin` の仮想識別強化 | WinMM が区別を公開しない。heuristic/metadata の選択が要る | loopMIDI/WMS provisioning 後の実測 | 仮想 port を `Unknown` のまま扱い、case 分類が曖昧 |
| SetupAPI/driver 署名の機械的検証 | enabled feature 外。probe は `cmd /c ver` workaround | provisioning runner 側で PowerShell `Get-AuthenticodeSignature` 等を検討 | driver 署名の記録が手動依存 |
| 採用 endpoint の確定 | provisioning 実測が前提 | 専用 runner での probe 再実行 | endpoint 固定の根拠がないまま進む |

## Verification Notes

- [x] `cargo check -p ruviform-audio-device --example endpoint_probe` / `cargo check -p ruviform-midi-device --example midi_endpoint_probe` が Pass（`mise exec -- cargo check`、Windows x86_64）
- [x] 両 example の Windows 実機実行が exit 0 で完走し、`evidence/rf629/` に出力保存（Windows 11 10.0.26300.9974）
- [ ] `mise run fmt-check` / `mise run check|lint|test ruviform-audio-device` / `mise run check|lint|test ruviform-midi-device` / `mise exec -- prek run --all-files`
- [ ] 非 Windows lane での compile/`NotRun` 出力（3OS CI で確認 — ローカルは Windows のみ）
