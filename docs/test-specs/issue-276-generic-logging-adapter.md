---
id: "TSPEC-034RoYjtRWwL5tpZsCCp6T"
type: "test-spec"
status: "approved"
title: "Issue #276 generic logging adapter test specification"
created: "2026-11-03"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/276"
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
    - "docs/designs/issue-276-generic-logging-adapter.md"
    - "docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/designs/issue-276-generic-logging-adapter.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/designs/issue-276-generic-logging-adapter.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #276 generic logging adapter test specification

## Purpose

`observability::logging` の公開契約(sink 別 filter/format・失敗分類・有界 writer・
retention・Health・stdout 非出力)を pin し、実装を置き換えても契約が保持されることを
保証する。`tests/logging.rs`(契約試験)と `src/logging/writer.rs` mod 内 unit test が本体。

## Feature

Host が `build_logging` / `build_logging_layers` で構築した Dispatch / Layer を経由して、
nonRT の `tracing` Event が sink 別の filter・format で stderr / rolling file へ届き、
失敗が型・counter で区別されること。

## Rules

- R1 sink 別 filter: 同一 target/level の event が、filter の異なる複数 sink へ独立に
  ルーティングされる(dual routing)。
- R2 format: `Text` は人間可読行、`JsonLines` は 1 event = 1 JSON object 行で、
  level/target/fields/metadata を含む。
- R3 stdout 非出力: どの sink 構成・経路でも stdout に書かない(構造的保証 +
  file sink 内容の確認)。
- R4 build 失敗分類: 不正 filter → `InvalidFilter`;`Required` sink 初期化失敗 →
  `RequiredSinkFailed`;`BestEffort` → `degraded` に記録して残 sink を継続;
  全 sink 失敗 → `AllSinksFailed`;`sinks` 空 → `NoSinks`。
- R5 retention: `max_files` 超の既存 log file が古い順に削除され ≤ max_files を維持
  (構築時と各 rotation 時;appender 内蔵に委譲)。
- R6 失敗計測の区別: queue-full drop・`WriteZero` partial write・io write 失敗
  (rotation 失敗を含む)・flush 失敗・channel disconnect が Health で別 counter。
- R7 非再帰計測: Logger 自身の異常は同じ pipeline に流れず、
  `LoggingHealth::snapshot()` は atomic load のみで event を発行しない。
- R8 shutdown: `WriterGuard` drop で worker が drain→flush→join され、
  `SHUTDOWN_TIMEOUT` 超過は `shutdown_abandoned` に記録される。

## Examples

- E1 `dual_sinks_have_independent_filters`: `info` filter の file sink と `warn` filter の
  file sink を同一 dispatch に置き、info+warn を発行 → 前者に両方・後者に warn のみ残る。
- E2 `jsonl_sink_emits_one_object_per_event`: JSONL sink へ `info!(answer = 42, "m")` →
  ファイルに `{…"level":"INFO","target":..,fields:{answer:42,message:"m"}}` の 1 行。
- E3 `rolling_file_retention_deletes_oldest`: `max_files=3` で prefix 一致の
  過去日付ファイルを 5 件 seeded → build 後に新しい 3 件のみ残る。
- E4 `required_file_sink_bad_dir_fails`: directory が file を指す Required sink →
  `Err(RequiredSinkFailed{..})`。同じ構成の BestEffort → `Ok` + `degraded` 1 件。
- E5 `all_best_effort_failures_error`: 全 sink BestEffort かつ全滅 → `Err(AllSinksFailed)`。
- E6 `invalid_filter_is_typed_error`: `filter = "!!bogus"` → `Err(InvalidFilter{..})`。
- E7 `queue_full_drops_are_counted`(writer unit): capacity=1 で受け取らない channel に
  2 send → `dropped_queue_full == 1`。
- E8 `write_zero_is_partial_write`(writer unit): `Ok(0)` を返す inner writer →
  `partial_writes` が増え `write_failures` は増えない。
- E9 `writer_failure_is_counted`(writer unit): 常に `Err` の inner writer →
  `write_failures` 増・event ごとに retry され続ける。
- E10 `disconnect_marks_sink_failed`(writer unit): worker 死亡後の `try_send` →
  `channel_disconnected` + `state = Failed`。
- E11 `guard_joins_and_flushes`: event 発行 → dispatch+guards drop → file に全 event。
- E12 `snapshot_emits_no_events`: 失敗を起こした後 `snapshot()` → 同じ sink 出力に
  異常内容が混入しない・counter が snapshot 前後で不変。

## Guarantees

- 各 sink は `SinkConfig.filter` の EnvFilter にのみ従い、他 sink の filter から影響を受けない。
- 構築済み sink の失敗は他の健全な sink の動作を止めない。
- `build_logging` は `set_global_default` を呼ばない — scoped `with_default` でのみ動くことを試験。
- stderr sink も同じ有界 writer 経路(queue-full で drop・非 RT でのみ使用)。

## Non-goals

- RT callback からの発行の動的検出(契約は文書のみ;`EventWriter` は alloc するため RT-safe でない)。
- 実際の日次 rotation の時間経過による切替(時刻を待つ試験は flaky;retention は
  seeded files で代替。rotation period→`Rotation` の写像は unit で確認)。
- readonly-dir / disk-full の実 OS 検証(Windows のみ。dir が file の初期化失敗で代替)。
- `tracing` Event 自体の emit 正確性(tracing-subscriber 側の責務)。

## Risk

- 失敗分類が曖昧になると「ログが出ない」「全 sink 死亡」が黙って成功扱いされ、
  AudioLens `inspect` の stderr-only 観測や disk-full 診断が失われる。
- retention 委譲の注意書き(保持数が max 未満になり得る)を隠すと 7 世代保持の
  期待とのずれを原因不明にする。
