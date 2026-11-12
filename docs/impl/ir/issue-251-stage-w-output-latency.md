---
id: "IMPL-034W4Hv9p1zQfi7uOoypfu"
type: "impl"
status: "completed"
title: "Issue #251 Stage W residual — error parity + 実効 output latency 実機観測"
created: "2026-11-12"
updated: "2026-11-12"
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
        path: "docs/impl/ir/issue-251-stage-w-output-latency.md"
  implements:
    - "docs/tasks/issue-251-task1-stage-w-real-device-closeout.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md"
    - "docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md"
    - "docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md"
  refines: []
  refined-by: []
  derives-from: []
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

# Issue #251 Stage W residual — error parity + 実効 output latency 実機観測

## Summary

Issue #251 Stage W（Windows/WASAPI）の残件 slice。2 件を実施した。

1. **`map_win_error` の `E_ACCESSDENIED` parity（headless）**: ネイティブ WASAPI
   input 経路の error mapper が `E_ACCESSDENIED`（`0x80070005`）を wildcard
   `Backend("backend")` に落としていたのを、cpal 経路（`error.rs` の
   `ErrorKind::PermissionDenied → DeviceError::Backend("permission_denied")`）と
   同一分類 `Backend("permission_denied")` へ写像。他の HRESULT 写像は不変。
2. **実効 output latency の実機観測**: `real_input_e2e.rs` に
   `real_output_stream_observes_latency` を新設。duplex 検証は output stream を
   unstarted drop していたため未観測だった `OutputTimestamp.playback` と実効
   output latency（`playback - callback`）を、Audient iD4・WASAPI で ~1 s の
   無音再生を通じて実測・記録した。値は assert せず記録（input 側と同一方針）。

結果は **「Stage W の残件 2 件を実装・実機観測 / Stage M・L・物理抜去・
permission denied 実機再現・xrun・consumer 側相関は NotRun のまま — Issue 全体の
Cross OS 完了ではない」**。

## Context

Stage W closeout（`issue-251-stage-w-real-device-closeout-slice.md`、PR #568）で
Audient iD4 の input direction は 4/4 実走 Pass となったが、残件が 2 件あった。

- `wasapi_input.rs` の `map_win_error`（L223 付近）は `E_ACCESSDENIED` の写像を
  持たず wildcard に落ちる。cpal 経路 `error.rs` は
  `PermissionDenied → Backend("permission_denied")` を持つため、同一の OS
  プライバシー拒否が経路で分類不一致になる parity 欠陥。
- `real_duplex_streams_agree_on_clock_domain` は output stream を open のみで
  start せず drop するため、`OutputTimestamp.playback`・実効 output latency・
  `output_latency()` の実値が一度も観測されていなかった（closeout matrix で
  `output_latency()` 実値は `Ok(None)` 観測として NotRun 記録）。

本 slice は port 契約（`ruviform_audio_core::device`）・公開 API を変更せず、
consumer（synvelis#115）が消費中の契約を維持したまま残件を閉じる。

## Implementation

### Changes

- `crates/ruviform-audio-device/src/wasapi_input.rs`:
  - `map_win_error` に `Foundation::E_ACCESSDENIED` →
    `DeviceError::Backend("permission_denied")` を追加（既知 Audclnt HRESULT 群の
    後・wildcard の前）。他の HRESULT 写像は一切変更していない。
  - test `map_win_error_maps_audclnt_codes_per_context` に `E_ACCESSDENIED` 行を
    追加し、`ErrorCtx::Control` / `ErrorCtx::ErrorCallback` 両 context で
    `Backend("permission_denied")` を検証。
- `crates/ruviform-audio-device/tests/real_input_e2e.rs`:
  - `real_output_stream_observes_latency`（新設、`#[ignore]` + `real-device`
    feature ゲート）: default output device を negotiated F32 format
    （`BufferSize::Default`）で open → `output_latency()` pre-start 記録 →
    ~1 s 無音再生 → `stop()` → `output_latency()` post-stop 記録 → summary。
    callback は buffer 全体を `fill(0.0)` で無音化し、relaxed atomic のみで
    callback/frame 数・`playback` 有無・`playback - callback` の
    min/max/mean（±1000 s clamp 付き sum）・non-positive delta 計数・error/xrun
    計数を観測（allocation-free・RT thread 安全）。
  - assert は配送契約のみ: callbacks > 0・total frames > 0・`playback` が
    1 度以上 `Some` であること（観測対象 channel が実際に populate された証拠）。
    latency 絶対値・non-positive delta は assert しない。
  - loud SKIP 維持: default output device 不在・F32-openable format 不在は
    println SKIP で return（環境不在を test 失敗にしない）。

