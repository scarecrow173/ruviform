---
id: "ADR-034Qx1ubqHuTT3UcXRAQGK"
type: "adr"
status: "accepted"
title: "Issue #187: input port surface shape, same-clock duplex via clock-domain correlation, capture timestamp/latency honesty"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/synvelis/issues/12"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
  implemented-by: []
  depends-on:
    - "../designs/audio-device-boundary-and-poc.md"
    - "select-audio-device-backend.md"
    - "realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "chunk-outputcallbackinfo-timestamp.md"
    - "issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md"
    - "public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
    - "../designs/audio-device-boundary-and-poc.md"
    - "select-audio-device-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/59"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/synvelis/issues/12"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/187"
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #187: input port surface shape, same-clock duplex via clock-domain correlation, capture timestamp/latency honesty

## Context and Problem Statement

Issue #187 は既存 output 中心の device boundary port（`ruviform_audio_core::device`）に、汎用 Audio Input/Duplex・Capture Clock/Latency 契約を additive に追加することを要求する。残る設計判断は (a) input 契約の trait 形状（`Device` 拡張 vs `InputDevice` 分離）、(b) 同一 clock duplex の表現方法（`DuplexStream` vs 独立 stream + clock domain 相関）、(c) capture timestamp/clock の語彙（device 独自型 vs `timed_input` 既存語彙の再利用）、(d) latency 報告の形状（裸の frame 数 vs coverage/provenance 付き報告）の 4 点である。headless PoC 段階で実 backend の input plumbing は未検証のため、過剰な抽象を確定しないことが制約となる。

## Decision Drivers

* additive のみ・output 契約の非破壊（Issue 前提条件）。`OutputCallbackInfo` は pub field 構造体のため field 追加は source-breaking であり、出力側の callback info は不変とする
* 「信頼できない値を sample-accurate と偽装しない」「条件が不明なまま Latency を加算しない」（Issue ・Synvelis #12 §16.5）。`Option`/`UNKNOWN`/provenance で正直に表現する
* ADC/DAC 実時刻と callback（host 観測）時刻の区別が latency 二重計上防止の前提（Synvelis #12 §16.5: 実 ADC/DAC 時刻が得られれば input/output latency を再加算しない）
* backend は「同一 clock」「frame position」「latency」を報告できないことがある — 未対応は `Option`/`UNKNOWN`/`Unsupported` で表現し、adapter は値を捏造しない
* `timed_input` の `ClockDomainId`/`InputStamp`/`ClockAnchor`/`map_stamp_to_frame` が stamp→frame の唯一権威として確定済み（#188）。device capture clock を別系統の stamp 型にすると二重の語彙ができる
* RT 契約（callback 内 alloc 0・非ブロッキング・panic 禁止）は output と同規則を踏襲（realtime ADR）
* 実 backend で duplex を真に同一 clock で駆動できるかは host/backend 依存（JACK は native duplex、WASAPI は独立 endpoint、CoreAudio は same-device duplex）— 未実測の同期保証を契約に書かない

## Considered Options

* **input port の形状**: (a) `DeviceEnumerator`/`Device` に入力側 method を追加する 1 ハンドル契約、(b) `InputEnumerator`/`InputDevice`/`InputStream` を独立 trait 系として追加、(c) 新規 `input` module に分離した完全独立 port
* **同一 clock duplex の表現**: (a) `Device::open_duplex_stream` で結合 stream 型を導入、(b) 独立 `InputStream`+`OutputStream` の `clock_domain()` 一致（`UNKNOWN` 非該当）で同一 clock を検出、(c) host/backend 単位の capability flag で「duplex 可能」を列挙
* **capture timestamp の語彙**: (a) `MediaTime`（`DEVICE_CLOCK_TIMEBASE`）+ `Option<MediaTimestamp>`（time+provenance）+ `ClockDomainId` を `InputCallbackInfo` に載せ、`capture_stamp()` で `InputStamp` を構成、(b) device 専用の新規 stamp 型を定義、(c) `InputStamp` を直接 callback info の型に使う
* **latency 報告**: (a) `StreamLatency{frames, coverage, provenance}` の `Option` 報告（`input_latency`/`output_latency`）、(b) `Option<u64>` の frame 数のみ、(c) PoC では latency API を持たず timestamp 差分のみにする

## Decision Outcome

