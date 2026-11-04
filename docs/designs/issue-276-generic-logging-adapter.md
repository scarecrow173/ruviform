---
id: "DESIGN-034RoXjgYbKKZ3ZSazkSGi"
type: "design"
status: "approved"
title: "Issue #276 汎用 Logging Adapter 設計"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "docs/specs/issue-276-generic-logging-adapter.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/foundation-observability-minimal-api.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-276-generic-logging-adapter.md"
  derived-by:
    - "docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md"
    - "docs/test-specs/issue-276-generic-logging-adapter.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-276-generic-logging-adapter.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #276 汎用 Logging Adapter 設計

## Context

Issue #275(Observability 統合設計正本・approved)で、`ruviform-observability` 単一クレート内の
feature `logging` として汎用 Logging Adapter を置くことが確定した。旧設計
`docs/designs/foundation-observability-minimal-api.md` で保留だった logging 区分を、
標準 `tracing` 基盤の上で具体化する最初の OBS Issue。依存クレートは
`tracing` / `tracing-subscriber`(env-filter, fmt, json)/ `tracing-appender` の optional 依存のみで、
collector・tracing_export・RT bridge・`ruviform-rt` への依存は持たない。

## Scope

### In Scope

- `src/logging/` サブモジュール(config / error / health / writer / build + mod の再エクスポート)。
- feature `logging` 配下の optional 依存と dev-dep(tempfile・serde_json・tracing)。
- `tests/logging.rs` 契約試験と `examples/logging_host.rs`。

### Out of Scope

- core `counter` API の変更。他 feature(`collector` 等)の実装。stdout sink。
- Host 側の env/CLI 解釈・複数 window 間の再登録管理(契約として文書化のみ)。

## モジュール構成と型所有権

```text
src/logging/
├── mod.rs      — モジュール契約文書(RT 禁止・stdout 非出力・Host 所有)と再エクスポート
├── config.rs   — LoggingConfig / SinkConfig / SinkDestination / LogFormat /
│                 RotationPeriod / SinkFailureMode
├── error.rs    — LoggingBuildError / SinkFailure
├── health.rs   — LoggingHealth / HealthSnapshot / SinkHealthSnapshot / SinkState /
│                 SinkKind / 内部 SinkMetrics(Arc<…> 共有)
├── writer.rs   — BoundedWriter(有界 channel + worker thread)/ EventWriter /
│                 WriterGuard / InnerWriter(Box<dyn Write+Send>)
└── build.rs    — build_logging / build_logging_layers / BuiltSinks /
                  LoggingBuildResult / LoggingLayersResult / SinkDegraded
```

### config 型

Issue 本文の sketch をそのまま所有。struct variant `RollingFile { directory, filename_prefix,
rotation, max_files: NonZeroUsize, queue_capacity: NonZeroUsize }` を採用し、
`Stderr` sink の queue 容量は公開 config には出さず `DEFAULT_STDERR_QUEUE_CAPACITY = 4096` の
内部既定とする(sketch 不変・API 面を増やさない)。全公開 enum は `#[non_exhaustive]`。
config struct は repo 慣行どおり pub field の plain struct(`#[non_exhaustive]` は付けない)。

`filter: String` は `EnvFilter` directive 構文(`"info"`、`"my_target=debug,warn"`、`"off"` 等)。
空文字列は `EnvFilter` の既定動作(ERROR 相当)に従うことを文書化する。

### writer: 自前有界 writer(tracing-appender `non_blocking` を使わない)

採用判断は ADR 参照。理由: queue-full drop・WriteZero・io 失敗・flush 失敗の
**区別された計測**が受入で要求され、`tracing_appender::non_blocking` は drop を内部で
握りつぶして観測手段を提供しないため。代わりに:

- `std::sync::mpsc::sync_channel::<Vec<u8>>(capacity)` で有界化(1 message = 1 formatted event)。
- `EventWriter`(`io::Write` を実装・fmt layer の `MakeWriter` が event ごとに生成)は
  内部 `Vec<u8>` に buffer し、Drop で `try_send`。`Full` → `dropped_queue_full`、
  `Disconnected` → sink `Failed` へ遷移 + `channel_disconnected` 計測。
- Worker thread は `recv()` で受け、`write_all` + 最終 `flush`。`WriteZero` → `partial_writes`、
  その他 io error → `write_failures`(rotation 失敗もここに集約される)、
  flush error → `flush_failures`。worker は失敗後も次 message を処理し続ける(transient 回復余地)。
- `WriterGuard` は sender を保持し、Drop で channel 切断 → worker は drain→flush→done 送信。
  Guard は `recv_timeout(SHUTDOWN_TIMEOUT = 5s)` で join を待ち、timeout 時は
  `shutdown_abandoned = true` を Health に記録して JoinHandle を放棄(スレッド強制終了は行わない;
  残り buffered message は失われる — クラッシュ時未 flush と併せて契約に明記)。

