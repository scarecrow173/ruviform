---
id: "DESIGN-rf253osmidibackend001"
type: "design"
status: "approved"
title: "Issue #253 OS MIDI input backend design（WinMM adapter + Unsupported 境界）"
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
  implements:
    - "docs/specs/issue-253-os-midi-backend.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/select-midi-input-backend.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-os-midi-backend.md"
  derived-by:
    - "docs/plans/implement-issue-253-os-midi-backend.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-os-midi-backend.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://learn.microsoft.com/en-us/windows/win32/api/mmeapi/"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 OS MIDI input backend design

## adapter 配置判断

`crates/ruviform-midi-device` を新設する。理由:

- `ruviform-audio-device` は「cpal を内部依存に閉じ込める audio
  device adapter」であり単一責務。MIDI input は別デバイスドメイン
  であり、同居させると cpal/audio 由来の型・エラーが MIDI 側へ
  混入する余地が生まれる。
- issue-188 detailed design は adapter crate 新設方針を示しており、
  `ruviform-audio-device` という命名規約に合わせ `ruviform-midi-device`
  とする（audio 側と対称的に「OS MIDI device 境界 adapter」）。
- 依存方向: `ruviform-midi-device` → `ruviform-audio-core`
  （`midi_input` port trait・`device::DeviceError`）+
  `ruviform-media-core`（`MidiEvent`/`InputStamp` 等）のみ。
  adapter→adapter・engine→adapter 以外の逆方向依存は作らない。

公開面: `pub struct SystemBackend`（`MidiInputEnumerator` を実装）+
`pub fn backend() -> SystemBackend` 相当の factory。WinMM の型・
エラー・feature 名は公開 API に露出しない。非対応 OS では同じ
型が `Unsupported` を返すため、consumer は cfg を意識しない。

## backend library 選定（ADR 参照）

`docs/adr/select-midi-input-backend.md` を正本とする。結論:

- **Windows: raw WinMM（`windows-sys` 0.61 の `Win32_Media`/
  `Win32_Devices_Properties` feature）を採用。**
- `midir` は不採用: `MIM_MOREDATA`/`MIM_LONGERROR` の喪失 signal・
  callback 生 timestamp の観測を wrapper が握りつぶし、
  defined-drop 計数の要件（R5）を満たせない。依存追加の正当性も
  薄い（subset しか使わない）。
- WinRT/UWP `Windows.Devices.Midi` は不採用: UIPI・packaging
  制約があり desktop plugin 文脈で重い（task 指定の優先順位とも一致）。

`windows-sys` は workspace で既に `ruviform-audio-device` 経由
（cpal）で lock 内に存在するため依存木の増分は feature 追加のみ。

## WinMM セマンティクス（MSDN 確認済み）

- `MIM_DATA`: `dwParam1` = packed message、`dwParam2` = timestamp。
  packed layout: byte0=status, byte1=data1, byte2=data2, byte3 未使用。
  WinMM は running status を展開済みで status byte を必ず含む。
- `dwParam2`: **midiInStart 呼出時に 0 に reset される ms カウンタ**
  （「the time stamps ... specified in milliseconds ... reset ... to
  zero」）。したがって stamp は「当該 subscription の stream 開始
  からの経過 ms」であり、絶対時計ではない。
- `MIM_MOREDATA`: `MIDI_IO_STATUS` open flag 使用時、driver 側 queue
  が溢れた message を通知（同じ packed 形式）。= 喪失 signal。
- `MIM_LONGDATA`/`MIM_LONGERROR`: SysEx（MIDIHDR buffer 経由）。
  canonical 語彙外のため計数のみ。
- `MIM_CLOSE`（`MM_MIM_CLOSE`）: stream close 通知。
- callback 制約: 「system-defined functions を callback 内で呼ぶな」
  （例外は EnterCriticalSection/LeaveCriticalSection/PostMessage/
  SetEvent/timeGetTime 等の whitelist）。→ queue push は Mutex +
  事前確保 VecDeque のみ。SysEx buffer の requeue
  （`midiInAddBuffer`）は callback 内で呼べないため `read` 側で
  行う（midir も callback 内 requeue を行うが MSDN 推奨に反する
  ため本実装では採用しない）。
