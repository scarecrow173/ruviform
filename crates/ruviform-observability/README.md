# ruviform-observability

Ruviform の共有観測プリミティブ — `foundation-observability-minimal-api`
スケッチ（Issue #101）の最初の実消費者面として、Issue #121 の L1
diagnostics が使う **counter pair** を提供する最小クレート。

## 責務

- `counter() -> (Counter, CounterReader)` — 共有 `Arc<AtomicU64>` セルを
  持つ計数ペアを生成する。
- `Counter`（書き手側）: `increment`（`Relaxed` `fetch_add`）・`read`・
  `reader()`。`Clone` は同一セルを共有する writer ハンドルを増やす。
- `CounterReader`（観測側）: `read`（`Relaxed` load のみ）。

## 層・許可依存

- 層: Primitives（最下位・相互依存なし）
- 許可依存: なし（std のみ）。依存なしの最小構成を維持する。
- 消費方向: `ruviform-plugin-host → ruviform-observability`（Issue #121）。
  `ruviform-rt` からの逆依存は追加しない（foundation DAG/ADR 問題は
  別 Issue — `docs/designs/foundation-observability-minimal-api.md`）。

## RT 契約

- `increment`/`read`/`reader`/`CounterReader::read`: wait-free・no-alloc・
  有界 — `Relaxed` atomic / `Arc` refcount のみ。
- `counter()`（生成）・`Clone`・`Drop`: 非 RT のみ（`Arc` セルの alloc/free）。
- 複数 counter の逐次 `Relaxed` 読みは同一時点の整合 snapshot を
  **保証しない** — 公開契約として `lib.rs` に明記。
- `u64` は modulo 2^64 wrap（現実的構成では到達不能 — saturating 措置なし）。

## 実装 Issue

- Issue #101: 候補・trigger（`foundation-observability-minimal-api`）
- Issue #104: `observability_poc` の `Counter`/`CounterReader` 参照形
- Issue #121: 初消費者 L1 diagnostics（audio counters・`Counter`/`CounterReader` を
  `InstKind::Audio` arm が所有）
- Issue #276: `logging` feature — 汎用 Logging Adapter（OBS-01）

## `logging` feature（Issue #276 / OBS-01）

標準 `tracing` / `tracing-subscriber` / `tracing-appender` 基盤の
**Host 所有**ロギングアダプタ。Ruviform 独自の Logger/Span API は
作らず、nonRT コードは `tracing` の Event/Span を直接発行する。

```toml
ruviform-observability = { path = "...", features = ["logging"] }
```

### 依存追加の理由（feature 最小・既定 off）

`logging` を有効にした時だけ `tracing` / `tracing-subscriber`（env-filter・
fmt・json）/ `tracing-appender` を optional 依存として引く。
既定（`--no-default-features`）では core counter pair のみで外部依存ゼロを
維持する。独自 logger を実装しないのは受入要件であり、標準エコシステムの
filter/format/rolling 実装を再利用するため。

### 所有権契約

- **Subscriber は Host が所有する。** `build_logging` は `Dispatch` を、
  `build_logging_layers` は `registry().with(..)` 合成可能な `Layer` 群を
  返すだけで、ライブラリは `set_global_default` / `init` を一切呼ばない。
  global 化は Host がプロセスごと 1 回だけ行う判断であり、
  `set_global_default` は既設定時に error を返す。GUI/headless/マルチ
  ウィンドウの scoped 運用には `tracing::dispatcher::with_default` を使う。
- **stdout には出さない。** `SinkDestination` は `Stderr` と `RollingFile`
  のみ — AudioLens `inspect` のような stdout の構造化出力を構造的に保護する。
- **製品固有の設定読み取りはしない。** filter 文字列は EnvFilter 構文だが
  `SinkConfig` で明示供給する。環境変数・CLI・GUI 設定から
  `LoggingConfig` への写像は Host の責務（AudioLens `AL_LOG`/flag/GUI
  優先順位も Host 側で解決する）。

### Sink 構成

- `SinkConfig { destination, format, filter, failure_mode }` — sink ごとに
  独立。filter は `Layer::with_filter` で sink 個別に適用され、同一
  event を各 sink が独自にルーティングする。
