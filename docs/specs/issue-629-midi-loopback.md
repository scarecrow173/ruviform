---
id: "SPEC-034XVlYdozn65zqMmYFFb5"
type: "spec"
status: "approved"
title: "Issue #629 MIDI loopback round-trip verifier and probe refresh (3rd slice)"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-midi-output.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-629-midi-loopback.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-629-midi-loopback.md"
    - "docs/plans/issue-629-midi-loopback.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-midi-loopback.md"
    - "docs/test-specs/issue-629-midi-loopback.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-midi-output.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-midi-output.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-midi-loopback.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI loopback round-trip verifier and probe refresh (3rd slice)

## What Should Be Built

Issue #629 の 3rd slice。2nd slice（`scarecrow173/ruviform#658`、merge
`7e7270f`）で `midi_output` port + WinMM `midiOut*` backend が main に
入ったことを受け、以下の 2 部構成で stale を解消し loopback round-trip
の検証経路を実装する。

- **probe refresh（`crates/ruviform-midi-device/examples/
  midi_endpoint_probe.rs`）** — 1st slice の probe が持っていた 2 つの
  stale を修正する。(a) production port view を input のみから
  input+output の両方向へ拡張する（#658 の output enumerator
  `info()` を同形式で表示）。(b) verdict 文言を更新する —
  「`midiOut*` send path は存在しない」という判定は #658 で解消済み
  であり、残る gap は provisioned loopback pair での実証のみ、という
  形に直す。(c) Windows MIDI Services（WMS）の利用可否検出を追加する
  （`powershell.exe` 経由の `Get-Command New-MidiLoopback` 系呼出、
  `cmd /c ver` の既存先例に倣う。loopMIDI vs WMS の採用判定材料）。
- **loopback round-trip verifier
  （`crates/ruviform-midi-device/tests/` の `#[ignore]`-gated
  real-device test）** — production output+input port 列挙 → 双方向
  name match で loopback pair を特定 → 両方向 open → 固定イベント列
  （NoteOn/CC/PitchBend/NoteOff）送信 → input `read` drain → 内容・
  順序を独立期待値と照合 → timestamp domain/latency 観測 → verdict
  出力。pair が無い環境では正直な NotRun で fail-closed（Skip=Pass
  にしない）。GS Wavetable Synth 等の実 output への非可聴 send
  （ProgramChange/CC/NoteOff — NoteOn なしで無音）を実 WinMM 通過
  確認として追加してよい。
- **stale doc 修正** — `crates/ruviform-audio-test/src/midi.rs` の
  「production MIDI output port は存在しない」旨の doc comment を
  #658 の実装に合わせる。`docs/discovery/issue-629-virtual-endpoint-
  survey.md` の Known Code Gap #1 を resolved（merge `7e7270f` 参照）
  に更新し、probe refresh の evidence を追記する。
  `docs/designs/`・`docs/test-specs/` の同 survey 文書に probe 出力
  契約（output view）を追記する。

本 slice が直接対象とするのは Issue AC2 の loopback round-trip 検証
経路の実装（provisioned pair 前提の verifier）と、#658 merge 後に
stale になった 1st slice の probe/discovery 記録の refresh。

## Why Now

- 2nd slice で `midiOut*` send path が main に入ったため、1st slice
  の probe verdict（「send path が無いので round-trip は NotRun」）は
  stale になった。残る gap は「provisioned loopback pair での実証」
  のみであり、その判定器・verifier を本 slice で用意する。
- Issue AC2「WinMM を使う #253 実 Backend から仮想 MIDI Port の
  Note/CC/Pitch Bend/Timestamp/Order が入出力できる」の検証経路を、
  provisioning 確立後にそのまま実行できる形で先に実装しておく。
- 仮想優先 POLICIES（2026-11-15 改訂）では、pair 未 provision 環境で
  の verifier 実行は `NotRun` であり、`virtual-os-endpoint` の Pass
  や実 OS L3 Pass と主張しない — 本 slice はその境界を守る。

## Users and Value

- **後続 slice（#629 残受入）**: loopMIDI/Windows MIDI Services の
  provisioning が確立した時点で、verifier を `test-midi-real-device`
  lane 経由で実行するだけで AC2 の round-trip 証拠を取得できる。
- **probe の再利用者**: probe の production view が双方向を映すため、
  provisioning 判定時に output 側の列挙・capability 宣言も同時に
  確認できる。WMS 検出は loopMIDI vs WMS の採用判断材料になる。
- **Ruviform 自身**: `midi_output`/`midi_input` 両 port の組み合わせ
  （send→receive の実 OS 往復）を dogfooding する最初の経路。

## Scope

### In Scope

- `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs` の
  refresh（output port view・verdict 文言・WMS availability 検出）。
- `crates/ruviform-midi-device/tests/` への loopback round-trip
  verifier（`#[ignore]`-gated、既存 `test-midi-real-device` lane が
  自動収集）。
