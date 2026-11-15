---
id: "TSPEC-034XVm1QwVvUtSrTpQpO"
type: "test-spec"
status: "approved"
title: "Issue #629 MIDI loopback round-trip verifier test spec"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-midi-loopback.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-629-midi-loopback.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-midi-loopback.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-midi-loopback.md"
    - "docs/specs/issue-629-midi-loopback.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-midi-loopback.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-midi-loopback.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-629-midi-loopback.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI loopback round-trip verifier test spec

## Purpose

`midi_endpoint_probe` の refresh（production output view・verdict・
WMS availability）と `tests/loopback.rs` の round-trip verifier が
満たすべき契約を固定する。verifier は `#[ignore]`-gated の手動
real-device lane であり、その「正しさ」は (a) build/lint を通る
こと、(b) pair 未 provision 環境で fail-closed の NotRun を記録
すること、(c) pair 存在時に独立期待値との内容・順序照合と timestamp
観測を行うこと、(d) `virtual-os-endpoint` evidence を主張しない
ことで定義される。

## Feature

MIDI Loopback Round-trip Verifier — production `midi_input`/
`midi_output` port 経由の send→receive 実 OS 往復を、provisioned
loopback pair 前提で検証する手動 real-device test + probe refresh。

## Rules

- **R1 pair 特定**: verifier は production `output_ports()` /
  `input_ports()` を列挙し、`info().name` が一致する port を
  双方向に探す（probe の loopback candidate signature と同一規則）。
  id 体系が方向で異なるため name match のみを使う。
- **R2 fail-closed NotRun**: backend 列挙の `Err`/空、name match
  0 件、非 Windows（`Unsupported`）では assert せず eprintln verdict
  + early return。Skip=Pass にしない — pair があるのに round-trip
  が成立しない場合のみ Fail（panic）。
- **R3 独立 Oracle**: 期待列はリテラル構築の固定 `MidiEvent` 列
  （`MidiValue::from_u7`/`bend_from_u14` で wire 幅から canonical
  化）であり、被試験コード（adapter の encode/decode）が期待値を
  生成しない。照合は観測列に対する順序保存の部分列一致。
- **R4 送信語彙**: 期待列は NoteOn/ControlChange/PitchBend/NoteOff
  の channel-voice 4 event（ch3・非 center bend — 値の往復を検証）。
  `send_sysex` は本 verifier の対象外（SysEx は input 側で
  `unsupported` 計数される語彙外 traffic）。
- **R5 timestamp 観測**: input port が `provides_timestamps` を宣言
  する場合、観測 event 全件で宣言 `clock_domain`・`Carried`
  provenance・非減少 `ticks` を assert する（`ports.rs` 先例）。
  tick 差は driver-domain 観測値として記録し、send wall-clock との
  絶対 latency は主張しない。
- **R6 証拠境界**: verifier の Pass は「実 OS API を通る往復が
  観測できた」OS-L1 実機証拠であり、`environment_kind=
  virtual-os-endpoint` の Pass ではない — name match は virtual の
  証明ではない（`MidiPortOrigin::Unknown`）。pair が仮想 endpoint
  であることの確認は provisioning 記録側の責務。
- **R7 probe 契約**: probe は production view を input+output 両方向
  で同形式表示し、verdict は「send path は実装済み・残 gap は
  provisioned pair」を正しく述べ、`NOT_PROVISIONED`/`NotRun` の
  fail-closed を維持する。WMS availability は best-effort で失敗時
  `NotRun` note（probe を落とさない）。
- **R8 lane 収集**: 新規 `#[ignore]` test は `test-midi-real-device`
  lane が自動収集する（mise.toml 変更不要）。通常 `cargo test` /
  `mise run test` では実行されない。
- **R9 silent smoke**: `real_device_silent_output_smoke` は
  `"GS Wavetable"` を name に含む output port へ ProgramChange/
  ControlChange/NoteOff のみ送信（NoteOn なし = 無音）し、send の
  `Ok` を assert する。device 不在は NotRun。

## Examples

- **E1 pair 未 provision（本ホスト実測 — 予定）**: Given WinMM
  output に GS Wavetable Synth のみ・input 0 port、When
  `loopback_round_trip` を `--ignored` で実行、Then name match 0 件で
  `verdict: NotRun` を記録して正常終了（assert なし）。
- **E2 loopback pair provisioned（将来）**: Given loopMIDI/WMS loopback
  稼働中に同名 in/out pair が列挙される、When `loopback_round_trip`
  を実行、Then 4 event が順序どおり観測され domain/provenance/ticks
  が宣言どおりで verdict に pair 情報と観測値が出力される。
- **E3 pair あるが往復しない（将来）**: Given name match するが
  実際には loop しない port 組、When 実行、Then timeout 後に観測列を
  dump して panic（Fail — 正直な失敗）。
- **E4 非 Windows**: `output_ports()`/`input_ports()` が
  `Err(Unsupported)` → NotRun early return。compile は全 OS で通る。
- **E5 probe refresh**: `midi_endpoint_probe` 実行で production
  output ports が表示され、verdict が「send path 実装済み・残 gap は
  provisioned pair」を述べ、WMS availability 行が出力される
  （失敗時は `NotRun` note）。

## Guarantees

- verifier は pair 未 provision で Pass を主張しない（fail-closed）。
- pair 存在時の round-trip は独立期待値との順序保存部分列一致で
  判定し、timestamp は宣言 domain/provenance/非減少 ticks を検証。
- probe は両方向の production view を表示し verdict が #658 後の
  実装と一致する。
- 新規 `#[ignore]` test は `test-midi-real-device` lane の収集対象。
- 3OS lane の compile/test が green（非 Windows は NotRun）。

## Non-goals

- 実 provisioning・`virtual-os-endpoint` evidence・reconnect/xrun/
  permission 再現・WASAPI 側仮想 endpoint・Test Runtime 結線・
  CI lane 統合・consumer 照合 — 後続 slice。
- `MidiPortOrigin::Virtual` 昇格・WMS endpoint id・pair 選択の
  引数化。
- `MidiScenario`（audio-test）の port 直接送信への転用。

## Risk

- name match が同名の無関係 port を pair と誤認 → round-trip が
  Fail になるが、それは「往復しない」正直な結果であり false Pass
  ではない。verdict に全 candidate を列挙して人間判別を可能にする。
- loopback が event を変換する実装（UMP 翻訳等）→ Fail として
  観測列 dump が残り、仕様相違の発見材料になる。
- GS Wavetable が無い環境で silent smoke が NotRun に留まる →
  device 不在を Fail にしない設計で吸収。
