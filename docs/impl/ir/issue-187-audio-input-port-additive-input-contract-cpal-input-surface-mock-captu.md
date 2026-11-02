---
id: "IMPL-034Qx80dtG1QqpllKScvin"
type: "impl"
status: "completed"
title: "Issue #187 Audio Input Port: additive input contract + cpal input surface + mock capture->handoff RT PoC"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/187"
  changes:
    added:
      - "crates/ruviform-audio-device/tests/mock_input_backend.rs"
    modified:
      - "crates/ruviform-audio-core/src/device.rs"
      - "crates/ruviform-audio-core/src/buffer.rs"
      - "crates/ruviform-audio-core/src/lib.rs"
      - "crates/ruviform-audio-device/src/convert.rs"
      - "crates/ruviform-audio-device/src/device.rs"
      - "crates/ruviform-audio-device/src/host.rs"
      - "crates/ruviform-audio-device/Cargo.toml"
      - "crates/ruviform-audio-device/tests/mock_backend.rs"
      - "crates/ruviform-audio/src/lib.rs"
      - "crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs"
      - "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - "docs/specs/issue-187-audio-input-port-capture-clock.md"
      - "docs/adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md"
      - "docs/designs/issue-187-audio-input-port-capture-clock.md"
      - "docs/plans/implement-issue-187-input-port-headless-poc.md"
      - "docs/tasks/issue-187-t1-input-port-contract.md"
      - "docs/tasks/issue-187-t2-cpal-input-surface.md"
      - "docs/tasks/issue-187-t3-mock-input-poc.md"
      - "docs/test-specs/issue-187-input-port-tests.md"
  implements:
    - "docs/tasks/issue-187-t1-input-port-contract.md"
    - "docs/tasks/issue-187-t2-cpal-input-surface.md"
    - "docs/tasks/issue-187-t3-mock-input-poc.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-187-input-port-headless-poc.md"
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
    - "docs/adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md"
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-187-input-port-tests.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #187 Audio Input Port: additive input contract + cpal input surface + mock capture->handoff RT PoC

## Summary

`ruviform_audio_core::device` へ additive な入力 contract（`DeviceEnumerator` input 列挙・`Device` input format/open・`InputStream`/`InputDataCallback`/`InputCallbackInfo`/`InputTimestamp`/`StreamLatency`/`LatencyCoverage`/`ClockDiscontinuity`）を追加し、cpal adapter に input 列挙・format mapping と正直な `Unsupported` open を実装し、mock input backend で deterministic capture→handoff の有界・無割当 RT PoC を検証した。実機 cpal input stream・HW latency 実測は範囲外で未検証として残す。

## Context

Spec `docs/specs/issue-187-audio-input-port-capture-clock.md`・ADR `docs/adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md`（`Device` 拡張・独立 stream + `clock_domain` 相関・`MediaTime`/`MediaTimestamp`/`StreamLatency` 語彙）・design `docs/designs/issue-187-audio-input-port-capture-clock.md`・plan `docs/plans/implement-issue-187-input-port-headless-poc.md` に基づく。タスク T1（contract）→ T2（cpal adapter）→ T3（mock PoC）。

## Implementation

### Changes