- `crates/ruviform-audio-test/src/midi.rs` の stale doc comment 修正
  （このファイルのみ — audio-test の他ファイルは別 Worker の占有
  範囲）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` の Known Code
  Gap #1 resolved 更新と evidence 追記、
  `docs/designs/`・`docs/test-specs/` 同 survey 文書への probe 出力
  契約追記。
- docs chain（spec/design/plan/task/test-spec/impl ir・exp）と
  `evidence/rf629/` の検証記録。

### Out of Scope

- loopMIDI/Windows MIDI Services の実 provisioning と
  `virtual-os-endpoint` evidence の取得（本 slice は verifier の実装
  と未 provision 環境での fail-closed NotRun 記録まで）。
- 仮想 endpoint 経由の reconnect/xrun/permission 再現（AC5 本体）・
  WASAPI 側仮想 endpoint 実走（AC1）。
- Test Runtime（#628）との結線・本番 pipeline 配線・User-mode
  controller/runner。
- 専用 Windows runner の CI/手動タスク統合（`.github/**`・
  `mise.toml` は範囲外 — `test-midi-real-device` lane は既存の
  `--ignored` 自動収集で足りる）。
- `MidiPortOrigin::Virtual` 昇格（仮想 port の API 判別 — 対象外）。
- Synvelis `scarecrow173/synvelis#115`/`#117`/`#126`・AudioLens
  `scarecrow173/audio-lens#127` の consumer 照合（AC7）。
- `MidiScenario`（audio-test）の port 直接送信への転用 — 期待列の
  形式言語のみ共有し、TransportScheduler 駆動の scenario 機構は
  再利用しない。
- macOS/Linux の loopback 検証（非 Windows は `Unsupported`/NotRun で
  fail-closed）。

## Requirements

- probe は引き続き手動 example であり CI で実行しない。production
  port view は `backend().input_ports()` と `backend().output_ports()`
  の両方向を同形式（id/name/origin/capabilities）で表示する。
- probe の verdict は fail-closed を維持する — provisioning 未確立・
  pair 未検出の環境では `NotRun`/`NOT_PROVISIONED` を報告し、
  `virtual-os-endpoint` evidence を主張する経路を持たない。
- Win32 依存の追加（WMS 検出の PowerShell 呼出を含む）は
  `#[cfg(windows)]` の後ろに置き、非 Windows では compile 通過 +
  `NotRun` 出力のみ。
- verifier は `#[ignore]`-gated で `test-midi-real-device` lane の
  収集対象とし、loopback pair が無い環境では assert せず正直な
  NotRun（eprintln verdict + early return）で終わる。Skip=Pass に
  しない。
- verifier の期待列は独立した固定値（リテラル構築）であり、被試験
  コード（adapter の encode/decode）が期待値を生成しない
  （自己参照 Oracle 禁止）。
- timestamp 検証は input port の宣言 domain・`Carried` provenance・
  非減少 driver ticks を既存 `ports.rs` 先例どおり確認し、観測値は
  verdict/evidence に記録する（latency の絶対値保証はしない）。
- クレート外から到達可能な全公開 enum に enum レベルの
  `#[non_exhaustive]`（本 slice は公開 API を追加しない見込み —
  追加する場合は遵守）。
- 実行していない検証を Pass と書かない。`environment_kind` 証拠は
  Case schema 語彙（`deterministic_backend`/`os_virtual_endpoint` 等）
  で正直に記録する。

## Acceptance Criteria

- [ ] `midi_endpoint_probe` が production output port view を表示し、
  verdict が「send path は #658 で存在、残 gap は provisioned pair」
  を正しく述べる。Windows 実機で再実行し
  `evidence/rf629/midi_endpoint_probe-windows-local.txt` を更新する。
- [ ] WMS availability 検出が probe 出力に含まれる（失敗時は NotRun
  として記録、probe 全体は落ちない）。
- [ ] `tests/` の loopback round-trip verifier が、provisioned pair
  存在時に Note/CC/PitchBend/NoteOff の内容・順序を独立期待値と照合
  し、timestamp domain/provenance/ticks を観測・記録する。
- [ ] pair 未 provision 環境では verifier が fail-closed の NotRun を
  記録し、`virtual-os-endpoint` Pass を主張しない。
- [ ] `crates/ruviform-audio-test/src/midi.rs` の stale doc comment が
  #658 後の実装に一致する。
- [ ] discovery/design/test-spec の survey 文書で Known Code Gap #1 が
  resolved と記録され、probe 出力契約（output view）が追記される。
- [ ] Issue #629 の残受入（実 provisioning・`virtual-os-endpoint`
  evidence・reconnect/xrun・WASAPI・consumer 照合）は本 slice 対象外
  として docs に明記される。

## Deferred Design Concerns

- loopMIDI/WMS の実 provisioning と `virtual-os-endpoint` evidence —
  後続 slice（専用 runner 運用が前提）。再開条件: provisioning 確立。
- 仮想 endpoint 経由の reconnect/xrun/permission（AC5）・WASAPI 実走
  （AC1）・Test Runtime（#628）結線・consumer 照合（AC7）— 後続
  slice / 別 Issue。
- `MidiPortOrigin::Virtual` 昇格・Windows MIDI Services endpoint id —
  API 判別手段の確立が前提。
- 実体は `docs/designs/issue-629-midi-loopback.md` の Deferred Design
  Concerns に記録する。

## Sources and Relations

- `scarecrow173/ruviform#629`（本 Issue、open のまま — PR は `Refs`）、
  `scarecrow173/ruviform#627`（Closed — Case Schema v1・
  `environment_kind`・Test Depth の契約元）、
  `scarecrow173/ruviform#658`（2nd slice merge `7e7270f` — `midiOut*`
  send path）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`（1st slice 成果
  — capability matrix・gap #1・`midiOutMessage` crash 記録 E2）、
  `docs/specs/issue-629-midi-output.md` + `docs/designs/` +
  `docs/plans/` + `docs/tasks/` + `docs/test-specs/`（2nd slice
  chain — output port 契約の正本）。
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
  （Case schema・`environment_kind` taxonomy の正本）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・§5（ローカル検証と
  Daily CI の責務分離）。
- 先例: `crates/ruviform-midi-device/tests/ports.rs` の
  `real_device_open_read_observe_close`（`#[ignore]` real-device
  lane 先例）、`tests/output_ports.rs` の `real_device_send_smoke`、
  `examples/midi_endpoint_probe.rs`（1st slice probe）、
  `examples/endpoint_probe.rs`（audio probe 先例）。