### Decisions During Implementation

- **non-positive delta を assert しない判断（実機発見 → 方針修正）**: 初回実装は
  input 側と同じ「`playback ≤ callback` は ordering violation で hard-fail」
  方針を採ったが、実機で 102 sample 中 3 件の `delta == 0` を観測して失敗した。
  cpal 0.18.2 の WASAPI `output_timestamp` は
  `playback = callback + (buffered + stream_latency)` で、`buffered` は
  written−consumed の飽和差分（stream 開始直後・underrun で 0 に張り付く）かつ
  `monotonic_output_callback` が playback を非減少に clamp する。本 driver は
  `GetStreamLatency` 相当が ~0 を返すため、buffered 飽和時に
  `playback == callback`（delta=0）へ退化する — これは backend estimate の
  正直な floor であり「device clock が garbage を報告した」ではない。
  `playback < callback` は Duration 加算と monotonic clamp の構造上この経路では
  起こり得ない。したがって `delta ≤ 0` は違反ではなく診断値として計数・記録に
  留め（`OUT_NONPOSITIVE_PLAYBACK`）、assert は配送契約と playback channel の
  存在に限定した。input 側の `capture > callback` assert は「capture instant が
  delivery instant より後」という物理的にあり得ない順序であり性質が異なる —
  同一 policy を機械適用しない判断を記録する。
- **`output_latency()` は `None` が正直な応答**: cpal portable 経路は
  `OutputStream::output_latency()` を populate しないため、本機でも
  pre-start/post-stop とも `Ok(None)`。値の捏造ではなく port の `Option`
  契約どおりの応答として記録（closeout の duplex 観測と一致）。
- **transient `UnsupportedConfig` の記録**: 初回 lane 実行では本 test の
  `open_output_stream` が 1 度だけ `DeviceError::UnsupportedConfig` で失敗
  した（先行 test の native input stream / duplex output drop 直後の driver・
  engine 状態に起因する一過性と推定）。以後の full lane 2 回・単独実行 1 回は
  すべて open 成功し再現せず。再現しない環境依存事象として本記録に残し、
  adapter 側の変更は行わなかった（範囲外・根拠不足）。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

- 「`playback ≤ callback` を ordering violation として hard-fail する」初回案:
  実機で delta=0 が観測され、cpal の playback estimate が buffered 飽和 +
  zero stream latency で `callback` instant へ正当に退化し得ることが判明。
  「値は assert せず記録」の task 要件および backend の実挙動と矛盾するため棄却。

## Validation

環境: Windows 11 x64・Audient iD4（`Analogue 1/2 (2- Audient iD4)`、driver
`audientusbaudio_sc`）・WASAPI host・44.1 kHz F32 Stereo・`BufferSize::Fixed(441)`。
toolchain: mise 管理 `rust 1.99.0`。canonical は mise task 経由。

### Canonical lane

