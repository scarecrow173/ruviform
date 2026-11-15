---
id: "SPEC-RF628sBscripted0000001"
type: "spec"
status: "approved"
title: "Issue #628 scripted port-trait backend — deterministic device boundary slice"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-628-scripted-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-628-scripted-backend.md"
  depends-on:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-628-scripted-backend.md"
    - "docs/test-specs/issue-628-scripted-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-628-scripted-backend.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 scripted port-trait backend — deterministic device boundary slice

## What Should Be Built

`crates/ruviform-audio-test`（test-only、Issue #628 first slice で新設済み）に
**scripted port-trait backend** を追加する。`ruviform-audio-core::device` の
`DeviceEnumerator`/`Device`/`InputStream`/`OutputStream` 契約をそのまま実装し、
VirtualClock・PcmScenario・MIDI trace driver と結線して、deterministic な
device 境界試験を可能にする:

- **Scripted backend** — `ScriptedBackend`（`DeviceEnumerator`）/
  `ScriptedDevice`（`Device`）/ scripted output・input stream。
  `Device: Send`・stream `Send` を満たす。
- **DeviceScript/FaultScript** — enumerate/open 時エラー
  （`DeviceBusy`/`Backend("permission_denied")`/`UnsupportedConfig`/
  `Unsupported`/`DeviceNotFound`）、callback index/frame 指定の
  error-callback 注入（`Xrun`/`StreamInvalidated`/`ClockDiscontinuity` 等）、
  `frame_position` jump、`capture: None` 化、可変 frame count、
  デバイス消失（vanish）/復帰（reappear）を script で表現する。
  device 境界の fault は backend script の一部として内包し、port 中立の
  generic `FaultInjector` は作らない（YAGNI）。
- **VirtualClock 結線** — `OutputTimestamp`/`InputTimestamp`・`now()`・
  `clock_domain`・`StreamLatency`・`buffer_size` を script 通り正直に報告
  し、first slice の `VirtualClock`/`VirtualClockDriver` を再利用する。
- **PcmScenario/PcmFixture 結線** — input: scenario block を
  `InputDataCallback` へ再生（gap は silence + `Xrun` 通知）、
  output: callback が書いた view を capture して検証可能にする。
  callback 発火は test driver の manual pump（deterministic・thread 不使用）。
- **MIDI 薄 adapter** — `MidiScenario` cue を `TimedMidiInput` へ変換し、
  VirtualClock 駆動の frame 位置で `ruviform-audio-core::midi_input::mock`
  の `MockPort` へ投入する。`inject_dropped`/`inject_unsupported`/
  `fail_next_open` を利用する。
- **正負テスト** — Format/Channel/Buffer/Clock、loss/xrun/disconnect/
  reconnect/permission/queue-full/RT 境界で Fault と Expected/Observed
  の一致を検証し、callback 発火経路の alloc-free を `rt_` prefix
  `assert_no_alloc` テストで証明する。

本 slice は Issue #628 の **2nd slice**。Issue の受入チェックボックスは
本 slice 単独で close しない（Issue は open のまま、PR は `Refs #628`）。
本 slice が直接対象とするのは scripted backend が提供する device 境界の
fault・clock・PCM/MIDI 経路とその正負テスト（Issue 受入の第 2・3・4・7
項目の backend 側部分）。

## Why Now

- first slice（deterministic core、PR `scarecrow173/ruviform#638` merge 済み）
  で VirtualClock・PcmFixture/PcmScenario・MIDI trace driver・oracle・
  RunMetadata が揃い、それらを device port 契約へ結線する土台ができた。
- POLICIES.md 2026-11-15 仮想優先改訂の OS-L1 deterministic backend 要件
  （Format/Channel/Buffer/Clock・loss/xrun/disconnect/reconnect/permission/
  queue-full/RT 境界の正負検証）を、実 OS API なしで検証するには
  port 契約を正直に実装した scripted backend が必要。
- 先例（`crates/ruviform-audio-device/tests/mock_backend.rs`・
  `mock_input_backend.rs`・`ruviform-audio-core::midi_input::mock`）は
  test 内完結であり、複数 consumer が共有する test-only crate としての
  scripted backend が本 slice の価値。

## Users and Value

- **Ruviform 自身**: device 境界契約（open/format/timestamp/latency/
  fault）の試験を機材なし・決定論的に実行できる。
- **AudioLens / Synvelis（後続 slice）**: 本番 Recording/Playback/
  Timestamp/Host adapter へ scripted device を `Box<dyn …>` で接続し、
  同一 Fixture version の独立 Oracle で回帰検証できる土台。
- **後続 slice（#629〜#631）**: 仮想 OS Endpoint・実 ABI Test Plugin・
  GUI Runner が乗る deterministic backend の device 側実装。

## Scope

### In Scope

- `crates/ruviform-audio-test/src/backend.rs` + `src/backend/` —
  scripted port-trait backend module。
- `ScriptedBackend`/`ScriptedDeviceHandle`/`DeviceSpec`/`OutputSpec`/
  `InputSpec`/`StreamScript`/`OpenFaults`/`StreamFault`/`PumpReport`/
  `StreamDirection` 等の公開 API。
- enumerate/open/describe/stream 制御面の fault script と、
  callback index/frame 指定の stream fault 注入。
- VirtualClock 結線による timestamp/`now()`/`clock_domain`/latency/
  `frame_position` の正直な報告。
- input: `PcmScenario` 再生（gap → silence + `Xrun`）、output:
  callback 書込み view の capture。
- MIDI 薄 adapter: `MidiScenario` → `TimedMidiInput` feed +
  `MockPort` pump（`inject_dropped`/`fail_next_open` 結線）。
