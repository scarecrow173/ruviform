---
id: "SPEC-rf253osmidibackend001"
type: "spec"
status: "approved"
title: "Issue #253 OS MIDI input backend slice（Windows 実装 + macOS/Linux Unsupported 境界）"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
  derived-by:
    - "docs/designs/issue-253-os-midi-backend.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-os-midi-backend.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 OS MIDI input backend slice

## 背景とスコープ

Issue #188 で確定した `ruviform-audio-core::midi_input` の 3 trait port
（`MidiInputEnumerator`/`MidiInputPort`/`MidiInput`）と `mock` backend、
および Issue #253 の VST3 正規経路 slice（(b)(c)、PR #555 merge 済み）
に続き、本 slice は Issue #253 の残スコープ (a)「実 OS MIDI backend」
のうち **Windows backend の実装** と **macOS/Linux の Unsupported 境界**
を対象とする。

対象:

- 実 port 列挙（`MidiInputEnumerator::input_ports`）・`MidiInputInfo`
  構築（id/name/origin/capabilities の正直な宣言）。
- `MidiInputPort::open` → `MidiInput`（subscription・bounded queue・
  source timestamp 付き event 生成・非 RT `read` drain・
  `MidiReadStats` 観測）。
- Windows: WinMM（winmm.dll）backend を `windows-sys` 経由で実装。
- macOS/Linux: `cfg` で分離した stub が `DeviceError::Unsupported`
  を返す境界（コンパイルは全 target で通る）。

対象外（Issue #253 に残る・本 slice では Pass に繰り上げない）:

- macOS CoreMIDI / Linux ALSA 等の実 backend 実装
- opt-in 実デバイス lane（mise task 追加）・実 MIDI デバイスでの
  E2E 検証・実 DAW VST3 往復
- host-graph consumer 配線（`InputStage`/`InputTx::pump` 経路への
  adapter 結線は既存経路が担う。adapter 提供面までが本 slice）
- `InputStage` 側の変更、CLAP/L0/L1 adapter 変更
- MIDI2/UMP/MPE の generic capability（Issue #317）

## 要件

### R1 adapter 配置と公開面

- 新規 adapter crate `crates/ruviform-midi-device` を新設する
  （`ruviform-audio-device` 内への同居はしない — 同クレートは
  「cpal を内部依存に閉じ込める audio device adapter」として単一責務
  であり、MIDI は別デバイスドメイン。issue-188 detailed design の
  「adapter crate 新設」方針に一致）。依存方向は adapter 層:
  `ruviform-audio-core`（port trait）+ `ruviform-media-core`
  （event/stamp 型）のみ下向き。engine・他 adapter への依存禁止。
- 公開面は OS backend を返す factory と port 実装に限定し、
  WinMM の型・エラー・feature 名を公開 API に露出しない
  （`ruviform-audio-device` の cpal 隔離と同じ境界原則）。全 backend
  エラーは `DeviceError` に正規化する。

### R2 Windows backend（WinMM）

- `input_ports`: `midiInGetNumDevs` + `midiInGetDevCapsW`（name）+
  `midiInMessage`（`DRV_QUERYDEVICEINTERFACE` による安定 id）で
  現在の port 集合を列挙。空集合は `Ok(vec![])`。
- `open`: port 番号の再解決（interface id 照合、fallback は
  index+name 検証）→ `midiInOpen`（`CALLBACK_FUNCTION | MIDI_IO_STATUS`）
  → SysEx 用 `MIDIHDR` buffer の prepare/add → `midiInStart`。
  失敗は `DeviceError` に正規化（不存在→`DeviceNotFound`、
  占有→`DeviceBusy` 等）。1 port handle は 1 回だけ open 可
  （2 回目は `DeviceBusy`）。
