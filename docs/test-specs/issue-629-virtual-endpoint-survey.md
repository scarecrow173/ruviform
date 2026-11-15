---
id: "TSPEC-034XCdLA9CKhCyJHi0wtK9"
type: "test-spec"
status: "approved"
title: "Issue #629 virtual endpoint probe test spec"
created: "2026-11-14"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-629-virtual-endpoint-survey.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  verified-by: []
  references:
    - "evidence/rf629/endpoint_probe-windows-local.txt"
    - "evidence/rf629/midi_endpoint_probe-windows-local.txt"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  defers: []
  deferred-by: []
---

# Issue #629 virtual endpoint probe test spec

## Purpose

`endpoint_probe` / `midi_endpoint_probe` の 2 つの手動 example が満たすべき契約を固定する。これらは CI では実行されない診断ツールであり、その「正しさ」は (a) build/lint を通ること、(b) 非 Windows で `NotRun` を報告して compile が通ること、(c) Windows で実行した際に endpoint を**正直に**報告し fail-closed に振る舞うこと、(d) `virtual-os-endpoint` evidence を主張しないこと、で定義される。後続 slice がこれらを runtime controller の provisioning 判定器として再利用する際の回帰基準でもある。

## Feature

Windows 仮想 endpoint capability probe — OS API 面の列挙・format 交渉 query・provisioning verdict を fail-closed に報告する手動診断 example。

## Rules

- **R-PROBE-01（fail-closed）**: 仮想 endpoint が provisioning されていない環境では、probe は全ての仮想 endpoint case を `NotRun` と報告し、`Pass` を出力する経路を持たない。`provisioning:` 行は `NOT_PROVISIONED` または `CANDIDATES_OBSERVED` のいずれかのみを出力する。
- **R-PROBE-02（非 Windows）**: `#[cfg(not(windows))]` では Win32 API を一切呼ばず、compile が通り `NotRun`（Windows 必須）を報告する。
- **R-PROBE-03（副作用最小）**: audio probe は `IAudioClient::Activate` までで `Initialize`/`Start` を呼ばない。MIDI probe は `midiIn*`/`midiOut*` の caps/message query のみで open/send を呼ばない。driver install・OS 設定変更を行わない。
- **R-PROBE-04（境界維持）**: probe は `src/` の公開 API を変更せず、OS 固有型を example の `#[cfg(windows)]` module 内に閉じる。新規依存・feature 追加を必要としない。
- **R-PROBE-05（RT 非該当）**: probe は control-path のみで callback・stream を持たず、RT 契約の対象外。ただし `unsafe` FFI は SAFETY コメント付き局所許可の先例に従う。
- **R-PROBE-06（evidence 非昇格）**: probe 出力は `environment_kind=virtual-os-endpoint` の Pass evidence として扱わない。列挙・format query の `observed` は「OS API 面が読めた」記録であり、endpoint 経由の streaming 証明ではない。
- **R-PROBE-07（MIDI output query 制限）**: `midiOutMessage` の `DRV_QUERYDEVICEINTERFACE*` は呼ばない（driver 依存 crash を実測）。output port の `interface_id` は `None` + `NotRun` note で報告する。
- **R-PROBE-08（IsFormatSupported 規約）**: `IsFormatSupported` には `ppclosestmatch` に有効な out-pointer を渡す（`NULL` は本ホスト build で `E_POINTER`）。closest format は `CoTaskMemFree` で解放する。
- **R-PROBE-09（3rd slice 追記 — production output view と WMS 検出）**: `midi_endpoint_probe` の production port view は `input_ports()` と `output_ports()` の両方向を同形式（id/name/origin/capabilities）で表示する。verdict は「`midiOut*` send path は 2nd slice で実装済み、残 gap は provisioned loopback pair」と述べ、send path 不在を前提にしない。Windows MIDI Services availability（`New-MidiLoopback` cmdlet / `midisrv` service）は best-effort で報告し、失敗時は `NotRun` note で probe を落とさない。

## Examples

- **Example 1（未 provisioning — 実測済み）**: Given Windows 11 10.0.26300.9974 で VB-CABLE/loopMIDI 未導入、When `endpoint_probe`/`midi_endpoint_probe` を実行、Then audio は 61 endpoint を列挙し virtual candidate 0 件・`provisioning: NOT_PROVISIONED`・全 virtual case `NotRun` を出力、MIDI は input 0/output 1 port・loopback candidate 0 件・`NOT_PROVISIONED` を出力する（`evidence/rf629/*-windows-local.txt`）。
- **Example 2（仮想 endpoint 検出時 — 将来）**: Given VB-CABLE 導入済み runner、When `endpoint_probe` を実行、Then "CABLE Input"/"CABLE Output" が render/capture に `state=active` で列挙され name heuristic で `[virtual-candidate]` が付くが、verdict の virtual case は依然 `NotRun`（streaming 証明は次 slice）のままである。
- **Example 3（loopback 検出時 — 将来）**: Given loopMIDI 稼働中、When `midi_endpoint_probe` を実行、Then 同名 port が midiIn/midiOut 双方に現れ `[loopback-candidate]` が付く。send→receive round-trip は 3rd slice の `tests/loopback.rs` verifier が担い、provisioned pair があれば独立期待値と照合して Pass、無ければ fail-closed `NotRun`（本 probe 自体は open/send しない）。
- **Example 4（非 Windows）**: Given Linux/macOS、When 両 example を build/run、Then compile が通り `NotRun — requires Windows` を出力する。

## Guarantees

- 両 example は `cargo check`/`clippy -D warnings`（workspace lint）を通る。
- 非 Windows で compile が通り `NotRun` を報告する（3OS lane green）。
- Windows で実行しても crash しない（`midiOutMessage` query を含まない）。
- 出力に OS/arch/build・endpoint/port 列挙・provisioning verdict が含まれ、`Pass` を仮想 endpoint case に対して出力しない。

## Non-goals

- 仮想 endpoint 経由の実 streaming・callback・format 交渉の成立確認（次 slice）。
- `virtual-os-endpoint` の `environment_kind` evidence 生成。
- driver install/uninstall・provisioning 自動化・CI lane。
- ~~`midiOut*` 送信経路の実装（crate に存在しない）~~ → 2nd slice `scarecrow173/ruviform#658` で解消。3rd slice は round-trip verifier（`tests/loopback.rs`）を追加 — 実 provisioning・`virtual-os-endpoint` evidence は依然 non-goal。
- SetupAPI/driver 署名の機械的検証。

## Risk

- probe の verdict が「列挙できた」を「endpoint が使える」と誤読されると、後続 slice が streaming 未検証のまま Pass を主張する危険がある — verdict 文言と test-spec R-PROBE-06 で固定する。
- name heuristic / bidirectional match は heuristic であり、実 provisioning 環境での識別精度は未検証 — 「candidate (unconfirmed)」表記を維持し、確定識別は専用 runner での人間確認 + 後続の interface-id/metadata 識別に委ねる。
