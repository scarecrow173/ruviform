---
id: "DESIGN-cJ0I0j6o66AAfUm2eE5q8y"
type: "design"
status: "approved"
title: "Issue #251 Stage W closeout 設計 — 実機 lane 拡張と証拠 matrix"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/251"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-251-stage-w-real-device-closeout.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/select-audio-device-backend.md"
    - "docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
---

# Issue #251 Stage W closeout design: 実機 lane の検証網羅拡張

## 方針

本 slice は実装ではなく **実機検証 + 証拠記録**。変更は既存 opt-in lane ファイル
`crates/ruviform-audio-device/tests/real_input_e2e.rs` 内の検証行拡張に限定する。
新規 lane / mise task / feature / workflow は追加しない（Issue #114 `l1-real-verify`
パターンの既存枠組みをそのまま使う）。port 契約（`ruviform-audio-core`）は不変であり、
adapter src には実機検証で欠落が判明した場合のみ触れる。

## 検証行の追加設計

### 1. `real_input_stream_captures_planar_audio`（既存拡張）

- `stream.input_latency()` の応答を開示して記録する。WASAPI-native path（host名 `WASAPI`）
  では `Ok(Some(StreamLatency))`（`IAudioClient::GetStreamLatency` 由来、
  `coverage = Unknown`、`provenance = Carried`）を assert する — Issue #565 で実装済みの
  実値応答契約の実機確認。非 WASAPI host では `Ok(None)` を許容（cpal に portable な
  latency query がないための正直応答）。
- data callback で `timestamp.capture == Some` のとき
  `callback - capture`（実効 input latency、ns）の min/max/sum を relaxed atomics で集計し、
  `capture > callback`（時刻順序違反 = capture 瞬間が配送瞬間より後）を
  `CAPTURE_ORDER_VIOLATIONS` で計数し post-stop で `assert_eq!(0)` する。
  latency 絶対値自体は環境依存のため assert せず実測値として記録する。
- `print_capture_summary` に実効 latency 統計を追加する。xrun/underrun は既存
  `XRUNS`/`ERROR_EVENTS`/`LAST_ERROR_KIND` の記録をそのまま証拠化する
  （発生 0 は「観測なし」として記録 — 発生させることを強制しない）。

### 2. `real_input_open_rejects_unsupported_configs`（新規・非対話）

実機の supported 空間に対し、port 側 negotiation が拒否すべき config を
`open_input_stream` に投げ、`Err(DeviceError::UnsupportedConfig)` を assert する。
いずれも backend を呼ばない port 内判定なので deterministic に検証できる。

| 行 | config | 期待 |
| --- | --- | --- |
| 非 F32 | `SampleFormat::S16` + 有効 rate/layout | `Err(UnsupportedConfig)` |
| 範囲外 rate（上） | 全 F32 range の `max_sample_rate` の最大値超 | `Err(UnsupportedConfig)` |
| 範囲外 rate（下） | 全 F32 range の `min_sample_rate` の最小値未満 | `Err(UnsupportedConfig)` |
| 範囲外 layout | いずれの range の `channels` にも一致しない layout（存在する場合） | `Err(UnsupportedConfig)` |
| `BufferSize::Fixed(0)` | 有効 format | `Err(UnsupportedConfig)` |
| `BufferSize::Fixed` 範囲外 | 全 candidate が bounded かつ要求 > max bound（`Unknown` bound 報告時はこの行を skip） | `Err(UnsupportedConfig)` |

「device が存在するのに全行 skip」はあり得ない（少なくとも非 F32・rate・Fixed(0) は常に検証可能）。

### 3. `real_duplex_streams_agree_on_clock_domain`（既存拡張）