| コマンド | exit | 結果 |
| -------- | ---- | ---- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check` | 0 | Pass |
| `mise run lint` | 0 | Pass |
| `mise run test` | 0 | Pass（初回に無関係な plugin-host 系 flaky 失敗、再実行で 201 test target 全 pass — 本件範囲外） |
| `mise run test-rt` | 0 | Pass |
| `mise run lint-audio-real-input` | 0 | Pass（`-D warnings`、`real-device` feature 込み） |
| `mise run test-audio-real-input` | 0 | **5/5 Pass**（Audient iD4 実機、`--test-threads=1`） |

### 実機証拠（`--nocapture` 補助証拠、canonical lane と同一 binary）

**output latency**（`real_output_stream_observes_latency`、full lane 内実行）:
host WASAPI・device `Analogue 1/2 (2- Audient iD4)`・44.1 kHz Stereo F32・
`stream.buffer_size()` pre-start `Fixed(441)`・`stream.now()` pre-start Ok・
**`output_latency()` pre-start `None` / post-stop `None`**。
callbacks=102・total frames=44629・frames/callback min=88 max=441・
playback timestamp `some=102 none=0`・**実効 output latency
（`playback - callback`）min=0 ns・max=31 972 789 ns・mean=30 905 917 ns
（102 samples、non-positive=3）**・error events=0・xrun=0。

単独再実行（`--nocapture`）でも同等値を確認: callbacks=96・frames=41983・
min=0 ns・max=31 950 113 ns・mean=30 800 500 ns・non-positive=3・
playback `some=96 none=0`・errors=0。

観測の解釈: mean ~30.9 ms は WASAPI shared-mode の buffer 深さ（~3 period ≈
30 ms @ 44.1 kHz/441 frame period）に整合し、`playback` が callback より
約 3 period 先の DAC instant を指すことを裏付ける。non-positive=3 は
stream 開始直後の buffered 飽和による delta=0 floor（上記 Decision 参照）。

**headless parity**: `map_win_error` の `E_ACCESSDENIED` 行は
`cargo test -p ruviform-audio-device`（Windows feature build）内の
`map_win_error_maps_audclnt_codes_per_context` で両 context の
`Backend("permission_denied")` 写像を検証済み（`mise run test` に含む）。

### 実機検証 NotRun / 観測なし matrix

| 検証行 | 状態 | 根拠 |
| ------ | ---- | ---- |
| `E_ACCESSDENIED → permission_denied` 写像 | **Pass（headless）** | `map_win_error` 単体 test で両 context 検証。実機での permission denied 再現は下記 NotRun |
| `real_output_stream_observes_latency` | **Pass（実機）** | 上記実機証拠（full lane 5/5 に含む） |
| `playback - callback` 実効 output latency | **観測済み** | min=0 / max≈32.0 ms / mean≈30.9 ms（102 samples）。環境依存値のため assert なし・記録のみ |
| `output_latency()` 実値 | **`Ok(None)` 観測** | cpal portable 経路の非 populate に対する正直応答（closeout の duplex 観測と一致）。実効 latency は `playback - callback` で代替観測済み |
| Stage M（macOS/CoreAudio）・Stage L（Linux/ALSA 他） | **NotRun** | 本 slice 対象外 — 別環境・別 closeout Issue が必要。Issue 全体の Cross OS 完了を宣言しない |
| 物理 USB 抜去による `AUDCLNT_E_DEVICE_INVALIDATED` 経路 | **NotRun（前 slice の代替実績を継承）** | 本 slice では未実施。closeout で既定 endpoint 切替経路の `StreamInvalidated` を実観測済み・物理経路は対話式 lane が案内 |
| OS permission denied 実機再現 | **NotRun** | privacy settings の deny 操作は環境に存在せず。headless 写像 parity のみ検証 |
| xrun / underrun（output 側） | **観測なし** | 1 s 再生で `xrun=0`/`error events=0`。誘発負荷は lane 責務外 |
| consumer 側 frame 相関（Synvelis #115/#117） | **NotRun（外部 gate）** | 同一 clock domain 上の frame 相関は consumer・Issue #315 の管轄 |

## Risks

- `playback` は backend-reported estimate であり物理 DAC instant の保証値では
  ない。cpal は underrun 時に buffered を 0 に飽和させ playback を後退させ得る
  （`monotonic_output_callback` で非減少に clamp）— 本観測の ~30.9 ms mean は
  「buffered + stream_latency の推定値」であり、真の end-to-end latency の
  上限/下限として consumer がどう解釈するかは synvelis#117 側の実測評価に委ねる。
- 初回 lane での一過性 `UnsupportedConfig`（open 時）は再現しなかったが、
  driver/engine 状態依存の可能性が残る。頻発する場合は open retry や原因調査が
  別 Issue として必要。
- `map_win_error` の `E_ACCESSDENIED` 写像は headless 検証のみ — `E_ACCESSDENIED`
  が実際に返る OS 状態（microphone privacy deny 等）での end-to-end 挙動は
  NotRun のまま。

## Follow-ups

- Stage M/L closeout は別環境の Issue として継続（Issue #251 全体の完了条件）。
- consumer 側受入: synvelis#115（same-clock capture raw fidelity）・#117
  （latency OS 実測）が `playback - callback` 実測値 ~30.9 ms と
  `output_latency()=None` の事実を参照する。
- 物理 USB 抜去・permission denied 実機再現は operator 付き環境の残課題
  （既存の対話式 lane 手順を継続利用）。
- `docs/impl/ir/README.md` index への本 record 追記は Coordinator の
  docs index 更新に委譲（本 slice の編集範囲外）。
