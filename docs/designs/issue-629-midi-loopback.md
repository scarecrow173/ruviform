---
id: "DESIGN-034XVm5dXwieJHapZCsdrZ"
type: "design"
status: "approved"
title: "Issue #629 MIDI loopback round-trip verifier and probe refresh design"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-midi-output.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-629-midi-loopback.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-midi-loopback.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/specs/issue-629-midi-output.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-midi-loopback.md"
    - "docs/test-specs/issue-629-midi-loopback.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-output.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by:
    - "docs/plans/issue-629-midi-loopback.md"
    - "docs/test-specs/issue-629-midi-loopback.md"
  verifies: []
  verified-by: []
  references:
    - "evidence/rf629/midi_endpoint_probe-windows-local.txt"
  defers: []
  deferred-by: []
---

# Issue #629 MIDI loopback round-trip verifier and probe refresh design

## Context

Issue #629 の 3rd slice。2nd slice（`scarecrow173/ruviform#658`、merge
`7e7270f`）で `midi_output` port + WinMM `midiOut*` backend
（`src/os/windows_output.rs`）が main に入った。1st slice の probe
（`examples/midi_endpoint_probe.rs`）はこの時点で 2 箇所 stale に
なった: production view が input のみ、verdict が「send path 不在」
を前提にしている。本設計は probe の refresh と、provisioned loopback
pair 前提の round-trip verifier を定義する。

利用する既存契約（調査済み）:

- `ruviform_midi_device::backend()` → `SystemBackend` —
  `MidiInputEnumerator::input_ports` /
  `MidiOutputEnumerator::output_ports` の両 port を `Box<dyn …>` で返す。
- input port: `info()`（`id`/`name`/`origin`/`capabilities` —
  `supports_ump=false`、`provides_timestamps=true`、per-open
  `clock_domain`）、`open()` → `MidiInput::read(&mut Vec,
  max_events) -> MidiReadStats`、`close()`。event は
  `TimedInputEvent<MidiEvent>`（`stamp.ticks` = `midiInStart` からの
  ms、`domain` = port 宣言 domain、`provenance = Carried`）。
- output port: `info()`（`id = "winmm-out:{index}:{name}"`、
  `provides_timestamps=false`）、`open()` → `MidiOutput::send` /
  `send_sysex` / `close`。
- port identity: input は interface-id または `winmm-fallback:{i}:{name}`、
  output は `winmm-out:{i}:{name}` — **id 体系が方向で異なるため
  pair 照合は `info().name` の一致で行う**（probe の loopback
  candidate 検出と同じ signature）。
- `MidiEvent`/`MidiKind`/`MidiValue`（`ruviform-media-core`）—
  channel-voice 語彙。canonical 値は `from_u7`/`bend_from_u14` で
  wire 幅から構築すれば encode→decode で恒等（codec unit test で
  固定済み）。
- `tests/ports.rs::real_device_open_read_observe_close` と
  `tests/output_ports.rs::real_device_send_smoke` — `#[ignore]` +
  early-return-on-empty の real-device lane 先例。
- `mise.toml` `test-midi-real-device` — `-p ruviform-midi-device
  -p ruviform-audio-plugin-bridge -- --ignored --test-threads=1` で
  `#[ignore]` を自動収集（変更不要）。

重要な設計上の発見・制約:

- **WinMM は仮想/物理を区別しない**（`MidiPortOrigin::Unknown`）。
  name match は「loopback-style candidate」であり、pair が見つかって
  もそれが仮想 endpoint である証明にはならない — verifier の Pass は
  OS-L1 の「実 OS API 往復」証拠であり、`environment_kind=
  virtual-os-endpoint` の Pass 主張には provisioning の人間確認が
  別途必要（fail-closed 境界は spec/test-spec で固定）。
- **`midiOutMessage` + `DRV_QUERYDEVICEINTERFACE` は output 側で
  crash 既知**（1st slice exp E2）— verifier は production port
  API のみを使い、Win32 FFI を一切追加しない。
- **非 Windows は `DeviceError::Unsupported`** — verifier は
  cfg 分岐なしで全 OS compile し、非 Windows では列挙 error →
  NotRun early return（3OS lane green）。
- **probe の WMS 検出は外部プロセス呼出** — `powershell.exe
  -NoProfile -NonInteractive -Command …`。`cmd /c ver` 先例どおり
  出力を verbatim 記録し、失敗は `NotRun` note であって probe を
  落とさない。

## Scope

### In Scope

- `examples/midi_endpoint_probe.rs`: `print_port_view` の output 側
  追加、verdict 文言の refresh、`print_wms_availability` 追加、
  module doc の更新。
- `tests/loopback.rs` 新設: `loopback_round_trip`（`#[ignore]`）+
  `real_device_silent_output_smoke`（`#[ignore]`、GS Wavetable
  Synth 名指しの非可聴 send）。
- `crates/ruviform-audio-test/src/midi.rs`: 冒頭 doc comment の
  stale 修正（「production MIDI output port は存在しない」→ #658 後
  の実装に合わせ、sink capture が scheduler 駆動 trace の記録点
  である理由を明確化）。
