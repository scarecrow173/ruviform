---
id: "SPEC-034QwZG7xkTZz51niLYsTZ"
type: "spec"
status: "approved"
title: "Issue #187 Audio Input/Duplex Port: input enumeration/open, capture timestamp/clock-domain/frame-position contract, latency report, RT-safe handoff"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/12"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/187"
  implemented-by: []
  depends-on:
    - "docs/designs/audio-device-boundary-and-poc.md"
    - "docs/adr/select-audio-device-backend.md"
    - "docs/adr/chunk-outputcallbackinfo-timestamp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/59"
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  defers: []
  deferred-by: []
---

# Issue #187 Audio Input/Duplex Port: input enumeration/open, capture timestamp/clock-domain/frame-position contract, latency report, RT-safe handoff

## What Should Be Built

Issue #187 が要求する「アプリケーション中立の Audio Input/Duplex・Capture Clock/Latency 契約」を、既存 output 中心の device boundary port（`ruviform_audio_core::device`）へ **additive** に拡張し、headless な mock input adapter で deterministic な capture→非 RT handoff を検証する。具体的には:

- **入力 Port 契約**: `DeviceEnumerator` への input 列挙（`input_devices`/`default_input_device`）、`Device` への入力側 describe/open（`default_input_format`/`supported_input_formats`/`open_input_stream`）、`InputStream`（start/stop/format/buffer_size/now/clock_domain/input_latency）と `InputDataCallback`/`InputErrorCallback`/`InputCallbackInfo`。`InputDevice` 独立 trait は設けず、`Device` が方向別の describe/open を持つ 1 ハンドル契約とする（cpal の `cpal::Device` と同じ model）。
- **Capture Clock / Timestamp 契約**: `InputTimestamp { callback, capture }` — `callback` は `OutputTimestamp::callback` と同じ device-clock 基準、`capture` は frame 0 が ADC で取得された実時刻を `Option<MediaTimestamp>`（time + `TimestampProvenance`）で運ぶ。`InputCallbackInfo` に `clock_domain: ClockDomainId`（同一 clock 判定・別 clock の識別用）と `frame_position: Option<u64>`（capture clock 上の frame 0 位置）を持たせる。`capture_stamp()` で `ruviform_media_core::timed_input` の `InputStamp` を構成し、既存の `ClockAnchor`/`map_stamp_to_frame` 権威へ接続する。
- **Latency 契約**: `StreamLatency { frames, coverage, provenance }` — `InputStream::input_latency`/`OutputStream::output_latency`（default `Ok(None)`）で報告。coverage は buffer 含有の区別（`HardwareOnly`/`HardwareAndBuffering`/`Unknown`）、provenance は値の出典（`Carried`/`Derived`/`Estimated`）。`capture`（実 ADC 時刻）が得られるとき consumer は latency 値を再加算しない — Synvelis #12 §16.5 の二重計上防止。
- **同一 clock duplex**: `DuplexStream` は本 PoC では作らず、独立 `InputStream`+`OutputStream` の `clock_domain` 一致（かつ `UNKNOWN` 非該当）で同一 clock を検出する方式を採用（比較・採否は ADR）。`DeviceError::ClockDiscontinuity` を追加し、frame position / timestamp の断絶を error callback で明示通知する。
- **RT-safe handoff**: callback 内で `AudioBufferView<'_, f32>`（read view・新規 `try_new` 追加）から capture block を取り込み、`ruviform_rt::channel` の bounded SPSC で非 RT へ配送する canonical pattern を mock PoC で検証（pool 方式は #185 の chunk-pool モデルと同型）。
- **cpal adapter の入力側表面**: `Host::input_devices`/`default_input_device` と `CpalDevice` の入力 format query は実装（output と同型の mapping）。`CpalDevice::open_input_stream` は `DeviceError::Unsupported` を返す正直な未対応（実 capture plumbing + 実機検証は後続 Issue）。

## Why Now