- callback（WinMM driver thread）: `MIM_DATA` を channel-voice
  `MidiEvent` に decode し adapter 内部 bounded queue へ
  timestamp 付きで push。`MIM_MOREDATA`/`MIM_LONGERROR` は
  `dropped` 計数、`MIM_LONGDATA` と非 channel-voice `MIM_DATA`
  （system common/realtime 等）は `unsupported` 計数 — いずれも
  サイレント失敗させない。callback 内で OS API を呼ばない
  （MSDN 制約）— SysEx buffer の requeue は `read` 側で行う。
- timestamp: `dwParam2`（midiInStart からの ms、MSDN 記載の
  「beginning at zero when the midiInStart function was called」）
  をそのまま `InputStamp` に載せる。`TimeBase` = 1/1000、
  `ClockDomainId` は subscription ごとに採番（複数 port 同時 open
  で epoch が衝突しないようにする）、`provenance` = `Carried`。
  engine 側の clock 変換は adapter の責務ではない。
- `read`: 非 RT。queue から `max_events` まで drain、
  `MidiReadStats`（`dropped`/`unsupported` の since-last-read
  カウンタ）を返す。subscription 死亡後は
  `DeviceError::StreamInvalidated`。
- `close`/`Drop`: `midiInStop`+`midiInReset`+`midiInClose`+
  header unprepare+buffer 解放。idempotent。

### R3 capability / provenance の正直な宣言

- `MidiPortCapabilities`: `supports_ump = false`（WinMM は MIDI 1.0
  byte stream）、`provides_timestamps = true`、`clock_domain = Some`
  （subscription ごとの採番 domain）。
- `MidiPortOrigin`: WinMM API では外部/仮想 port を判別できないため
  `Unknown` を返す — port 型に `MidiPortOrigin::Unknown` variant を
  additive に追加する（`#[non_exhaustive]` enum のため既存 consumer
  へ非破壊）。
- `MidiReadStats`: 受信したが channel-voice 語彙に変換できない
  message（SysEx・system common/realtime）の計数を `dropped`
  （queue overflow）と区別するため `unsupported: u64` field を
  additive に追加する。`dropped` の既存意味（内部 queue overflow +
  driver 側喪失通知）は維持する。

### R4 macOS/Linux 境界

- `cfg` で OS 別 module を分離し、非対応 OS では
  `input_ports`/`open` が `DeviceError::Unsupported` を返す
  stub を置く（todo! ではなく明示的 error）。全 target で
  コンパイルが通ること。

### R5 検証

- device 不在環境で全 lane が Pass すること（列挙 0 件は Pass
  条件にしない）。
- 単体テスト: message decode（全 channel-voice kind・不正値の
  defined reject）、queue overflow → `dropped` 観測、
  `unsupported` 計数、open 失敗パス（不正 device id →
  `DeviceNotFound`）、timestamp stamp 構築、非対応 OS の
  `Unsupported`。
- 実 MIDI デバイス opt-in 確認は `#[ignore]` test / example に
  留め、CI の Pass 条件にしない（実機未検証は NotRun/Unsupported
  として正直に記録）。

## 受入条件（本 slice）

- `crates/ruviform-midi-device` が `MidiInputEnumerator`/
  `MidiInputPort`/`MidiInput` を実装し、Windows では WinMM の
  実 port 列挙・open・source timestamp 取得・`read` drain まで
  動作する（実デバイス不在時は空列挙/`DeviceNotFound` で
  defined に振る舞う）。
- macOS/Linux では `Unsupported` を返す境界がコンパイル・
  テスト可能な形で存在する。
- device-loss/recreation/overflow は契約どおり観測可能
  （`dropped`/`unsupported`/`StreamInvalidated`/`DeviceNotFound`）
  で、サイレント失敗しない範囲で実装する。WinMM で検出不能な
  mid-stream device loss は制限として design/IR に明記する。
- `Box<dyn MidiInput>` として `InputTx::pump` が要求する
  `read` 契約（bounded・stats）を満たす — adapter 提供面の
  trait-object 経路で結線確認する。