- `LogFormat::{Text, JsonLines}` — Text は ANSI off の人間可読 1 行、
  JsonLines は event ごと 1 JSON object。
- `SinkDestination::RollingFile { directory, filename_prefix, rotation,
  max_files, queue_capacity }` — ファイル名は
  `{prefix}.{rotation-date}`（daily なら `app.2026-11-04`）。retention は
  `tracing-appender` の `max_log_files` に委譲し、構築時・rotation 時に
  prefix 一致ファイルを古い順に削除する（AudioLens の 7 daily 世代は
  `Daily` + `max_files=7`）。
- `filter` が不正なら `LoggingBuildError::InvalidFilter` で build 失敗。
- `RotationPeriod::{Daily, Hourly, Minutely, Never}`。

### 失敗モデル

- `SinkFailureMode::Required` の sink 初期化失敗 →
  `LoggingBuildError::RequiredSinkFailed`（build 全体失敗、構築済み sink は
  先に shutdown）。
- `SinkFailureMode::BestEffort` の失敗 → `LoggingBuildResult::degraded` に
  明示記録し、残 sink は動作を継続。
- 全 sink 失敗（全て BestEffort）→ `LoggingBuildError::AllSinksFailed`。
  全滅構成を成功扱いしない。空 `sinks` も `NoSinks` で明示失敗。
- 実行時失敗は error ではなく `LoggingHealth::snapshot()` の per-sink
  カウンタで区別して観測する: `dropped_queue_full`（有界 queue 満杯
  drop）・`partial_writes`（`WriteZero`）・`write_failures`（その他の
  write/rotation 失敗）・`flush_failures`・`channel_disconnected`
  （worker 死亡/shutdown 後の emit）・`shutdown_abandoned`。
  Logger 自身の失敗を同じ Logger で再帰出力しない — `snapshot()` は
  atomic load のみで event を一切発行しない（非再帰計測）。
- 上游注意: `tracing-appender` は rotation 時のファイル作成失敗を自身で
  stderr に報告して旧ファイルへの書込を継続する。その経路は本 adapter の
  カウンタに到達しない。`Write` 経由で表面化した write 失敗は
  `write_failures` に計測される。

### Writer とライフサイクル

各 sink は専用 background worker スレッドが drain する有界非ブロック
writer（`std::sync::mpsc::sync_channel` + `queue_capacity`）。発行側は
`try_send` のみで決してブロックせず、満杯時は drop を計数する。
`tracing_appender::non_blocking` ではなく自前実装なのは、queue-full /
partial write / flush 失敗 / disconnect を区別する観測性が
`non_blocking` に存在しないため（ADR
`docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md`）。

`WriterGuard` の drop が shutdown を開始する: worker は shutdown marker
以前に queue された event を全て drain・flush してから終了し、guard は
`SHUTDOWN_TIMEOUT`（5s）で join を待つ。timeout 時は worker を殺さず
abandon し `shutdown_abandoned` に記録する。marker 以後に queue された
event・abandon 後・プロセス crash 時の未 flush 分は失われる（契約として
rustdoc に明記）。`Dispatch`・guard は使用期間中保持すること — guard を
先に drop するとそれ以降の event は `channel_disconnected` として捨てられる。

### RT 契約

**RT コールバックから通常の `tracing` を発行しない。** Subscriber の
layer は発行スレッド上で同期実行され、本 adapter の writer 経路は event
ごとに `Vec<u8>` へ format 用 alloc を行うため RT-safe ではない。
RT 側の観測は RT-safe な専用機構（counter pair 等）を使う。

### センシティブデータ

adapter は event の field をそのまま sink に流すだけで、内容の検査・
redact・rate-limit は行わない。raw PCM・prompt・path・credential を
無制限に出力しない責務は発行側にある（本モジュール自身はそのような
event を一切生成しない）。

### Host sample

```sh
cargo run -p ruviform-observability --features logging --example logging_host
```

`examples/logging_host.rs` — Host 側 config 写像 → `build_logging` →
scoped dispatch → health snapshot → guard shutdown の一通りを示す。
