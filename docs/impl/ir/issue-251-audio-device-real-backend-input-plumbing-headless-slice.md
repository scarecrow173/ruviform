---
id: "IMPL-034UK2UedkRURdGA03P7k9"
type: "impl"
status: "completed"
title: "Issue #251 audio-device real backend input plumbing (headless slice)"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/251"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-device/tests/real_input_e2e.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-device/src/convert.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/device.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/host.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/tests/mock_input_backend.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
    - "docs/impl/ir/issue-187-audio-input-port-additive-input-contract-cpal-input-surface-mock-captu.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-187-audio-input-port-additive-input-contract-cpal-input-surface-mock-captu.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
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

# Issue #251 audio-device real backend input plumbing (headless slice)

## Summary

`ruviform-audio-device` の `open_input_stream` を #187 の正直な `Err(DeviceError::Unsupported)` placeholder から実 cpal input stream 構築へ置き換え、`build_input_stream::<f32>` → interleaved→planar `AudioBufferView` → `InputDataCallback` の経路を RT 契約（open 時一括確保・callback 内無割当/無ブロック）どおりに実装した。`InputTimestamp`（callback/capture 分離・`DEVICE_CLOCK_TIMEBASE`）・`clock_domain`（host 監査済み profile）・`input_latency`（`Ok(None)` 正直応答）・error callback 経由の typed `DeviceError` 通知を接続し、same-clock duplex 判定の分岐を `clock_profile` で実装した。実機検証は本 slice 対象外 — opt-in lane（`real-device` feature + `#[ignore]` E2E + mise tasks）の構造のみ整備し、全実機行は `NotRun` として記録する。

## Context

Issue #187（impl record `docs/impl/ir/issue-187-...md`・design `docs/designs/issue-187-audio-input-port-capture-clock.md`）で input port 契約・cpal input 列挙/format surface・mock capture→handoff PoC が完了し、`clock_domain`/`frame_position`/`input_latency` の実値・実機 capture は「cpal adapter の実機調査 Issue」として #251 へ deferred されていた。本 record はその headless 実装 slice。spec/plan/task doc は #251 では新設せず、issue 本文と #187 の設計判断を直接の根拠とする。

## Implementation

### Changes

- `crates/ruviform-audio-device/src/convert.rs`: negotiation core を方向中立化（`negotiate_impl` + `RangeFields` を `SupportedOutputFormat`/`SupportedInputFormat` 双方に実装、`negotiate_input` 追加）。input 方向の `read_interleaved` を追加 — backend の interleaved `f32` を channel-major planar scratch へ展開し、`frames_capacity` 超過時は chunk 分割 + per-chunk timestamp 合成（`chunk_input_timestamp_info`、overflow 時は元 instant 保持）。末尾の端数サンプルは whole frame に満たないため view に露出しない。
- `crates/ruviform-audio-device/src/stream.rs`: `CpalInputStream`（`InputStream` 実装: `start`/`stop`/`format`/`buffer_size`/`now`/`clock_domain`/`input_latency`）と `InputCallbackState`（data callback が単独所有する callback+scratch+clock profile）を追加。`map_input_timestamp` で cpal `InputStreamTimestamp` を `InputTimestamp` へ写像 — `callback` は常に値を持ち、`capture` は変換不能時・`StreamInstant::ZERO`（backend "未報告" sentinel）時に `None`。`StreamInstant::as_nanos()` の `i64` overflow は epoch-delta fallback（output と同一規則）。`CpalOutputStream` にも `clock_domain` を追加し input/output が同一 host profile を報告する。
- `crates/ruviform-audio-device/src/device.rs`: `ClockProfile`/`clock_profile(host_name)` を追加 — cpal 0.18.2 host source 監査に基づき、共有 monotonic clock を持つ `WASAPI`（QPC）/`CoreAudio`（`mach_absolute_time`）/`PipeWire`（graph clock）は `ClockDomainId::MONOTONIC`、per-device clock の `ALSA` と未監査 host は `UNKNOWN` を報告（`same_clock_domain` が `UNKNOWN==UNKNOWN` を same-clock と見なさない規則と整合）。WASAPI の capture は `GetBuffer` `qpc_position` の carried fact として `TimestampProvenance::Carried`、他は `callback - declared latency` の `Derived`。`open_input_stream` を実装: F32 以外は `UnsupportedConfig`、`supports_input()==false` は `Unsupported`、`negotiate_input` で config 確定、`frames_capacity * channels`（`FRAMES_CAPACITY_HARD_CAP` 上限・`try_reserve_exact` で fallible alloc）を open 時に一括確保、`build_input_stream::<f32>` に data/error closure を接続。error は `map_error` で `ErrorCtx::ErrorCallback`/`Control` を区別して正規化。`epoch` は `stream.now()` で open 直後に捕捉。
- `crates/ruviform-audio-device/src/host.rs`: 列挙（`input_devices`/`default_input_device`/`output_devices`/`default_output_device`）ごとに `clock_profile(host名)` を解決し全 `CpalDevice` へ伝播 — 1 host の全 device/双方向 stream が同一 domain を報告する。
- `crates/ruviform-audio-device/tests/mock_input_backend.rs`: 旧 `cpal_adapter_reports_input_open_as_unsupported` を `cpal_adapter_input_open_reports_typed_surface` へ更新 — placeholder `Unsupported` の断言を廃し、実 open が成功すれば全 port query（`format`/`clock_domain`/`now`/`buffer_size`/`input_latency()==Ok(None)`）が typed に応答すること、失敗は typed `DeviceError` であることを検証（device 非存在 host では従来どおり silent skip）。
- `crates/ruviform-audio-device/tests/real_input_e2e.rs`（新規）+ `Cargo.toml` feature `real-device` + `mise.toml` tasks `test-audio-real-input`/`lint-audio-real-input`: Issue #114 `l1-real-verify` パターン準拠の opt-in 実機 lane。`#![cfg(feature = "real-device")]` + `#[ignore]` で既定 `cargo test`/`lint` から分離。`real_input_stream_captures_planar_audio`（open→start→planar callback→stop の end-to-end、callback>0/frame>0/channel・domain一致/timestamp 単調/non-finite=0 を assert）と `real_duplex_streams_agree_on_clock_domain`（input/output の `clock_domain` 一致 + `same_clock_domain` 判定を報告）を収録。**実行は NotRun**（実機 + capture permission が必要な手動 lane）。
- `docs/development-environment.md`: 新 lane 2 task をタスク表・注記に追記。

