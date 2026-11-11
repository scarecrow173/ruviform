---
id: "IMPL-034R251StageWcl0seoutX"
type: "impl"
status: "completed"
title: "Issue #251 Stage W real-device closeout — Windows/WASAPI 実機 lane 証拠化"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/251"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-device/src/wasapi_input.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/tests/real_input_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/issue-251-stage-w-real-device-closeout.md"
      - type: "file"
        path: "docs/designs/issue-251-stage-w-real-device-closeout-design.md"
      - type: "file"
        path: "docs/plans/issue-251-stage-w-real-device-closeout-plan.md"
      - type: "file"
        path: "docs/tasks/issue-251-task1-stage-w-real-device-closeout.md"
      - type: "file"
        path: "docs/test-specs/issue-251-stage-w-real-device-closeout.md"
      - type: "file"
        path: "docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md"
  implements:
    - "docs/tasks/issue-251-task1-stage-w-real-device-closeout.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-251-stage-w-real-device-closeout.md"
    - "docs/designs/issue-251-stage-w-real-device-closeout-design.md"
    - "docs/plans/issue-251-stage-w-real-device-closeout-plan.md"
    - "docs/test-specs/issue-251-stage-w-real-device-closeout.md"
    - "docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-187-audio-input-port-additive-input-contract-cpal-input-surface-mock-captu.md"
    - "docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-251-stage-w-real-device-closeout-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #251 Stage W real-device closeout — Windows/WASAPI 実機 lane 証拠化

## Summary

Issue #251 Stage W（Windows/WASAPI）の実機 closeout。`tests/real_input_e2e.rs` を拡張し、
Audient iD4（WASAPI、`Analogue 1/2` endpoint、44.1 kHz F32 Stereo）で open→capture→
handoff・timestamp 整合・実効 latency・`input_latency()` 応答・duplex clock domain・
UNKNOWN rejection・format 不一致拒否・device 切断 typed error を実機観測した。
実走で **callback timestamp の provenance 欠陥**（`IAudioClock::GetPosition` snapshot 由来の
epoch-0/stale instant）を発見し、`wasapi_input.rs` の delivery instant を
`QueryPerformanceCounter` 直読みへ修正して全行 Pass を確認した。
結果は **「Stage W（Windows+WASAPI）Pass / Stage M・L は NotRun — Issue 全体の
Cross OS 完了ではない」**。

## Context

先行 headless slice（`issue-251-audio-device-real-backend-input-plumbing-headless-slice.md`）は
open→SPSC→`InputDataCallback` 経路を実装し、実機実行を全行 `NotRun` で残した。
Issue #565/PR #567 が WASAPI-native input path（`frame_position`・capture timestamp・
`GetStreamLatency`・`StreamInvalidated` 正規化）を main に統合済み。本 slice はその
post-merge 状態で残存受入項目を実測し証拠化する closeout（spec/design/plan/task/
test-spec は `issue-251-stage-w-real-device-closeout` 系列）。

## Implementation

### Changes

- `crates/ruviform-audio-device/tests/real_input_e2e.rs`: 4 検証行を拡張/新設。
  - `real_input_stream_captures_planar_audio`: `frame_position` 全 callback `Some` +
    contiguity（gap ≤ `ClockDiscontinuity`）の WASAPI assert、`capture` instant
    ordering（`capture ≤ callback`）違反計数、実効 input latency
    （`callback - capture` min/max/mean、±1 s clamp 付き）記録、`input_latency()`
    の WASAPI 契約 assert（`Ok(Some(StreamLatency))` — `Ok(None)` は native path
    退化、0 frames 捏造は検出対象）、violation 診断計装（instant min/max・delta trace）。
  - `real_duplex_streams_agree_on_clock_domain`: 既存同一 domain 観測に加え
    `same_clock_domain(real, UNKNOWN) == false`・`same_clock_domain(UNKNOWN, UNKNOWN)
    == false` の rejection/fallback verdict を実測。
  - `real_input_open_rejects_unsupported_configs`（新設）: device の supported 空間から
    動的に無効 config を構成し `Err(DeviceError::UnsupportedConfig)` のみを assert
    （非 F32 ×2・rate 上下限超・未 cover layout・`Fixed(0)`・bounded bound 超過 —
    `Unknown` bound 時は行 skip）。
  - `real_input_device_removal_notifies_typed_error`（新設、対話式）:
    `RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS` 未設定時は手順表示 SKIP（二重ゲート）、
    armed 時は baseline callback 確認→`CONOUT$` 抜去/既定 endpoint 切替指示→
    window 内 `StreamInvalidated` 観測→post-removal `stop()`/`now()`/`buffer_size()`
    記録。観測なしは `NOT-EXERCISED`（ unattended を成功扱いしない）。