- Synvelis 録音基本設計 [#12](https://github.com/scarecrow173/synvelis/issues/12)（§16 Monitoring/Latency 合意・§17 実装順）が「同一 capture clock 上の複数入力取得」「非 RT Writer への安全配送」「出典付き latency/clock 契約」を要求しており、Ruviform 側の汎用入力面が未定義。
- Issue #187 は M0 監査（[discovery](../discovery/issue-194-m0-baseline-audit.md)）で「実機 duplex/capture 走行ログ」待ちの条件付き Blocked と分類されたが、本 spec が対象とする **設計+headless PoC** は実機不要であり先行可能。
- `timed_input`（`ClockDomainId`/`InputStamp`/`ClockAnchor`/`map_stamp_to_frame`、#188）と SPSC `channel`（`ruviform-rt`、#18/#22）の権威が揃っており、device capture clock を既存語彙へ接続する設計判断が可能。

## Users and Value

- **Synvelis（第一 consumer）**: 録音経路が device port 経由で capture block を timestamp/frame 位置付きで受領し、`ClockAnchor` 経由で engine frame へ写像できる。Track Arm/Punch/Comp の DAW 意味論は Synvelis 側責務のまま。
- **将来の adapter/backend 実装者**: input port が第二 backend の実装対象として文書化され、mock adapter が shape の実装可能性を証明する。
- **`audio-lens`（第二 consumer 分類対象）**: 本 spec の全変更は additive API 追加のみ（既存シンボルの削除・改名・意味変更なし）。分類は `no impact`（#219 contract の手順に基づき実装完了時の diff で検証）。synvelis は `impacted`（新規 API 提供）。なお `Device`/`DeviceEnumerator` への新規 required method は caller 側互換を壊さないが既存 *implementor* は source 更新を要する（`publish = false` の workspace 内影響のみ・全実装を本 change で更新済み）。

## Scope

### In Scope

- `ruviform-audio-core::device` への入力契約追加（上記列挙型・trait・callback・config・timestamp・latency 型）と `AudioBufferView::try_new` の追加。
- `OutputStream` への additive 拡張: `clock_domain()`/`output_latency()`（両方 default 実装付き — `ClockDomainId::UNKNOWN`/`Ok(None)`）。`DeviceError::ClockDiscontinuity` variant 追加。
- `ruviform-audio-device` の入力側非 RT 表面: input 列挙・`default_input_format`/`supported_input_formats` mapping、`open_input_stream` → `Err(DeviceError::Unsupported)`。
- mock input backend（`crates/ruviform-audio-device/tests/`）による deterministic capture→handoff 検証: 2ch 以上の scripted block、timestamp/frame position、同一 clock 判定、error 注入、`rt_` alloc-0 契約テスト。
- 既存 output 経路の回帰検証（`mise run test`/`test-rt` 全緑維持）。
- 設計比較の文書化: InputStream vs DuplexStream、timestamp 意味論（ADC vs callback）、latency 定義と coverage/provenance、fallback/rejection 規則。

### Out of Scope

- 実 backend での input stream 実装（cpal `build_input_stream` plumbing・interleaved→planar unpack・実 timestamp mapping）— `open_input_stream` は `Unsupported` を返し、後続実装 Issue へ分離。
- 実機での HW latency 計測・実 capture 走行・実 backend の同一 clock 確認（cpal `clock_domain` は `UNKNOWN` 維持）。
- `DuplexStream` 型の導入（比較は ADR で記録し、backend capability 実証後の後続 Issue へ defer）。
- Synvelis 固有の Track Arm/Punch/Loop/Take/Comp/Project File/Undo/Recording Writer、ファイル永続化契約。
- 異なる clock domain を持つ複数 device の完全な drift 補正保証（domain 識別可能性のみ提供）。
- `ruviform-audio`（engine）側の session/graph 統合、monitor routing、PDC との接続実装。

## Requirements

- Rust edition 2024・workspace MSRV（1.89）。`unsafe` 不使用。`publish = false` 継承。新規公開 enum は全て `#[non_exhaustive]`。
- **additive のみ**: 既存 output 契約（trait・型・意味）を破壊しない。`OutputStream` への新規 method は default 実装で後方互換を維持。
- **cpal 閉じ込め**: cpal 型・エラー・feature 名を公開 API に露出しない（adapter 層の既存原則）。
- **正直性**: timestamp・latency・clock domain・frame position の `None`/`UNKNOWN`/`Estimated` を成功として偽装しない。backend が報告できない値を adapter が捏造しない。`callback` instant を `capture` へ暗黙代入しない（callback 基準の推定値を ADC 時刻と混同しない）。
- **RT 契約**: `InputDataCallback` 内は RT 区間 — alloc/dealloc・ブロッキング・file/network I/O・無制限待機・panic を禁止（realtime ADR と同規則）。`InputErrorCallback` は RT-adjacent（no alloc・no block）。view は callback 返却まで有効な借用であり保持不可。
- **定義済み失敗面**: open/enumeration の失敗は `DeviceError` の定義済み variant で返す（`UnsupportedConfig`/`Unsupported`/`DeviceBusy`/`Backend` 分類）。stream 中の xrun・断線・clock 断絶は error callback で通知し RT をブロックしない。
- **二重計上防止**: `capture` が `Carried`/`Derived` で得られる場合、consumer は `StreamLatency` を timestamp 差分へ再加算しない（Synvelis #12 §16.5）。`includes buffering` 不明の latency を他成分と無条件加算しない。
- **同一 clock のみの保証**: timestamp 比較・整列・placement 補正の sample-accuracy 保証は同一 `clock_domain`（`UNKNOWN` 以外）内に限る。domain 不一致・`UNKNOWN` での加算補正は consumer 側で明示 rejection とする。

## Acceptance Criteria

- [ ] **AC-1（Port 契約・additive）**: `DeviceEnumerator::input_devices`/`default_input_device`、`Device::default_input_format`/`supported_input_formats`/`open_input_stream`、`InputStream`、`InputDataCallback`/`InputErrorCallback`/`InputCallbackInfo`/`InputTimestamp`/`InputStreamConfig`/`SupportedInputFormat`/`StreamLatency`/`LatencyCoverage` が `ruviform_audio_core::device` に追加され、object-safe + `Send` + `Box<dyn>` 交換が compile 検証される。既存 output API の挙動不変（既存テスト全緑）。
- [ ] **AC-2（Timestamp/Clock 契約）**: `InputTimestamp` が `callback`（host 観測時刻）と `capture`（ADC 実時刻・provenance 付き `Option`）を区別し、`InputCallbackInfo` が `clock_domain` と `frame_position`（`Option`、断絶時の stitch 禁止規則を doc 明記）を運ぶ。`capture_stamp()` が `InputStamp`（`DEVICE_CLOCK_TIMEBASE` + domain + provenance）を構成し `map_stamp_to_frame` へ接続可能。`capture: None` を fallback 合成で埋めない。
- [ ] **AC-3（mock deterministic capture）**: mock input adapter が 2ch 以上の scripted block を `frame_position`/timestamp 付きで callback へ届け、非 RT 側が block 単位で内容・境界・連続性を検査できる（pattern fill で data integrity を検証）。
- [ ] **AC-4（同一 clock 判定）**: 同一 `clock_domain`（`UNKNOWN` 以外）を持つ input/output stream 対が「同一 clock」と検出され、異 domain・`UNKNOWN` が混同されない。mock が共有 domain で duplex-equivalent 構成を示す。
- [ ] **AC-5（RT-safe bounded handoff）**: callback 内で capture block を pool から取得→充填→bounded SPSC `try_push` で非 RT へ渡し、drain 済み block を recycle する往復が、全 block を順序通り欠落なく配送する。`assert_no_alloc` 計測窓内の callback 経路 alloc が 0。channel 満杯時は定義済みの overflow 記録（block/panic しない）。
- [ ] **AC-6（Latency 契約）**: `StreamLatency` が frames+coverage+provenance を運び、backend 未報告は `Ok(None)`、coverage 不明は `Unknown`、推定値は `Estimated`/`Derived` で報告される。`input_latency`/`output_latency`（default `Ok(None)`）が RT 非適格の control API として文書化される。
- [ ] **AC-7（明示通知・定義済み失敗面）**: mock が `Xrun`/`StreamInvalidated`/`ClockDiscontinuity`/`Backend("device_changed")` advisory を error callback へ配送し、`open_input_stream` が format 不一致を `UnsupportedConfig`、未対応 backend 機能を `Unsupported` で返すことを検証。`DeviceError::ClockDiscontinuity` が `Copy`+`Display` 契約を持つ。
- [ ] **AC-8（cpal adapter 入力表面）**: cpal `Host` が `input_devices`/`default_input_device` を実装し、`CpalDevice` が入力 format query を output と同型で mapping する。`CpalDevice::open_input_stream` は `DeviceError::Unsupported` を返し、実機 capture を偽装しない。`list_devices` example への入力列挙追加は任意。
- [ ] **AC-9（Output 回帰）**: `mise run test`/`test-rt` で既存 output 経路（mock_backend・convert/stream 単体テスト・lib.rs device_contract）が全緑のまま。
- [ ] **AC-10（未検証の明示）**: 実機依存項目（cpal input stream・HW latency 計測・実 backend 同一 clock・他 OS）と検証コマンド結果が impl record と issue コメントで区別記録される。

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `DuplexStream` 型 | 独立 stream + `clock_domain` 一致で同一 clock 検出。結合 API は作らない | backend capability 実証（JACK/CoreAudio aggregate 等）後の後続実装 Issue | 不要な抽象を先行確定すると実 backend に合わない可能性 |
| cpal input stream plumbing | `Err(Unsupported)` で正直に未対応 | #187 後続実装 Issue（実機検証 lane 付き） | 未検証 plumbing を「動く」と偽装する |
| 異 clock drift 補正 | domain 識別可能性のみ提供、補正保証なし | engine 側 resample/drift 設計 Issue | 別 clock の加算補正で誤配置 |
| input→output/monitor routing・capture tap | port 境界の検証のみ、graph 統合なし | #184/#185/#195 の graph 層 + engine Issue | 録音経路の二重設計 |
| `OutputStream` の `playback`/`output_latency` 実測 wiring | default `Ok(None)`/`UNKNOWN` のまま | cpal adapter の latency 実装 Issue | latency 値の捏造 |
| `frame_position` の backend 提供手段 | mock で規則検証、cpal では未提供（`None`） | cpal/他 backend の実測調査 Issue | なし（`Option` で正直に表現済み） |

## Sources and Relations

- 一次要件: GitHub Issue [#187](https://github.com/scarecrow173/ruviform/issues/187)（`relations.source`/`implements`）。M7 実装境界は [#193](https://github.com/scarecrow173/ruviform/issues/193)、consumer 要件は [synvelis #12](https://github.com/scarecrow173/synvelis/issues/12)（§16 Monitoring/Latency・§17 実装順）。
- 上流（`relations.derives-from`）: [M0 baseline 監査](../discovery/issue-194-m0-baseline-audit.md)（実測 baseline と Blocked 条件の正本）。
- 前提（`relations.depends-on`）: [device boundary 設計](../designs/audio-device-boundary-and-poc.md)（port の 3-trait 形状と `output_*` 拡張点）、[backend 選定 ADR](../adr/select-audio-device-backend.md)、[#59 timestamp 合成 ADR](../adr/chunk-outputcallbackinfo-timestamp.md)（per-chunk instant offset の先例）。
- 関連（`relations.related`）: [RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)、[#188 spec](issue-188-timed-midi.md)（`ClockDomainId`/`InputStamp`/`ClockAnchor` 語彙の正本）、[synvelis-driven 設計](../designs/synvelis-driven-target-architecture-and-migration.md)、[`AGENTS.md`](../../AGENTS.md)。
- 参考（`relations.references`）: [#59](https://github.com/scarecrow173/ruviform/issues/59)・[#184](https://github.com/scarecrow173/ruviform/issues/184)・[#185](https://github.com/scarecrow173/ruviform/issues/185)・[#188](https://github.com/scarecrow173/ruviform/issues/188)・[#190](https://github.com/scarecrow173/ruviform/issues/190)・[synvelis #18](https://github.com/scarecrow173/synvelis/issues/18)・[#22](https://github.com/scarecrow173/synvelis/issues/22)。