### Decisions During Implementation

- `input_latency()`/`output_latency()` は cpal が portable な latency query を公開しないため `Ok(None)` を返す — `Ok(0)` の捏造も backend 別推定値の提示もしない。「unsupported は型で表現」の port 契約どおり。
- `capture` timestamp の `StreamInstant::ZERO` は backend の "未報告" sentinel として `None` に写像 — 実 capture が stream clock 原点以前に起きることはなく、clock-origin 値を露出すると consumer が誤配置し得るため。overflow も同様に `None`（callback 値での代替は buffering latency 分だけ配置をずらすため禁止 — port 契約）。
- `frame_position` は cpal が frame position を公開しないため常に `None` — callback 回数からの合成 counter は真の discontinuity を隠すため採用しない（#187 決定の踏襲）。
- clock domain は「検証可能な共有 monotonic clock のみ `MONOTONIC`、それ以外は `UNKNOWN`」の保守的分類。ALSA は per-substream device clock で双方向の同一性すら証明不能なため `UNKNOWN` — duplex は `same_clock_domain` の fallback 側に落ちるのが正しい挙動。e2e lane も `same_clock_domain` の真偽を assert せず報告のみに留めた（host 依存のため）。
- negotiation は output と同一規則を `negotiate_impl` に集約（DRY: 方向別の重複実装を避け、`RangeFields` trait で supported-range 型差を吸収）。
- `open_input_stream` の失敗分類: `!supports_input()` → `Unsupported`（backend が input を駆動不能）、config 不適合 → `UnsupportedConfig`、非 F32 → `UnsupportedConfig`（adapter は F32 のみ open、黙って変換しない）。
- e2e lane の失敗ポリシー: device 非存在は環境要因として loud `SKIP`（print + pass）、device が存在する以上の全契約違反（open 失敗・callback 0・timestamp 逆行・non-finite sample 等）は hard fail — lane は問題を見つけるためのものであり寛容さない。

## Related Experiments

### Adopted

<!-- なし（既知解の機械的実装 + cpal 0.18.2 source 監査に基づく決定のみ） -->

### Rejected

<!-- なし -->

## Validation