### build: 2 entry points

- `build_sinks(config) -> Result<BuiltSinks, LoggingBuildError>`(内部共通):
  各 sink について filter parse → destination 初期化(`RollingFileAppender::builder()
  .rotation(..).filename_prefix(..).max_log_files(n).build(dir)`)→ BoundedWriter spawn →
  `fmt::layer::<Registry>()` + format 分岐(Text=plain / JsonLines=`.json()`)+
  `.with_ansi(false)` + `.with_writer(make_writer)` + `.with_filter(env_filter)` →
  `Box<dyn Layer<Registry> + Send + Sync>`。
- `build_logging` = `build_sinks` + `registry().with(layers)` → `Dispatch::new(..)`。
- `build_logging_layers` = `build_sinks` の layers・guards・degraded・health をそのまま返し、
  Host が `registry().with(result.layers).with(host_layers)` で既存 Subscriber へ合成できる。

失敗分類:

- filter parse 失敗 → `LoggingBuildError::InvalidFilter { sink_index, filter, reason }`(即 error)。
- destination 初期化失敗(InitError / io): `Required` → `RequiredSinkFailed { sink_index, kind, source }`
  で build 全体を error 終了(それまでに spawn した sink は drop で正常 join)。`BestEffort` →
  `SinkDegraded { sink_index, kind, reason }` を集積して継続。
- `sinks.is_empty()` → `LoggingBuildError::NoSinks`(黙った no-op 成功を防ぐ)。
- 全 sink が `BestEffort` で全滅 → `LoggingBuildError::AllSinksFailed { failures }`(黙って成功扱いしない)。

### Health

`LoggingHealth`(Clone 可能・`Arc<[SinkMetrics]>` 共有)に `snapshot() -> HealthSnapshot`。
`SinkHealthSnapshot { sink_index, kind: SinkKind, state: SinkState{Active,Degraded,Failed},
events_written, events_dropped_queue_full, partial_writes, write_failures, flush_failures,
channel_disconnected, shutdown_abandoned, degraded_reason }`。すべて atomic(Ordering::Relaxed)
— snapshot 自体は tracing event を発行しない(非再帰計測)。Logger 自身の異常は
**同じ logging pipeline に流さず**、atomic カウンタと `degraded_reason` への記録に留める。

### Rolling file / retention

`tracing-appender` 0.2.3+ の `Builder::max_log_files(n)` は appender 構築時と各 rotation 時に
「prefix 一致・非 dir/symlink」の候補から古い順に削除し ≤ n 件を維持する(0 は削除なし)。
独自 retention 処理は不要 — **責務は appender 内蔵に委譲する**ことを契約に明記し、
保持数が時々 max 未満になり得る upstream 注意書きも rustdoc に転記する。
ファイル名形式は `{prefix}.{date}`(daily: `YYYY-MM-DD`)。rotation period は
`Rotation::{DAILY,HOURLY,MINUTELY,NEVER}` へ写像。

### 所有権・契約(契約として rustdoc/README に明記)

- ライブラリは `set_global_default` / `init` / `EnvFilter::from_env` を呼ばない。
  Host が `Dispatch` を global 登録するか scoped(`dispatch::with_default`)で使うか決める。
  global 登録は原則プロセス 1 回・multiwindow からの再登録禁止は Host 側契約。
- **RT callback から通常 `tracing` を使わない**: Layer は発行スレッド同期実行で、
  `EventWriter` は event ごとに `Vec` を alloc するため RT-safe ではない。
- adapter は raw PCM / Prompt / Path / Credential を「自動無制限出力」しない — sink は event を
  そのまま通すだけなので秘匿情報の不発行は発行側の責務。adapter 自身はイベント内容を触れない。
- stderr sink も同一の有界 writer 経路(`DEFAULT_STDERR_QUEUE_CAPACITY`)。
  パニック・abort 時に未 flush の queued message は失われる。

## Data / Control Flow

```text
Host config → build_logging / build_logging_layers
  → per-sink: EnvFilter parse → RollingFileAppender|stderr → BoundedWriter::spawn
    → fmt Layer(Text|Json) + with_filter → Box<dyn Layer>
  → registry().with(layers) → Dispatch(または layers を Host へ)
Emit: tracing::info!(..) → Dispatch → per-sink Layer(filter→format)
  → EventWriter.buf → try_send(bounded ch) → worker → write_all → sink writer
Health: LoggingHealth::snapshot() → atomic loads(非再帰)
Shutdown: WriterGuard drop → sender drop → worker drain→flush→done → join(5s timeout)
```

## 検証方針

`tests/logging.rs` の契約試験(file sink 経由で text/JSONL 内容・dual filter routing・
retention・failure 分類・scoped dispatch・health snapshot)と `writer.rs` の
mod 内 unit test(queue-full・WriteZero・io 失敗・disconnect→Failed・join timeout)。
`cargo check --no-default-features` / `--features logging` / feature DAG は T3 で実施。