- `crates/ruviform-audio-device/src/wasapi_input.rs`: 実走で発覚した timestamp
  provenance 欠陥の修正（詳細は下記）。`timestamp.callback` を
  `IAudioClock::GetPosition` snapshot 由来から `perf_counter_instant(qpc_frequency)`
  （packet 処理時点の QPC 直読み）へ変更。`IAudioClock` service 取得・
  `clock_callback_instant` を削除し、`RunContext` に `qpc_frequency` を追加。

### Decisions During Implementation

- **callback instant の provenance 修正**（実機発見 → 根治）。cpal 準拠の
  `GetPosition` qpc は「position counter が最後に更新された瞬間」の snapshot であり、
  (a) stream 最初の packet では clock 未開始のため `0` を返し callback instant が
  epoch 0 に落ちる（delta = −319 936 秒 = −capture instant そのもの）、
  (b) 以後も packet record instant より最大 ~1 ms 遅れるため `capture > callback` が
  193 中 47 packet で発生した。port 契約は callback を「host-observed delivery
  instant」と規定する — packet 処理時点の `QueryPerformanceCounter` 直読みが
  定義上そのものであり、`now()` と同一 epoch/grid（100 ns）で、record instant が
  delivery instant を超えることは起こり得ない。修正後: order violations=0、
  実効 latency は全 callback で ~10.04–10.12 ms（1 period pipeline delay）に収束。
  `audio_clock` は他の用途を持たないため service 取得ごと削除（cpal との乖離は
  意図的 — #565 での frame_position 供給と同じく「契約が要求する正直さ」を優先）。
- **観測対象として残す診断計装**: violation 時の capture instant min/max と
  先頭 256 callback の delta trace を `--- violations > 0` の場合のみ出力。
  再発時の「bogus timebase の同定」を可能にする証拠機構であり、Pass 時は無出力。
- **`input_latency()` は値 assert でなく契約 assert**: `frames` の絶対値は
  driver 依存（本機では `GetStreamLatency` 由来 0 frames を報告 — sub-frame 値の
  切捨て）のため、`Ok(Some)` と `coverage/provenance` のラベル開示のみ規定。
- **removal 検証の trigger 二経路化**: 物理抜去は人的操作が前提だが、本機では
  non-elevated session で `SetDefaultEndpoint` による既定 capture endpoint 切替が
  `OnDefaultDeviceChanged` → `StreamInvalidated` を同一 typed error で発火することを
  確認した。prompt/test-spec は両 trigger を明示 — 「`AUDCLNT_E_DEVICE_INVALIDATED`
  物理経路」と「default-device monitor 経路」は別記録とし、後者を本実績とする。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

- `IAudioClock::GetPosition` snapshot を callback instant に使い続ける案:
  epoch-0/stale を consumer 側で許容する設計は port 契約（`capture ≤ callback` の
  ordering は「device clock が garbage を報告した」以外では破られない）に反するため。
- 物理抜去を待って `NOT-EXERCISED` 記録で終える案: 既定 endpoint 切替で同一
  typed error 経路を確実に発火できると判明したため、実観測を優先した。

## Validation

環境: Windows 11 x64（Audient iD4 `VID_2708&PID_0009`、`Present=True`、driver `audientusbaudio_sc`）。
toolchain: mise 管理 `rust 1.99.0`（workspace MSRV 1.89）。canonical は mise task、
`--nocapture` 付き直接 cargo 実行は補助証拠として併記。

### Canonical lane