- `mise run fmt-check`: pass。
- `mise run check`（`cargo check --workspace --all-targets`）: pass。
- `mise run lint`（`cargo clippy --workspace --all-targets -- -D warnings`）: pass。
- `mise run test`（`cargo test --workspace`）: pass — `ruviform-audio-device` は lib 59 + integration 12 + doctest 6 全件 ok。新規 input negotiation/planar unpack/chunk/timestamp map/RT no-alloc/clock_profile テストを含む。旧 placeholder 前提の `mock_input_backend` テスト 1 件は新契約へ更新済み。
- `mise run test-rt`（`cargo test --workspace rt_`）: pass・exit 0 — `rt_input_callback_path_allocates_nothing`（`InputCallbackState::handle` 全経路の無割当機械検証）を含む。
- `cargo clippy -p ruviform-audio-device --all-targets --features real-device -- -D warnings`: pass（lane ターゲットの lint。`mise run lint-audio-real-input` と同一コマンド）。
- MSRV: `CARGO_TARGET_DIR=target/msrv-audio-device mise exec rust@1.89.0 -- cargo test -p ruviform-audio-device` — pass（lib 59 + mock_backend 4 + mock_input_backend 12 + doctest 6 全件 ok）。`--features real-device --all-targets` の `cargo check` も同 toolchain で pass（lane ターゲットの MSRV 適合を確認）。audio-device 専用の恒常 MSRV task は未整備のため ad-hoc 実行 — 恒常化が必要なら別 Issue で `test-audio-device-msrv` を追加する。
- `git merge origin/main`: worktree は `origin/main`（`9de1b2f4`）取り込み済み。

### 実機検証 NotRun matrix

Stage W closeout slice（`docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md`、
Audient iD4 / WASAPI 実機）で 2026-11-11 に実測済み — 最新値はそちらを正とする。

| 検証行 | 状態 | 根拠 |
| ------ | ---- | ---- |
| `mise run test-audio-real-input`（capture e2e + duplex domain） | **Pass（Stage W で実走）** | Windows+WASAPI・Audient iD4 で 4/4 Pass。callbacks/capture `Some`/frame_position contiguity/order violations=0 を実測。Stage M・L は NotRun のまま |
| `mise run lint-audio-real-input` | Pass（本 slice 及び Stage W で継続 Pass） | lane ターゲットの lint は headless で検証可能 |
| 実測 input latency / timestamp 精度 | **Pass（実測済み）** | WASAPI native `input_latency()=Ok(Some(StreamLatency{frames:0,Unknown,Carried}))`・実効 latency（callback−capture）min 10.04 ms・全 callback `capture ≤ callback`。cpal 経路は `Ok(None)` の正直応答を duplex 行で記録 |
| xrun / device removal / permission denied / format mismatch の実機発生 | 部分 Pass | format mismatch は 7 行 `Err(UnsupportedConfig)` を実測。device loss は既定 endpoint 切替 → `StreamInvalidated`（+`ClockDiscontinuity`）を実測（物理 USB 抜去経路は NotRun・対話式手順を文書化）。xrun は観測なし（0）。permission denied は NotRun |
| same-clock duplex の実機相関（frame 単位の時刻整合） | 部分 Pass | `clock_domain` 一致（`ClockDomainId(2)`）と `same_clock_domain` verdict=true、UNKNOWN×2 の rejection=false を実測。frame 相関 math は consumer 側（Synvelis #115/#117）と Issue #315 の管轄のまま |

## Risks

- `clock_profile` の host 分類は cpal 0.18.2 source 監査に基づく静的判断 — backend 更新や host 追加で前提が変わり得る。`UNKNOWN` 側に倒す保守設計だが、誤って `MONOTONIC` を主張する host が将来増えた場合は same-clock 誤判定のリスクがある（cpal 更新時の再監査が必要）。
- `StreamInstant::ZERO` を "未報告" sentinel とする扱いは backend 実装依存 — cpal が `capture` に真の 0 を返す backend があれば `None` に退化する（値を偽装するより安全側）。
- scratch は `effective_max_frames`（交渉済み supported range の最大）基準で open 時確保 — backend が交渉値を超える buffer を返す場合は chunk 分割で吸収するが、極端に大きい delivery では callback 回数が増える（有界・無割当は維持）。
- 実機でのみ発現する問題（driver 依存の timestamp 異常・permission 経路・device removal のタイミング）は headless では検出不能 — e2e lane 実行が前提。

## Follow-ups

- `mise run test-audio-real-input` を実機環境で実行し、結果を本 record の NotRun matrix へ反映する（Issue #251 の実機検証行）。
- Issue #315（generic multi-clock/drift capability）へ、`UNKNOWN` domain host での duplex fallback 運用と drift 補償の接続点を引き継ぐ。
- Synvelis #115（real input stream・same-clock capture・raw fidelity）・#117（monitoring/punch/latency OS 実測）が本 adapter の consumer 側受入。
- cpal 更新時に `clock_profile` の host 監査表を再検証する。