* **port 形状**: (a) `Device`/`DeviceEnumerator` への入力 method 追加。`Device` は既に方向中立の live handle（name/id）+ 方向別 describe/open であり、cpal の `cpal::Device` が input/output 双方を開く model と一致する。`InputDevice` 分離は name/id/describe の表面を複製し、1 つの backend handle に 2 trait を実装させるだけで抽象化の利益がない。方向未対応は `supported_*_formats` の空 `Vec` と `open_*` の `Err(Unsupported)` で表現する（成功偽装なし）。
* **duplex**: (b) 独立 stream + `clock_domain` 相関。`InputStream`/`OutputStream` に `clock_domain()` を持たせ（`OutputStream` 側は default `ClockDomainId::UNKNOWN` で additive）、同一値・非 `UNKNOWN` の一致のみを「同一 clock」として consumer が扱う。`DuplexStream` は JACK 系 native duplex を除き実 backend の capability 実証がなく、PoC で確定する抽象として早すぎる。同一 clock 検出の規則（`UNKNOWN` 同士は「同一」とみなさない）を port 契約として明文化し、結合 API は実証後の後続 Issue へ defer する。
* **timestamp 語彙**: (a) `InputTimestamp{callback: MediaTime, capture: Option<MediaTimestamp>}` + `InputCallbackInfo{timestamp, clock_domain, frame_position}`。`MediaTimestamp`（time+provenance）は `observe` の既存型で「carried fact vs estimate」の正直性を再利用する。`InputStamp` 直載せの (c) は `ticks: u64`/`TimeBase` の構成責務を adapter へ分散させるため不採用 — adapter は `MediaTime` instant を届け、`capture_stamp()` が一箇所で `InputStamp` を構成する。`capture` が `None` のとき `callback` instant を代用しない（callback 基準値を ADC 時刻と混同すると placement が buffering 分だけ誤る）。
* **latency 報告**: (a) `StreamLatency{frames, coverage: LatencyCoverage, provenance: TimestampProvenance}` を `DeviceResult<Option<…>>` で返す。coverage（`HardwareOnly`/`HardwareAndBuffering`/`Unknown`）は「buffer 含有が不明なら加算しない」という Synvelis 要件の直表現、provenance（`Carried`/`Derived`/`Estimated`）は値の出典。裸の frame 数 (b) は buffer 含有を隠し二重計上を招く。latency API 無し (c) は output 側の `playback` timestamp との対称契約を欠き、Issue の必須項（Input/Output latency 情報）を満たさない。
* **`frame_position: Option<u64>`**: `InputCallbackInfo` に capture clock 上の frame 0 位置を載せる。epoch 内で非減少・無損失時は callback あたり frames 分進行。断絶（device restart・backend が報告する discontinuity）は `DeviceError::ClockDiscontinuity`（新規 variant）で error callback 通知し、前後の position/timestamp を consumer が接続しない規則とする。backend 未報告は `None`（adapter が callback 回数から合成しない — 真の断絶を隠蔽するため）。

### Consequences

* Good, because: output と平行な trait 形状（enumerator→device→stream→callback→error）で学習コストが最小。`MediaTime`/`ClockDomainId`/`TimestampProvenance`/`InputStamp` の既存語彙に接続し、stamp→frame 権威が `timed_input` 一箇所に留まる。`DuplexStream` 不在で実 backend の capability 差に引っ張られない。`OutputStream` の default method（`Ok(None)`/`UNKNOWN`）で additive 互換を保つ。
* Bad, because: `Device` が方向別 method を両方持つため、片方向のみの device で「呼べるが必ず失敗する」method が増える（契約で失敗面を定義済みにすることで緩和）。`clock_domain` を stream open 後にしか比較できず、open 前の同一 clock 事前判定は提供しない（`Device` 単位の domain claim は backend が証明できないため）。`StreamLatency`/`LatencyCoverage`/`frame_position`/`ClockDiscontinuity` と語彙が増える。

### Confirmation

* mock input backend が `InputTimestamp`/`clock_domain`/`frame_position`/`StreamLatency` を script で供給し、deterministic capture→handoff と同一 clock 判定・断絶通知を検証する。
* `OutputStream` の default method で既存実装（`CpalOutputStream`・mock `FakeStream`/`StubStream`）が無変更で compile することを確認する。
* `capture_stamp()` の `InputStamp` が `map_stamp_to_frame` に渡り、`capture: None` が `Option` のまま残ることを unit test で検証する。

## Implementation Plan

* Affected paths: `crates/ruviform-audio-core/src/device.rs`（input contract 追加）、`crates/ruviform-audio-core/src/buffer.rs`（`AudioBufferView::try_new`）、`crates/ruviform-audio-device/src/host.rs`/`device.rs`（input 列挙・format query・`open_input_stream`→`Unsupported`）、`crates/ruviform-audio-device/src/convert.rs`（`map_supported_input_range` 等の mapping）、`crates/ruviform-audio-device/tests/mock_input_backend.rs`（mock PoC）、`crates/ruviform-audio-device/Cargo.toml`（dev-dep `ruviform-rt`）
* Patterns to follow: `OutputTimestamp`/`OutputCallbackInfo`/`OutputDataCallback` の doc・HRTB・`**RT contract:**` ラベル、`Device::id`/`OutputTimestamp::playback` の `Option` での「未対応を型で表現」、「untrusted backend data は adapter boundary で検証」の既存規則、`assert_no_alloc` の `rt_` テスト、`mock_backend.rs` の scripted fake
* Patterns to avoid: `callback` instant を `capture` へ暗黙代入、backend 未報告値の合成（frame position を callback 回数で捏造）、`InputStamp` を port 契約の型に直載せ（構成責務の分散）、`DuplexStream` の先行確定、RT 区間での確保・lock・I/O
* Dependencies or configuration: dev-dep `ruviform-rt`（mock handoff 用・adapter 本体の依存にはしない）。新規 feature なし

