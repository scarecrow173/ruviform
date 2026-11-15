# Issue #629 MIDI loopback verifier — 実装実験記録（2026-11-15）

`docs/tasks/issue-629-midi-loopback.md` の 3rd slice（probe の
#658 後 refresh + loopback round-trip verifier）実装で試行・観測した
事項の記録。実装結果の正本は
[`docs/impl/ir/issue-629-midi-loopback.md`](../ir/issue-629-midi-loopback.md)、
イベントログは
[`issue-629-midi-loopback-exp.jsonl`](issue-629-midi-loopback-exp.jsonl)
を参照。

## 試行 1: probe の production view 双方向化と verdict 修正

- **目的**: 1st slice probe が input のみだった production view を
  #658 の output enumerator で補完し、「send path 不在」の stale
  verdict を修正する。
- **観測**: `backend().output_ports()` は `Box<dyn
  MidiOutputEnumerator>` 経由で `info()`（id/name/origin/
  capabilities）を返し、input と同形式で表示できる。output は
  `provides_timestamps=false`/`clock_domain=None` を正直に宣言する
  ため、input との対比がそのまま capability の可視化になる。
- **結論**: `print_port_view` に output 側 match を追加（error は
  `production output enumeration failed:` で個別報告 — 片方向の
  失敗で他方を隠さない）。verdict の round-trip 行を「send path は
  2nd slice で実装済み、残 gap は provisioned loopback pair
  （verifier: `tests/loopback.rs`）」に更新。

## 試行 2: WMS availability 検出の方式

- **目的**: loopMIDI vs Windows MIDI Services の採用判定材料として
  probe に WMS の有無を出す。
- **観測**: `powershell.exe -NoProfile -NonInteractive -Command` で
  `Get-Command New-MidiLoopback` と `Get-Service midisrv` を照会。
  本ホスト実測: `New-MidiLoopback: absent` だが `midisrv: Running`
  — service は常駐するが PowerShell module/cmdlet は未導入、という
  状態がそのまま記録できた。`cmd /c ver` 先例どおり spawn 失敗・
  非ゼロ終了は `NotRun` note で probe を落とさない。
- **結論**: module 内 `#[cfg(windows)]` の `Command` 呼出のみ
  （safe API、新規 unsafe なし・新規 windows feature なし）。
  cmdlet と service を別行で報告し、「service 稼働 = loopback
  使える」と誤読しない形にした。

## 試行 3: verifier の pair 特定方式

- **目的**: production output+input port から loopback pair を
  特定する規則を決める。
- **観測**: port id は方向で体系が異なる（input は interface-id または
  `winmm-fallback:{i}:{name}`、output は `winmm-out:{i}:{name}`）
  ため id 照合は不可能。`info().name` の双方向一致のみが
  probe の loopback candidate signature と同じ正直な鍵。
- **結論**: `info().name` 一致で pair 化し、複数 candidate は全列挙
  して先頭を使用。name match は「loopback-style candidate」で
  あり virtual endpoint の証明ではない（`MidiPortOrigin::Unknown`）
  — verdict と test-spec R6 で `virtual-os-endpoint` 非主張を固定。

## 試行 4: 期待列の独立 Oracle

- **目的**: send→receive の照合に使う期待値を被試験コードから
  独立させる（自己参照 Oracle 禁止）。
- **観測**: `MidiValue::from_u7`/`bend_from_u14` で wire 幅から
  canonical 値を構築すれば encode→decode が恒等（codec unit test
  で固定済み）。ch3・velocity 100/64・CC10=90・bend 0x2A00
  （center 以外）で channel/payload の往復を検証できる。
- **結論**: 4 event の固定リテラル列（NoteOn/CC/PitchBend/NoteOff）
  を `expected_channel_voice_sequence()` に置き、観測列への
  順序保存部分列一致で判定。`MidiScenario`（audio-test）は
  TransportScheduler 駆動用で port 直接送信に非適合のため再利用
  せず、形式言語のみ共有。

## 試行 5: `test-midi-real-device` lane の実機実行で発覚した
#658 の潜在 defect

- **目的**: canonical lane で新規 `#[ignore]` test を含めて実機
  実行し、verifier の fail-closed `NotRun` と silent-send smoke を
  記録する。
- **観測**: lane が `os::windows_output::tests::real_device_send_
  smoke`（src/ unit test、#658 由来）で FAILED —
  `winmm midiOutOpen failed`。単独再実行でも同じ。PowerShell
  P/Invoke で切り分け: `midiOutOpen` に `CALLBACK_NULL`(flags=0)
  → rc=0 成功、`CALLBACK_FUNCTION`(0x30000) のみ → rc=0 成功、
  `CALLBACK_FUNCTION|MIDI_IO_STATUS`(0x30040) → **rc=10
  MMSYSERR_INVALFLAG**、`MIDI_IO_STATUS` 単独 → rc=10。
  GS Wavetable Synth は `MIDI_IO_STATUS` flag を拒否する。
- **結論**: `src/os/windows_output.rs::open_at_index` が
  `CALLBACK_FUNCTION | MIDI_IO_STATUS` を無条件で渡すため、本
  ホストの唯一の output device では open が必ず失敗する。2nd
  slice はこの lane を実行していなかった（NotRun 記録）ため
  潜在化していた。当初は編集範囲外として coordinator に scope
  確認を送付 → **Option A 承認**（同ファイルを占有範囲に追加）
  で本 slice 内に修正（試行 6）。

## 試行 6: `midiOutOpen` flag fallback と `send_sysex` degrade

- **目的**: coordinator 承認のもと `MIDI_IO_STATUS` 非対応
  driver で output port が open できるようにし、SysEx 完了待ちの
  意味論を正直に degrade する。
- **観測**: fallback（`CALLBACK_FUNCTION` のみ）で open は成功
  するが、`MM_MOM_DONE` は status message であり fallback では
  一切届かない — `send_sysex` の condvar wait は永久ブロックに
  なる。buffer を driver 参照中に unprepare することは unsound
  なため、fire-and-forget への降格ではなく `Unsupported` 拒否が
  唯一の正直な degrade（trait contract の「long-message path を
  持たない backend は honest refusal」と同じ語彙）。
- **結論**: `open_at_index` は `MMSYSERR_INVALFLAG` のときのみ
  `CALLBACK_FUNCTION` で 1 回 retry（他エラーは retry しない）。
  `Subscription.io_status` で経路を記録し、`send_sysex` は
  `!io_status` で早期 `Unsupported`。`send`（`midiOutShortMsg`）
  は completion 不要のため無影響 — module/`lib.rs`/API doc に
  明記。実機 lane 再実行で lib smoke・`real_device_silent_
  output_smoke`（GS Wavetable へ非可聴 send、`verdict: Pass`）・
  `output_ports` smoke（`send_sysex: Unsupported` degrade を
  観測）が全て Pass。