- 実走記録: input/output `clock_domain` 一致 assert は既存どおり。
- 非同一時 rejection/fallback 観測: 同一物理装置の WASAPI stream は全て
  `MONOTONIC` を報告するため、実機上で非同一 domain pair は構成不能。
  contract 判定 `same_clock_domain(in_domain, ClockDomainId::UNKNOWN) == false` と
  `same_clock_domain(UNKNOWN, UNKNOWN) == false` を lane 内で実測表示し、
  `UNKNOWN` 側に倒す rejection/fallback 判定を観測として記録する
  （実 hardware での非同一 pair は `NotRun` と正直に記録）。
- `input.input_latency()`/`output.output_latency()` の応答値も引き続き開示する
  （output は cpal 経路のため `Ok(None)` — 「unsupported は型で表現」の実測記録）。

### 4. `real_input_device_removal_notifies_typed_error`（新規・対話式）

物理抜去は完全自動化不能なため、**env var 二重ゲートの対話式テスト**として実装する。

- `RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS` 未設定 → 手動手順を `println!` で表示して
  `SKIP`（pass、証拠 matrix では `NotRun`）。`#[ignore]` + feature gate の既存二重ゲートに
  さらに operator 明示 opt-in を重ねることで、標準 lane run が待機で stalls しない。
- 設定時 → open+start して baseline capture（callbacks>0 を確認）後、
  「デバイスを抜去せよ」旨を console へ表示し、window 秒間 error callback を観測する。
  `cargo test` の capture は `CONOUT$` を経由する helper で bypass し、
  `--nocapture` なしの通常 lane 実行でも指示が実時間で見える
  （`CONOUT$` が開けない環境では `eprintln!` fallback）。
- 観測: `DeviceError::StreamInvalidated` が error callback に到達することを assert。
  WASAPI では unplug → `AUDCLNT_E_DEVICE_INVALIDATED` → `map_win_error(ErrorCallback)`
  → `StreamInvalidated`、または default-device monitor が `NOTPRESENT`/`OnDeviceRemoved`
  を検出 → `StreamInvalidated` の 2 経路がある（どちらも同じ typed variant に正規化）。
  `ClockDiscontinuity`/`Xrun` 等の同時発生は計数して記録するが必須としない
  （backend/driver 依存の付随症状であり、契約は「typed error の到達」）。
- post-removal control-path: `stop()`/`now()`/`buffer_size()` の応答を記録する。
  `stop()` は worker 退出の競合で `Ok` にも `Err(StreamInvalidated)` にもなり得るため
  assert せず実測記録とする（`now()` は QPC 由来で device liveness 非依存の
  `Ok` 継続が見込まれる — これも観測）。
- window 内に event が来なかった場合 → `NOT-EXERCISED` と表示して pass
  （証拠 matrix では「未実施」と記録 — 観測の捏造をしない）。
- `#[ignore]` 文字列に対話要件を明記する。

## 記録先

- lane 出力（stdout）がそのまま実測値の一次証拠。PR 本文・IR に転記する。
- `docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md`（新規 IR）に
  Stage W matrix（Pass/NotRun/NOT-EXERCISED 各行の根拠）を置く。
- `docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md`
  の NotRun matrix も同じ実測結果に同期更新する（行項目の整合のみ、history としての
  本文は保持）。
- Stage M/L は本 slice では実行しない — matrix に `NotRun`（環境未提供）として残し、
  「Stage W Pass / Stage M,L NotRun — Issue 全体の Cross OS 完了ではない」と明記。

## 棄却した代替案

| 案 | 棄却理由 |
| --- | --- |
| 新しい lane/task を mise.toml に追加 | 共通契約で禁止。既存 `test-audio-real-input` lane が opt-in 入口 |
| `Disable-PnpDevice` 等による自動抜去 | 非 elevated 環境では不可。対話式＋CONOUT$ 表示の方が確実 |
| 抜去検証を常時 window wait | 標準 lane run が毎回待機で stalls する。env var 二重ゲートで解決 |
| 抜去未実施を fail にする | operator 不在 run で lane が常時 fail する。NOT-EXERCISED pass + matrix 記録が正直 |
| permission denied の強制再現（OS 設定操作） | GUI/registry 操作を伴い lane 責務を超える。手順のみ記録し NotRun |