- survey docs 3 件の更新（discovery gap #1 resolved・probe 出力
  契約 output view）+ evidence。

### Out of Scope

- spec の Out of Scope と同一（実 provisioning・`virtual-os-endpoint`
  evidence・reconnect/xrun・WASAPI・Test Runtime 結線・CI 統合・
  origin 昇格・consumer 照合・`MidiScenario` 転用・非 Windows
  loopback）。

## Components and Boundaries

### `midi_endpoint_probe.rs` refresh

- `print_port_view()` — input 側の既存表示に加え、
  `backend().output_ports()` を同形式で表示:
  `production output ports: N` + 各 port の
  `- {name}  id={id}  origin={origin:?}  ump={} timestamps={} domain={:?}`。
  output は `provides_timestamps=false`/`clock_domain=None` が正直な
  宣言として見える（input との対比が capability の可視化になる）。
- `print_wms_availability()` — `powershell.exe -NoProfile
  -NonInteractive -Command` で `Get-Command New-MidiLoopback` と
  `Get-Service midisrv` を照会し、stdout を verbatim 出力。spawn
  失敗・非ゼロ終了は `NotRun — <reason>` note。`#[cfg(windows)]`
  module 内のみ。
- `print_verdict()` — 「`midiOut*` send path は crate に存在しない」
  系の記述を「send path は 2nd slice で実装済み、残 gap は
  provisioned loopback pair での round-trip 実証（verifier:
  `tests/loopback.rs`）」に更新。「open/stream proof は next slice」
  の記述を「verifier は存在するが pair 未 provision では NotRun —
  `virtual-os-endpoint` evidence は依然主張しない」に更新。
- module doc — 「cross-checks … input enumeration」を両方向に修正、
  「streaming proof is the next slice」を「round-trip verifier は
  `tests/loopback.rs` に存在（provisioned pair 前提）」に更新。

### `tests/loopback.rs` — loopback round-trip verifier

`#[test] #[ignore = "requires a provisioned MIDI loopback pair; manual opt-in check"]`
の 1 本 + 補助の silent-send smoke 1 本。cfg 分岐なし（port trait
経由のみ — OS 固有型・Win32 FFI を持ち込まない）。

`loopback_round_trip` の流れ:

1. `backend().input_ports()` / `output_ports()` を列挙。
   `Err`/空は eprintln + early return（NotRun）。
2. **pair 特定**: 各 output port の `info().name` と一致する input
   port を探す（双方向 name match = probe の loopback candidate
   signature と同一規則）。candidate が複数あれば先頭を使い、全
   candidate を verdict に列挙。0 件なら `verdict: NotRun —
   NOT_PROVISIONED` を eprintln して return（assert しない）。
3. **open**: input port → `open()`、output port → `open()`
   （input を先に開き send 前の取りこぼしを防ぐ）。open 失敗は
   `expect` ではなく verdict 付き panic（実 device があるのに
   open できないのは Fail であり NotRun ではない）。
4. **送信**: 固定イベント列を `output.send()` で順に送る。各送信の
   間に ~30ms sleep（WinMM shortmsg は driver queue 経由で非同期
   到達 — loopback 転送の時間差を吸収）。
   期待列（独立リテラル — `MidiValue::from_u7` /
   `bend_from_u14` で wire 幅から canonical 構築）:
   - `NoteOn` ch3 key60 velocity `from_u7(100)`
   - `ControlChange` ch3 controller 10 value `from_u7(90)`
   - `PitchBend` ch3 `bend_from_u14(0x2A00)`（center 以外 —
     値の往復を検証）
   - `NoteOff` ch3 key60 velocity `from_u7(64)`
5. **drain**: 最大 ~3 秒・`read(&mut out, 64)` を繰り返し、観測
   event を蓄積。期待数に達するか timeout で打ち切り。
   `MidiReadStats.dropped`/`unsupported` を累積記録。
6. **照合**: 観測列に期待列が **順序を保つ部分列** として現れるか
   を検査（loopback が余分な traffic を載せる可能性を許容しつつ、
   順序・内容の一致を要求）。見つからなければ観測列を dump して
   panic（Fail — pair があるのに round-trip しないのは正直な失敗）。
   見つかれば stamp 検証へ。
7. **stamp 観測**: input port が `provides_timestamps` を宣言する
   場合、観測 event 全件で `stamp.domain == 宣言 domain`・
   `provenance == Carried`・`ticks` 非減少を assert（`ports.rs`
   先例）。先頭/末尾 event の `ticks` 差を loopback 往復の
   driver-domain 観測値として verdict に記録（絶対 latency 保証
   ではない — domain は `midiInStart` 起点の ms で send 側 wall
   clock と直接比較不能）。
8. **verdict**: pair の name/id、送信数・観測数・一致結果、
   dropped/unsupported、tick 範囲、経過時間を eprintln で出力
   （`evidence/rf629/` へ転記可能な形）。

`real_device_silent_output_smoke`:

