---
id: "SPEC-034RoXidyTvNC4Cd998l35"
type: "spec"
status: "approved"
title: "Issue #276 汎用 Logging Adapter(observability::logging)"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-276-generic-logging-adapter.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/foundation-observability-minimal-api.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/foundation-observability-minimal-api.md"
  derived-by:
    - "docs/designs/issue-276-generic-logging-adapter.md"
    - "docs/plans/issue-276-generic-logging-adapter.md"
    - "docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md"
    - "docs/test-specs/issue-276-generic-logging-adapter.md"
    - "docs/specs/issue-336-sink-degraded-clone.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-276-generic-logging-adapter.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #276 汎用 Logging Adapter(observability::logging)

## What Should Be Built

`ruviform-observability` 単一クレートに feature `logging` として実装する、Host 所有の汎用 Logging Adapter。
基盤は標準 `tracing` / `tracing-subscriber` / `tracing-appender` とし、Ruviform 独自の Logger / Span Emit API は作らない。
nonRT の Ruviform・両 Consumer は `tracing` の Event / Span を直接発行する。

公開構成は Issue #276 本文の API sketch を正本とする:

- `LoggingConfig { sinks: Vec<SinkConfig> }`
- `SinkConfig { destination, format, filter, failure_mode }`
- `SinkDestination::{Stderr, RollingFile { directory, filename_prefix, rotation, max_files, queue_capacity }}`
- `LogFormat::{Text, JsonLines}` / `SinkFailureMode::{BestEffort, Required}` / `RotationPeriod::{Daily, Hourly, Minutely, Never}`
- `build_logging(config) -> Result<LoggingBuildResult, LoggingBuildError>` — `Dispatch` + WriterGuard + 初期 Degraded + Health を返す
- `build_logging_layers(config) -> Result<LoggingLayersResult, LoggingBuildError>` — Host の既存 Subscriber へ合成可能な `Layer` を返す

## Why Now

AudioLens #138・Synvelis #81 の Logging 初期導入の前提となる OBS チェーンの独立先行 Issue。
Debug / Profiling / Collector の完成を待たずに、両 Consumer が最低限必要とする「stderr / rolling file・text / JSONL・
sink 別 filter・日次 rotation・7 世代 retention・有界 queue・失敗の明示分類」を共通化し、Consumer 側の
独自ロガー分岐を防ぐ。

## Users and Value

- AudioLens(AL-OBS-01): `inspect` の stdout JSON 保護(ログの stdout 無混入)・日次 7 世代・file 失敗時 degrade を
  既存挙動の回帰基準として維持できる。
- Synvelis(SYN-OBS-01): Multiwindow / GUI / Headless で Subscriber 所有を Host に残したまま、scoped dispatch と
  layer 合成の両経路を使える。
- 将来の Ruviform 非 RT クレート: `tracing` Event を発行するだけで Host が構築した sink へ流れる。

## Scope

### In Scope

- feature `logging` ゲートの `observability::logging` モジュール(config・error・health・writer・build)。
- stderr sink と rolling file sink(daily/hourly/minutely/never rotation + `max_log_files` retention)。
- sink 別 `EnvFilter` 文字列 filter(不正は型付き error)と sink 別 format(text / JSON Lines)。
- `Required` 失敗は型付き error、`BestEffort` 失敗は `degraded` に記録して残 sink を継続、全 sink 失敗は error。
- 有界 queue + 単一 background worker の非 RT writer。queue-full drop・WriteZero partial write・
  write/rotation 失敗・flush 失敗を Health で区別。Worker join + shutdown timeout 契約。
- `LoggingHealth` による非再帰計測(Logger 自身の異常を同じ Logger に再帰出力しない)。
- rustdoc・README・Host example・feature DAG / MSRV 検証。

### Out of Scope

- `set_global_default` / `init` の呼出(ライブラリは決して呼ばない。Host の責務)。
- 製品固有の環境変数(AL_LOG 等)・CLI flag の解釈。Host が `LoggingConfig` へ写像する。
- Collector / tracing_export / RT Bridge / integration への依存・実装(別 OBS Issue)。
- stdout sink(設計上 stdout には一切書かない)。
- raw PCM / Prompt / Path / Credential 等の内容検査・マスキング(発行側の責務;契約として明記)。
- Audit / Provenance / Authorization 記録の置換。
- OS 差検証(本 dispatch は Windows のみ;Linux/macOS は未検証と明記)。

## Requirements

- 公開 enum はすべて `#[non_exhaustive]`(ADR 準拠)。既存 core `counter` API の型・挙動を変えない。
- `--no-default-features` で core に tracing 系依存を持ち込まない。`logging` 単独で Builder が利用可能。
- MSRV 1.89 でコンパイル可能な依存版のみ採用(tracing 0.1 / tracing-subscriber 0.3 / tracing-appender 0.2 系)。
- 非 RT writer はメモリ上限を持ち、queue-full で黙ってブロック・無制限 alloc しない。
- **RT callback から通常の `tracing` Event を発行しない**ことを契約として rustdoc・README に明記する
  (Subscriber の Layer は発行スレッドで同期実行され、本 adapter の writer は alloc を伴うため RT-safe ではない)。

## Acceptance Criteria

- AC1: stderr / file それぞれで Text・JSONL が出力され、sink 別 filter による dual routing と
  target / metadata が期待どおりであることを契約試験で確認する。
- AC2: RollingFile の日次 rotation・`max_files` による既存ファイルの削除保持・prefix 命名を試験する。
  OS 差は Windows のみ検証;他 OS は未検証と明記する。
- AC3: Sink 構成上 stdout への出力経路が存在しない(`inspect` JSON 保護の構造的保証)。
- AC4: 不正 filter・初期化不能な dir・Required / BestEffort / 全 sink 失敗が型付き error または
  明示 `degraded` として区別される。
- AC5: queue-full drop・partial write(WriteZero)・write/rotation 失敗・flush 失敗が
  Health Snapshot で区別され、Logger 異常が同じ Logger に再帰出力されない。
- AC6: Background worker が WriterGuard の shutdown で join され、timeout 超過時は放棄を記録する。
- AC7: `--no-default-features` で core のみ、`--features logging` で Builder がコンパイルできる。
- AC8: rustdoc・README・Host example が整備され、`mise run` 系検証がすべて成功する。