- `crates/ruviform-audio-core/src/device.rs`: `DeviceEnumerator::input_devices()`/`default_input_device()`、`Device::supported_input_configs()`/`default_input_config()`/`open_input_stream()`、`InputStream` trait（`start`/`stop`/`format`/`buffer_size`/`now`/`clock_domain`/`input_latency`）、`InputDataCallback`/`InputErrorCallback`/`InputCallbackInfo`（`timestamp`・`clock_domain`・`frame_position: Option<u64>`・`capture_stamp()`）、`InputTimestamp`（`callback: MediaTime` + `capture: Option<MediaTimestamp>`）、`StreamLatency`（`frames`/`coverage`/`provenance`）、`LatencyCoverage`（`HardwareOnly`/`HardwareAndBuffering`/`Unknown`）、`DeviceError::ClockDiscontinuity` を追加。`OutputStream` に default `clock_domain()`（`ClockDomainId::UNKNOWN`）と `output_latency()`（`Ok(None)`）を additive に追加。既存 output API は不変。
- `crates/ruviform-audio-core/src/buffer.rs`: `AudioBufferView::try_new(&[S], channels, frames)` を `AudioBufferViewMut::try_new` と同じ検証（0 channel 拒否・`channels * frames` 乗算 overflow 検査・スライス長完全一致）で追加。無割当。
- `crates/ruviform-audio-core/src/lib.rs`: 新公開型を re-export。
- `crates/ruviform-audio-device/src/convert.rs`: output 専用だった cpal config/range 検証・format mapping を input/output 共有化。
- `crates/ruviform-audio-device/src/device.rs`・`host.rs`: cpal `input_devices()`/`default_input_device()` 列挙と `supported_input_configs`/`default_input_config` mapping を実装。`open_input_stream` は `Err(DeviceError::Unsupported)` を返す（実機 capture は本タスク範囲外）。cpal 型・エラー・feature は公開 API に露出しない。
- `crates/ruviform-audio-device/tests/mock_input_backend.rs`（新規）: scripted mock input stream（deterministic data・>=2ch・callback/capture timestamp・frame position・clock domain・latency report・error injection・lifecycle）と、preallocated block pool + SPSC の bounded capture→handoff。queue full は block/panic せず drop として観測。RT テスト `rt_input_callback_handoff_allocates_nothing`・`rt_input_callback_queue_pressure_stays_bounded` を含む。
- `crates/ruviform-audio-device/tests/mock_backend.rs`・`crates/ruviform-audio/src/lib.rs`・`crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`: trait 拡張に伴う既存 `Device`/`DeviceEnumerator` 実装を input 側は正直な `Unsupported`/`None` 返却で更新。output 挙動は不変。
- `crates/ruviform-audio-device/Cargo.toml`: `ruviform-rt` を dev-dependency のみに追加（`cargo tree --edges normal` で normal deps に非含有を確認済み）。
- docs: spec/ADR/design/plan/task×3/test-spec/impl record を作成し、managed index（各 `docs/*/README.md`）を再生成。

### Decisions During Implementation

- `OutputStream::output_latency()`/`clock_domain()` は default method 化し、既存 output 実装の source 互換を維持（trait に新規 required method を足すと全実装が壊れるため）。
- 既存 `OutputCallbackInfo`（公開フィールド構造体）には capture timestamp field を追加しない — public-field struct への field 追加は source-breaking のため。入力側は新規 `InputTimestamp` で設計自由があった。
- `InputCallbackInfo::frame_position` は callback 回数から合成せず `Option<u64>` のまま。同一 clock 判定は `ClockDomainId` 等値かつ非 `UNKNOWN`（`UNKNOWN == UNKNOWN` は同一 clock 主張にしない）を API doc と mock テストの両方に固定。
- `InputTimestamp.callback` と `capture` を分離し、capture 欠損時に callback 時刻を代用しないことを型で強制（`capture: Option<MediaTimestamp>`、`capture_stamp()` は非負時のみ `Some`）。
- `open_input_stream` を `Unsupported` で正直に失敗させ、enumeration/format surface のみ実装 — 実 capture を装わない。
- review 指摘を受け: cpal `supported_*_configs` の `ErrorKind::UnsupportedOperation` を「その方向の能力なし」= contract の `Ok(vec![])` へ正規化（failed query の `Err` と区別 — adapter の untrusted-boundary 責務）。
- 同: `capture_stamp()` で `capture.time.base() != DEVICE_CLOCK_TIMEBASE` を `None` へ（foreign-base tick の無断 relabel を防ぐ）。
- 同: same-clock 規則を `same_clock_domain()` に集約、PoC handoff の oversized delivery を panic ではなく clamp + drop count、held block を `in_flight` で観測、error callback の consumer pattern を bounded SPSC sink とした。

## Validation