- device loss: WinMM は mid-stream の device removal を `read`
  側へ通知する API を持たない。open 時解決失敗 → `DeviceNotFound`、
  列挙時の消失 → 列挙から脱落、という形でのみ観測可能。
  **制限として明記**（実機 slice で再検証）。

## timestamp / clock domain

- `TimeBase`: `num=1, den=1000`（ms）。
- `ClockDomainId`: adapter が subscription（= port handle の open）
  ごとに採番する private domain。理由: dwParam2 は midiInStart 毎に
  0 reset される per-stream epoch であり、複数 port を同時 open
  した場合に単一 domain id では epoch が衝突する。`ClockDomainId`
  の doc は「backend が割り当てる id」を許容しており、engine の
  ClockAnchor は domain 毎に独立しているため per-subscription
  採番が正しい。採番は process 内 atomic counter（private base
  `0x5255_4D49_4449_4D49_4E31` "RUVMIDIMIN1" 相当の固定 base +
  increment。well-known id 0/1/2 と衝突しない）。
- `TimestampProvenance`: `Carried`（driver が message に付与した
  timestamp を変換せず保持）。
- adapter は `map_stamp_to_frame` を呼ばない。engine/consumer の
  責務。

## module 構成（`crates/ruviform-midi-device`）

```
src/lib.rs        — crate doc, pub use, cfg dispatch
src/system.rs     — SystemBackend + MidiInputEnumerator impl（OS 非依存）
src/midi.rs       — dwParam1/dwParam2 → MidiEvent decode（pure・全 OS で test 可能）
src/queue.rs      — bounded event queue + stats（pure・全 OS で test 可能）
src/os/windows.rs — WinMM FFI 実装（cfg(windows)）
src/os/unsupported.rs — Unsupported stub（cfg(not(windows))）
tests/            — 結線・公開面テスト
examples/         — list_midi_ports（手動・CI 対象外）
```

pure module（midi.rs/queue.rs）を cfg 外に置くことで、decode/queue
の正しさを device 非依存・全 OS で単体テストできる。

## WinMM 実装詳細

### 列挙

`midiInGetNumDevs` → 各 index で `midiInGetDevCapsW`（szPname
32×u16 → String）+ `midiInMessage(hmi=index as HMIDIIN,
DRVM_DRIVER=...` ではなく `DRV_QUERYDEVICEINTERFACESIZE`(2061)/
`DRV_QUERYDEVICEINTERFACE`(2060) で stable id を取得
（midir と同じ手法）。id が取れない legacy device は
`winmm:{index}:{name}` fallback（open 時に name 再照合して誤爆を
防ぐ）。GetDevCaps 失敗 index はスキップ（列挙中の消失は定義動作）。

`MidiInputInfo`: `id`=上記、`name`=szPname、
`origin=MidiPortOrigin::Unknown`、`capabilities`=
`stamped(false /*ump*/, true /*timestamps*/, Some(domain))`。
domain は port 構築時に採番（1 handle=1 open=1 subscription のため
port 単位で一意）。

### open

1. `opened` AtomicBool CAS で 1 回制限（2 回目 `DeviceBusy`）。
2. index 解決: interface id で全 port を再スキャンして一致を探す
   → 見つからなければ fallback key の name 照合 → それも駄目なら
   `DeviceNotFound`。
3. `midiInOpen(CALLBACK_FUNCTION | MIDI_IO_STATUS,
   dwInstance=Shared ptr)`。MMERR_ALLOCATED→`DeviceBusy`、
   MMSYSERR_BADDEVICEID→`DeviceNotFound`、その他→`Backend`。
4. SysEx 用 buffer を `SYSEX_BUFFER_COUNT=4` × 1KiB 確保し
   `midiInPrepareHeader`+`midiInAddBuffer`。
5. `midiInStart`。失敗時は取得済み resource を解放して Err。
6. `Subscription` 構築（Arc<Shared> + HMIDIIN + headers）。

### callback（extern "system"）