- output port のうち `info().name` が `"GS Wavetable"` を含むものを
  探し（大文字小文字を区別しない contains）、無ければ NotRun
  early return。見つかれば open → `ProgramChange` +
  `ControlChange` + `NoteOff`（NoteOn を送らない = 無音）を send →
  close。send の `Ok` を assert — 「実 WinMM `midiOut*` 経路が
  実 device を通る」ことの OS-L1 実機証拠。loopback pair の有無に
  依存しない。

### Data and Control Flow

```
test (control thread)
  ├─ backend().input_ports() / output_ports()      … production enumerators
  ├─ name match → (in_port, out_port) pair
  ├─ in_port.open() → Box<dyn MidiInput>
  ├─ out_port.open() → Box<dyn MidiOutput>
  ├─ for ev in EXPECTED: out.send(ev); sleep(30ms)
  ├─ loop ≤3s: input.read(&mut buf, 64) → accumulate + stats
  ├─ subsequence match EXPECTED ⊆ observed (order preserved)
  ├─ stamp invariants (declared domain / Carried / non-decreasing)
  └─ eprintln verdict → evidence/rf629/ 転記
```

probe 側は `main` → `print_environment` → `win::run()`:
`enumerate_all`（WinMM caps）→ `print_port_view`（production
input+output view）→ `print_wms_availability` → `print_verdict`。

## Non-Functional Constraints

- **fail-closed**: pair 未検出・backend 不在・非 Windows は全て
  NotRun early return であり、Pass を出力する経路は pair 検出後の
  照合成功のみ。`virtual-os-endpoint` を主張しない。
- **RT 非該当**: verifier/probe は control-path のみ。callback・
  stream・割り込み処理を持たない。
- **3OS lane**: verifier は trait 経由のみで cfg 不要 — 非 Windows
  は `Unsupported` → NotRun。probe の Win32/PowerShell は
  `#[cfg(windows)]` module 内。
- **lint**: workspace clippy（pedantic + `-D warnings`）。
  `unsafe` 追加なし（verifier は safe code のみ、probe の既存
  `#![expect(unsafe_code)]` module 内に WMS 呼出を追加するのみで
  新規 unsafe block は増やさない — `Command` は safe API）。
- **依存**: 新規依存・feature 追加なし。`Cargo.toml`/`Cargo.lock`/
  `mise.toml` 変更不要。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| name match が偽 pair を拾う（同名の無関係 port） | round-trip が失敗し Fail に見える | 実際に send→receive が成立しなければ Fail が正直な結果。candidate 全列挙で人間が判別可能 |
| loopback が event を変換・間引く（WMS MIDI 2.0 loopback の UMP 変換等） | 内容照合が Fail | pitch bend は wire-representable 値を選び恒等を期待。変換が観測されたら verdict に記録し仕様相違として報告（verifier は発見器でもある） |
| WinMM timestamp domain が send wall-clock と比較不能 | latency を誤読 | ticks 差のみ記録し「driver-domain 観測値」と明記。絶対 latency は主張しない |
| GS Wavetable Synth が環境に無い | silent smoke が NotRun | name 不一致は NotRun early return — device 不在を Fail にしない |
| `powershell.exe` が無い/ポリシーで塞がれる環境 | WMS 検出が NotRun | spawn 失敗は note のみで probe は完走 |

## Deferred Design Concerns

- 実 provisioning（loopMIDI/WMS）と `virtual-os-endpoint` evidence —
  後続 slice。再開条件: 専用 runner での provisioning 確立。
- pair 複数存在時の選択ポリシー（現状: 先頭 + 全列挙）— provisioning
  実測後に必要なら env var/引数化を検討。
- 仮想 endpoint 経由の reconnect/xrun/permission（AC5）・WASAPI
  実走（AC1）・Test Runtime（#628）結線・consumer 照合（AC7）—
  後続 slice / 別 Issue。
- `MidiPortOrigin::Virtual` 昇格・WMS endpoint id — API 判別手段の
  確立が前提。
- audio 側仮想 endpoint（VB-CABLE/VAC）の probe/verifier 対応 —
  `endpoint_probe.rs` は別系統であり本 slice 対象外。

## Verification Notes

- [ ] `cargo check`/`clippy`/`test`（mise task 経由）が Pass —
  `mise run fmt-check`、`mise run check|lint|test
  ruviform-midi-device`、`mise run check|lint|test
  ruviform-audio-test`（doc comment のみの変更だが直接影響先として
  検証）。
- [ ] `midi_endpoint_probe` の Windows 実機再実行が exit 0 で完走し、
  `evidence/rf629/midi_endpoint_probe-windows-local.txt` を更新。
- [ ] `mise run test-midi-real-device` の実機実行で、pair 未 provision
  の本ホストでは loopback verifier が NotRun を記録し、silent-send
  smoke と既存 ignored test の結果を `evidence/rf629/` に保存。
- [ ] 非 Windows lane の compile/`NotRun` 出力（3OS CI で確認 —
  ローカルは Windows のみ）。
