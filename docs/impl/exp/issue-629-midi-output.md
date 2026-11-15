# Issue #629 MIDI output path — 実装実験記録（2026-11-15）

`docs/tasks/issue-629-midi-output.md` の 2nd slice（discovery gap #1
「`midiOut*` 送信経路不在」の解消）実装で試行・観測した事項の記録。
実装結果の正本は
[`docs/impl/ir/issue-629-midi-output.md`](../ir/issue-629-midi-output.md)、
イベントログは
[`issue-629-midi-output-exp.jsonl`](issue-629-midi-output-exp.jsonl)
を参照。

## 試行 1: pitch bend の 7-bit byte 取り出し（cast lint）

- **目的**: `encode_message` で `bend_as_u14` の結果（`<= 0x3FFF`）
  から data1/data2 を取り出す。
- **観測**: `(raw14 & 0x7F) as u8` / `(raw14 >> 7) as u8` は値が
  構成上 7-bit に収まるが、`clippy::cast_possible_truncation` は
  `u16 → u8` の `as` を機械的に警告する。workspace は `-D warnings`
  で実質 deny。
- **結論**: `u8::try_from(...).ok()?` を採用 — `Option` 返しの関数
  と整合し、lint suppress（`#[expect]`）を増やさない。media-core
  自身が同じ問題を `#[expect]` で処理している先例はあるが、こちらは
  `Option` があるため try_from の方が自然。

## 試行 2: SysEx 送信の完了待ち設計

- **目的**: `midiOutLongMsg` は非同期 — `MIDIHDR`/`lpData` は
  `MM_MOM_DONE` まで生存必須。誰がいつ buffer を解放するか。
- **観測**: 入力側は driver→queue の一方向で buffer pool + lazy
  requeue が必要だったが、出力は 1 send = 1 buffer で
  `send_sysex(&mut self)` が直列化される。callback は WinMM 契約上
  allocation/OS call 不可のため、完了通知は `Mutex<bool>`+`Condvar`
  の flag のみで足りる。
- **結論**: buffer（`Box<MIDIHDR>`+`Box<[u8]>`）を send_sysex の
  stack frame に置き、prepare→longmsg→done wait→unprepare を同期的
  に行う。pool・世代カウンタ・未完了キューは持たない（KISS/YAGNI）。
  `&mut self` 直列化により「前 send の DONE 未消費で次の flag を
  reset」する競合は構造上起きない。device stall で無期限 block
  し得る点は doc に明記（control path の既知リスク）。

## 試行 3: framing 違反・非対応 event の `DeviceError` 写像

- **目的**: `send` に `encode_message == None` の event、`send_sysex`
  に `0xF0…0xF7` でない bytes が来た場合の error variant。
- **観測**: `DeviceError` に InvalidArgument 相当の variant はなく、
  `Backend(&'static str)` は「backend failure」用、`UnsupportedConfig`
  は「configuration」用 — どれも厳密には caller-contract 違反を
  表さない。
- **結論**: 両者とも「この port の wire 表現で送れない」点で同一
  分類とし `DeviceError::Unsupported` に統一。codec の `None` と
  port の `Unsupported` が同じ「表現不能」意味論で対応する。error
  variant 追加は shared `DeviceError`（lease 外・影響大）のため
  行わず、写像の限界は本記録と IR Risks に残す。

## 試行 4: `doc_markdown` clippy（`SysEx`/`WinMM` の backtick）

- **観測**: 新規ファイルの doc comment に bare の `SysEx`/`WinMM` を
  書くと `clippy::doc_markdown` が `-D warnings` で error。
  既存 `windows.rs` は `WinMM` を backtick 済みのため非該当。
- **結論**: doc comment 内の `SysEx`/`WinMM` を全て backtick 化。
  `#[allow]` による lint 隠蔽は行わない（禁止事項）。

## 試行 5: 検証レーン実行

- **観測**: `mise run test-rt` は workspace 全体の `rt_` filter で
  コンパイルを伴うため初回は長時間（mbx cache で ~1.5 分）。本 slice
  は control-path のみで新規 `rt_` test は追加していない —
  `encode_message` は pure だが `rt_` prefix 契約は既存 test 群で
  カバー済みの範囲。test-rt は「影響なし確認」として Pass。
- **結論**: canonical = mise task のみで全レーン Pass。実デバイス
  smoke（`#[ignore]`）は `test-midi-real-device` lane が自動収集
  するため mise.toml 変更不要 — 本 slice では NotRun と記録。