`dwInstance` = `*const Shared`（Arc::into_raw で渡し、Subscription
が Arc を所有）。先頭で `closing` flag を確認し true なら即 return
（close 中の UAF 防止 — Arc が Subscription drop まで Shared を
生かすので ptr は常に valid）。

- `MM_MIM_DATA`(0x3C3): `dwParam1` から status/d1/d2 を取り、
  channel-voice（0x8n–0xEn）なら `midi::decode` → queue.push
  （満杯なら dropped++）。非 channel-voice（0xF0+ system、
  0xF1–0xF7 common/realtime、status<0x80 の異常値）は
  `unsupported++`。
- `MM_MIM_MOREDATA`(0x3C5): `dropped++`（driver queue 溢れ通知）。
- `MM_MIM_LONGDATA`(0x3C4): `unsupported++`（SysEx=語彙外。
  buffer requeue は read 側）。
- `MM_MIM_LONGERROR`(0x3C6): `dropped++`（long message 喪失）。
- `MM_MIM_CLOSE`(0x3C7): `dead=true`（以後 read は
  `StreamInvalidated`）。
- その他 msg: 無視（定義済み未知 message のみ）。

queue push は `Mutex<QueueState>` + 事前確保 `VecDeque`
（capacity=1024）。push は capacity 内なら alloc なし。
Mutex は callback-safe（critical section 系、MSDN whitelist の
EnterCriticalSection 相当）。OS API・alloc・I/O は呼ばない。

### read（非 RT）

`Mutex` lock → drain 最大 `max_events` → counter snapshot を
reset して返す（since-last-read semantics。mock の振る舞いと一致）。
その後 done 状態の SysEx header を `midiInAddBuffer` で requeue
（OS 呼出は非 callback thread なので合法）。`dead` なら
`StreamInvalidated`。

### close / Drop

`midiInStop` → `midiInReset` → `closing=true` → `midiInClose` →
header `midiInUnprepareHeader` + buffer free。idempotent
（`closed` flag）。`Drop` も同経路を呼ぶ（close 忘れ対策）。

## port 型への additive 変更（audio-core）

1. `MidiPortOrigin::Unknown` 追加 — WinMM は origin を判別不能。
   `#[non_exhaustive]` のため既存 match は wildcard で吸収。
2. `MidiReadStats.unsupported: u64` 追加 — 受信したが語彙外の
   message 計数。`new(dropped)` は unsupported=0、
   `with_unsupported` constructor 追加。既存 literal 構築は
   同 crate 内 test のみで追従修正。
3. 破壊的変更なし。mock は `unsupported=0` で不変。

## 非対応 OS stub

`src/os/unsupported.rs`: `ports() -> Err(DeviceError::Unsupported)`。
`SystemBackend::input_ports` はこれを呼ぶ。port を返せないため
`open` 経路は存在しない。全 target でコンパイル。

## 検証方針

- pure module の単体テスト（decode/queue/stats/stamp 構築）—
  device 非依存で全 OS Pass。
- Windows: `midiInOpen` 不正 id → `DeviceNotFound`、列挙が
  Ok（空でも）であること、公開面 trait-object 経路の結線。
- 非 Windows: `input_ports` → `Err(Unsupported)`。
- `#[ignore]` 実デバイス smoke（`examples/list_midi_ports` +
  ignored test）— CI Pass 条件外。
- canonical: `mise run fmt-check check lint test test-rt`。

## 未解決・制限（正直な記録）

- WinMM に mid-stream device-loss 通知がない → `read` 中の
  hot-unplug は検出不能。open 解決失敗・列挙脱落・`MIM_CLOSE`
  のみ。実機 slice で要再検証。
- `MIM_MOREDATA` の正確な意味（message 喪失 vs 通知のみ）は
  MSDN 記述から「喪失 signal」と解釈。実機で挙動確認が必要。
- timestamp は per-stream ms（絶対時計でない）→ domain 採番で
  対処するが、engine 側 anchor 校正の実挙動は consumer slice で検証。
- macOS/Linux は Unsupported 境界のみ。CoreMIDI/ALSA 実装は別 slice。
- `PumpStats`/`InputTx` 側は `MidiReadStats` の新 field を
  透過しない（bridge 側の対応は本 slice 範囲外・未対応作業に記載）。