- `mise run fmt-check`: pass。
- `mise run check`（`cargo check --workspace --all-targets`）: pass。
- `mise run lint`（`cargo clippy --workspace --all-targets -- -D warnings`）: pass。
- `mise run test`（`cargo test --workspace`）: pass（114 test binaries・全件 ok）。`mock_input_backend` の deterministic capture・data integrity・delivered-info 一致・timestamp/frame position・clock-domain 等値規則・latency・error injection・bounded handoff・drop/in-flight 観測の全 12 test が green。
- `mise run test-rt`（`cargo test --workspace rt_`）: pass・exit 0（80 test-result 行全件 ok・`rt_input_callback_handoff_allocates_nothing`/`rt_input_callback_queue_pressure_stays_bounded` 含む）。検証中盤に観測した exit 1 は `cargo tree`/`grep -c` 補助 pipeline（0 match → grep exit 1）の合成 artifact であり task 自体の失敗ではなかった — 本 run で単独 exit 0 を確認。
- `cargo tree -p ruviform-audio-device --edges normal`: `ruviform-rt` は normal deps に非含有（dev-dep のみ）を確認。
- `git merge origin/main`: `Already up to date.`（最終検証前に実施）。
- review-protocol（承認代替）: Reviewer A（cooperative）/Reviewer B（adversarial）を独立 dispatch → validate → judge。verdict: **ACCEPT WITH NON-BLOCKING NOTES**。CONFIRMED 指摘は全て修正済み（下記）。
  - P1: cpal `supported_*_configs` の `ErrorKind::UnsupportedOperation`（= その方向の能力なし）を contract の `Ok(vec![])` へ正規化（input/output 両方 — 従来 `Err(Unsupported)` として failed query に見せていた）。
  - P1: `mise run test-rt` exit 1 の原因調査 → 上記の通り shell pipeline artifact と判明・task 単独で exit 0 を確認。
  - P2: `CaptureBlock::copy_from` の oversized delivery panic 経路を clamp + drop count へ（RT 契約の no-panic を PoC pattern として保証）。
  - P2: `capture_stamp()` が foreign-base `MediaTime` を `DEVICE_CLOCK_TIMEBASE` に無断 relabel していた → base 照合し不一致は `None`。
  - P2: same-clock 規則（equal AND non-`UNKNOWN`）を `same_clock_domain()` helper として集約・mock test と API doc から参照。
  - P2/P3: `scripted_sample` の data integrity assert・delivered-info vs script の assert・`ClockDiscontinuity` の Display sweep 追加・`AudioBufferView::try_new` reject unit test・error callback を bounded SPSC sink へ（RT-adjacent 規則に整合）・`Backend("device_changed")` を error injection script に追加・held `working` block を `in_flight` counter で観測可能化・`capture_stamp` foreign-base/negative reject test・`FakeInputStream::start` の started flag 順序修正・`InputTimestamp`/`InputCallbackInfo`/`StreamLatency`/`SupportedInputFormat`/`InputStreamConfig`/`InputDataCallback`/`InputErrorCallback`/`Box<dyn InputStream>` の Send/Copy pin 追加・crate doc の media-core import list/`Box<dyn InputStream>`/`DEVICE_CLOCK_TIMEBASE` carrier 記述/`OutputStream::output_latency` provenance 語彙/spec の additive 文言/test-spec fixture 値/design の scratch 記述を現状に整合。

## Related Experiments

### Adopted

- `same_clock_domain()` を free function として port module に集約 — `ClockDomainId` は media-core 型のため method 追加は不可、consumer ごとの `a == b` 誤用を防ぐ authoritative rule として採用。
- error callback の consumer 側 pattern として bounded SPSC sink（`DeviceError` は `Copy` + `&'static str` payload で allocation-free）— `Mutex<Vec>` では RT-adjacent 規則に反するため。

### Rejected

- `OutputCallbackInfo` への capture timestamp field 追加 — public-field struct への field 追加は source-breaking のため見送り（既存 output 契約非破壊を優先）。
- `DuplexStream` 専用型 — 独立 stream + `clock_domain` 相関で表現可能（ADR）。

## Risks

- 実機検証は全て未実施・範囲外: cpal `build_input_stream` plumbing、実 HW capture timestamp の provenance/精度、HW latency 実測、実機での同一 clock duplex（同一 `ClockDomainId` が実際に発行されるか）は mock での契約検証のみで real backend 未検証。
- `test-rt` mise タスクは RT テスト自体は pass するが wrapper の exit code が 1 を返す状況がある（補助チェック出力起因）。CI `rt-verify` への影響を別途確認要。
- `UNKNOWN` clock domain の backend が将来複数出た場合、同一 clock 非主張ルールは契約で保護されるが、cpal adapter が `UNKNOWN` 以外の domain id を発行する設計は real backend 実装時に要確定。

## Follow-ups

- cpal `build_input_stream` plumbing（real `InputStream` 実装）— `open_input_stream` の `Unsupported` を実装で置換。
- HW latency 実測経路（`StreamLatency` の `coverage`/`provenance` を実測値で埋める backend）。
- 実同一 clock duplex 検証（独立 `InputStream`/`OutputStream` の `ClockDomainId` 相関の real backend 観測）。必要なら `DuplexStream` 追加は別 ADR。
- engine（Synvelis #12 側）への multi-track capture 統合 — capture→handoff→non-RT writer 接続。