| コマンド | exit | 結果 |
| -------- | ---- | ---- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check` | 0 | Pass |
| `mise run lint` | 0 | Pass |
| `mise run test` | 0 | Pass（1 回目に `ruviform-plugin-host` `l1_proxy_e2e::composite_e2e::spawn_failed_attempt_record_captures_stderr` が flaky 失敗 `Spawn vs Live` — 本件範囲外・再実行で pass。in-scope crate の影響なし） |
| `mise run test-rt` | 0 | Pass |
| `mise run build` | 0 | Pass |
| `mise run lint-audio-real-input` | 0 | Pass（`-D warnings`、MSRV lint 含む） |
| `mise run test-audio-real-input` | 0 | **4/4 Pass**（Audient iD4 実機） |

### 実機証拠（`--nocapture` 補助証拠、canonical lane と同一 binary）

**capture**（`real_input_stream_captures_planar_audio`）: host WASAPI・device
`Analogue 1/2 (2- Audient iD4)`・44.1 kHz Stereo F32・`BufferSize::Fixed(441)`。
callbacks=192・total frames=84672・frames/callback=441 固定・channel/domain/timestamp
regressions=0・capture `Some` 192/192・**order violations=0**・実効 input latency
min=10 038 500 ns max=10 117 300 ns mean=10 059 264 ns・frame_position `Some` 192/192
gaps=0（first=0 last=84231）・`clock_domain`=`ClockDomainId(2)`（MONOTONIC）・
`input_latency()`=`Some(StreamLatency{frames:0, coverage:Unknown, provenance:Carried})`・
`buffer_size()`/`now()` pre/post `Ok`・non-finite=0・xrun=0・error events=0。

**duplex**（`real_duplex_streams_agree_on_clock_domain`）: input/output とも
`ClockDomainId(2)`・`same_clock_domain(input,output)=true`・
`same_clock_domain(2,UNKNOWN)=false`・`same_clock_domain(UNKNOWN,UNKNOWN)=false`・
`input_latency()=Some(...)`・`output_latency()=None`。

**format mismatch**（`real_input_open_rejects_unsupported_configs`）: 7 行すべて
`Err(UnsupportedConfig)` — S16・F64・88 200 Hz（>max 44 100）・1 Hz（<min 44 100）・
Mono（未 cover layout）・`Fixed(0)`・`Fixed(921)`（>bound 441）。

**device loss**（`real_input_device_removal_notifies_typed_error`、armed
`RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS=120`、直接 cargo 実行）: baseline 94 callbacks
確認後、既定 capture endpoint を `Analogue 1/2` → `Loop-back 1/2`（同一 iD4 の別
endpoint、`IPolicyConfig::SetDefaultEndpoint` role 0/1/2 全て `S_OK`、管理者権限不要）
へ切替。error events=3（**StreamInvalidated=2**・ClockDiscontinuity=1・last kind
code=8）が error callback に到達 → removal notification verified。
post-removal `stop()=Ok`・`now()=Ok`・`buffer_size()=Ok(Fixed(441))`。既定 endpoint は
検証後 `Analogue 1/2` へ復元済み。

### 実機検証 NotRun / 観測なし matrix

| 検証行 | 状態 | 根拠 |
| ------ | ---- | ---- |
| `mise run test-audio-real-input` | **Pass**（4/4） | 上記実機証拠 |
| 物理 USB 抜去による `AUDCLNT_E_DEVICE_INVALIDATED` 経路 | **NotRun（代替実施）** | 本環境は unattended shell で物理抜去不能。同一 typed error を生じる既定 endpoint 切替経路（default-device monitor→`StreamInvalidated`）を実観測。物理経路は対話式 lane が prompt で案内済み・手順を文書化 |
| xrun / underrun | **観測なし** | 2 s capture・1 s baseline で `XRUNS=0`/`error events=0` — 発生しない環境での正直な記録。誘発負荷は lane 責務外 |
| `ClockDiscontinuity` | 観測あり（removal 時 1 件） | reroute による position epoch reset の正当通知。capture 行では 0 |
| OS capture permission denied | **NotRun** | permission を拒否する制御は環境に存在せず。deny 経路は `error.rs` の headless 検証済み |
| same-clock duplex の frame 相関（時刻整合 math） | **NotRun** | domain 一致と `same_clock_domain` verdict は実測済み。frame 相関は consumer（Synvelis #115/#117）と Issue #315 の管轄 |
| `output_latency()` 実値 | **NotRun**（`Ok(None)` 観測） | cpal 経路の portable query 非存在に対する正直応答として記録 |
| Stage M（macOS/CoreAudio）・Stage L（Linux/ALSA 他） | **NotRun** | 本 slice 対象外 — Issue 全体の Cross OS 完了を宣言しない |

## Risks

- `perf_counter_instant` を callback instant に変更したことで callback/capture 同一
  epoch 上の ordering が構造的に保証される反面、`GetPosition` snapshot が持つ
  「stream 内部の位置 snapshot との一致」は捨てた — port 契約上 callback は
  delivery instant であり snapshot instant ではないため正当な交換だが、cpal 由来
  parity を捨てた点は将来の移植差分として記録する。
- `StreamLatency{frames:0}` は `GetStreamLatency` が sub-frame 値を返す本 driver の
  実測値 — 0 そのものの妥当性は driver 依存であり、latency 絶対値の真偽は
  consumer（Synvelis #117 の実測）側で評価されるべき観測値。
- 対話式 removal lane は `StreamInvalidated` の到達のみ assert し、`AUDCLNT_E_`
  物理経路は未観測 — 正規化の両経路が同一 variant に写像されることは
  `error.rs` の headless 検証と設計上の写像でカバーされるが、物理実測は残課題。

## Follow-ups

- Issue #315（generic multi-clock/drift capability）へ、`callback`/`capture` 同一
  epoch 化後の drift 観測（同一 QPC clock 上の packet-to-packet jitter）を接続材料
  として引き継ぐ。
- Synvelis #115（same-clock capture raw fidelity）・#117（latency OS 実測）が
  consumer 側受入 — 本実績値（実効 latency ~10 ms・`MONOTONIC` duplex=true）を参照。
- 物理 USB 抜去経路の実観測は operator 付き環境で実施する残課題
  （`RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS` で再現可能）。
- Stage M/L は別 environment での closeout Issue が必要（本 Issue の宣言範囲外）。