- `tests/backend.rs`・`tests/backend_faults.rs` 新設、`tests/rt_contract.rs`
  拡張（callback 発火経路の alloc-free 証明）。
- crate README・`src/lib.rs` doc の本 slice 分更新、docs chain、
  evidence/rf628 検証記録。

### Out of Scope

- 本番 pipeline への配線（Audio Input→本番 Recording、Output→本番
  Playback、MIDI→本番 Timestamp/Host adapter — consumer 側）— 後続 slice。
- consumer oracle（AudioLens f64/長尺/解析非回帰、Synvelis PCM/Project/
  RT/Recording/MIDI — 双方の resolved Ruviform SHA）— 別 repo・後続 slice。
- CI lane 追加・workflow 変更・`mise.toml` 変更 — 範囲外。新規テストは
  既存 `test`/`test-rt` lane に自動収集される形で「既存 CI への統合」
  を満たす。
- 3 OS 実走証拠・Windows 仮想 OS Endpoint — `scarecrow173/ruviform#629`。
- rate change mid-stream — port 契約上不可。`ClockDiscontinuity` +
  re-open で代替する（design に明記）。
- port 中立の generic `FaultInjector` — device 境界 fault は backend
  script に内包（YAGNI）。

## Requirements

- port 契約の正本は `crates/ruviform-audio-core/src/device.rs` の
  `DeviceEnumerator`/`Device`/`InputStream`/`OutputStream`。実装は契約を
  そのまま満たし、`Device: Send`・stream `Send` を満たす。
- Fake-only 分岐を製品処理へ侵入させない — 本 backend は test-only
  crate 内に閉じ、production crate は本クレートに依存しない。
- capability を偽装しない — 非対応 direction は `Ok(vec![])`/
  `Ok(None)`/`Err(Unsupported)` で正直に報告し、config 不一致は
  `UnsupportedConfig` で拒否する。
- timestamp/latency/`frame_position`/`capture`/`buffer_size` は script
  通り正直に報告する（捏造しない）。
- callback 発火は test driver の manual pump — deterministic・
  thread 不使用。callback が発火するのは `start()` 後のみ。
- Format/Channel/Buffer/Clock・loss/xrun/disconnect/reconnect/
  permission/queue-full/RT 境界を正負テストで検証し、Fault と
  Expected/Observed が一致する。
- callback 発火経路（view 構築 + callback 起動 + timestamp 構築）は
  allocation-free — `rt_` prefix `assert_no_alloc` テストで証明。
- 本 Harness の公開必要 API・内部 Test-only API・製品配布除外・
  軽量ローカルテスト/Daily Full CI レーン範囲の本 slice 分を
  crate README・`src/lib.rs` doc・docs に明文化する。
- 公開 enum は全て `#[non_exhaustive]`（
  `docs/adr/public-enum-non-exhaustive-policy.md`）。`unsafe` 不使用。
- 新規依存・新規 feature の追加なし（既存 path 依存 + dev-dep
  `assert_no_alloc` のみで実現する）。

## Acceptance Criteria

- [ ] `ScriptedBackend` が `DeviceEnumerator` を実装し、
  enumerate/open/describe/stream の scripted fault を
  `DeviceError` のまま観測できる。
- [ ] input stream が `PcmScenario` を `InputDataCallback` へ再生し、
  output stream が callback 書込みを capture し、timestamp/
  `frame_position`/`capture`/`clock_domain`/latency/`buffer_size` が
  script・VirtualClock 通りに報告される。
- [ ] Format/Channel/Buffer/Clock・loss/xrun/disconnect/reconnect/
  permission/queue-full の正負ケースで Fault と Expected/Observed が
  一致するテストが Pass する。
- [ ] `rt_` prefix テストが callback 発火経路の alloc-free を
  `assert_no_alloc` で証明し、`mise run test-rt` で実際に収集・実行
  される（テスト名で確認）。
- [ ] MIDI 薄 adapter が `MidiScenario` cue を `TimedMidiInput` へ
  変換し、VirtualClock frame 位置で `MockPort` へ投入、
  `inject_dropped`/`fail_next_open` が観測できる。
- [ ] test-only の公開範囲・正直な制限（rate change 不可・callback は
  test driver スレッド発火・permission denial の honest mapping 等）
  が crate README・`src/lib.rs` doc・design doc に明記されている。
- [ ] Issue #628 の残受入（pipeline 配線・consumer oracle・CI lane・
  3 OS 実走）は本 slice 対象外として docs/PR に明記されている。

## Deferred Design Concerns

- 本番 pipeline 配線・consumer oracle・CI lane 追加・3 OS 実走証拠・
  Windows EXT-L3 — 後続 slice / 別 Issue が担当。
- `environment_kind = deterministic-backend` の Pass は OS-L1 証拠で
  あり、実 OS path（WASAPI/WinMM 等）の証明ではない。

## Sources and Relations

- `scarecrow173/ruviform#628`（本 Issue）、`scarecrow173/ruviform#627`
  （Closed — Case Schema v1・`environment_kind`・Test Depth の契約元）。
- `docs/specs/issue-628-deterministic-test-runtime.md` /
  `docs/designs/issue-628-deterministic-test-runtime-design.md`
  （first slice — 本 slice が結線する deterministic core）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・検証レベル定義。
- port 契約正本: `crates/ruviform-audio-core/src/device.rs`。
- 先例: `crates/ruviform-audio-device/tests/mock_backend.rs`・
  `mock_input_backend.rs`、`crates/ruviform-audio-core/src/
  midi_input/mock.rs`（`TimedMidiInput`・`inject_dropped`/
  `fail_next_open`）、`crates/ruviform-audio-core/src/lib.rs`
  `device_contract` test module。