## Verification

* [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` exit 0（既存 output 回帰を含む）
* [ ] mock が 2ch scripted block を `frame_position`/`capture` timestamp 付きで届け、SPSC handoff で全 block 順序通り配送
* [ ] `assert_no_alloc` 窓内で callback→view→充填→`try_push` 経路の alloc 0
* [ ] 同一 `clock_domain`（非 `UNKNOWN`）一致で同一 clock 判定、`UNKNOWN` 同士は非一致
* [ ] `Xrun`/`StreamInvalidated`/`ClockDiscontinuity`/`UnsupportedConfig`/`Unsupported` の定義済み失敗面が error callback/open result で観測できる
* [ ] `open_input_stream` が cpal で `DeviceError::Unsupported` を返す（実機 capture 偽装なし）
* [ ] `cargo tree -p ruviform-audio-device --edges normal` で `ruviform-rt` が normal 依存に現れない（dev-dep のみ）

## Pros and Cons of the Options

### input port の形状

* **(a) `Device` 拡張（採用）**: cpal model と一致・describe 表面の重複なし・既存 `Box<dyn Device>` 交換をそのまま使える。悪い点: 片方向 device で必ず失敗する method が生える — `Ok(vec![])`/`Err(Unsupported)` の規則で明示する。
* **(b) 独立 `InputDevice` trait**: 方向別の型分離が明確。悪い点: name/id/describe の複製、1 backend handle に 2 trait、`input_devices()` と `output_devices()` が同じ物理 device を別 object として返す場合の同一性問題。
* **(c) 新規 `input` module の完全独立 port**: output との結合を最小化。悪い点: `DeviceId`/`DeviceError`/`BufferSize`/`AudioFormat` の共有語彙を跨いだ二重 port になり、duplex 相関の接続点が失われる。

### 同一 clock duplex の表現

* **(a) `DuplexStream`**: full-duplex を型で保証。悪い点: WASAPI 等の独立 endpoint backend で実現不可能・実証なしの抽象・`open_duplex_stream` が多くの backend で `Unsupported` 固定になると契約が形骸化。
* **(b) 独立 stream + `clock_domain` 相関（採用）**: 既存 lifecycle を再利用・backend 正直性を保つ（同一 clock を証明できない backend は `UNKNOWN` のまま）。悪い点: 「同一 clock で駆動」を型保証できず consumer が一致判定を行う責務を負う — 判定規則を契約として明文化して緩和。
* **(c) capability flag で duplex 可能性を列挙**: open 前に判定可能。悪い点: backend が duplex を「サポートするが同一 clock かは不明」という 3 値以上の状態を flag では表現できず、結局 stream の domain 比較が必要。

### capture timestamp の語彙

* **(a) `MediaTime`+`MediaTimestamp`+`ClockDomainId`（採用）**: `OutputTimestamp` と同じ `MediaTime` 基盤で input/output の時刻比較が自然。`MediaTimestamp`/`TimestampProvenance`/`ClockDomainId`/`InputStamp` は全て既存型で語彙を増やさない。
* **(b) device 専用 stamp 型**: 型上の独立性。悪い点: `InputStamp`/`MediaTimestamp` と同じ情報を持つ第三の stamp 型になり、変換層と二重権威が生まれる。
* **(c) `InputStamp` 直載せ**: stamp が一級の型として届く。悪い点: `ticks`/`base`/`domain`/`provenance` の構成責務が adapter 実装ごとに分散し、`DEVICE_CLOCK_TIMEBASE` の一貫適用を各 adapter が担う（`capture_stamp()` 一箇所構成の利点を失う）。

### latency 報告

* **(a) `StreamLatency`（採用）**: buffer 含有・出典を型で表現し、二重計上禁止の要件を consumer が構造で判定できる。
* **(b) `Option<u64>` のみ**: 最小 API。悪い点: 「buffering を含むか不明」「推定値か報告値か不明」を語彙で表せず、consumer が無条件加算し得る。
* **(c) latency API なし**: 実測のみ。悪い点: backend が報告値を持つ場合に取得経路がなく、Issue 必須項を満たさない。

## More Information

- Issue [#187](https://github.com/scarecrow173/ruviform/issues/187) / [Synvelis #12](https://github.com/scarecrow173/synvelis/issues/12)（§16.5 latency 分類・二重計上防止、§17 実装順）
- 設計書 [../designs/audio-device-boundary-and-poc.md](../designs/audio-device-boundary-and-poc.md)（`output_*` 命名が予約した input 拡張点）
- [#59 timestamp 合成 ADR](chunk-outputcallbackinfo-timestamp.md)・[#188 timed MIDI ADR](issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md)（clock 権威と backend deferral の先例）
- `ruviform_media_core::timed_input`（`ClockDomainId`/`InputStamp`/`ClockAnchor`/`map_stamp_to_frame`）・`ruviform_media_core::observe`（`MediaTimestamp`/`TimestampProvenance`）
