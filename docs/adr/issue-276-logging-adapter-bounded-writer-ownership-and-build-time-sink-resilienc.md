---
id: "ADR-034RoY92ThOb7k8jYxuEQy"
type: "adr"
status: "accepted"
title: "Issue #276 logging adapter: bounded writer ownership and build-time sink resilience"
created: "2026-11-04"
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
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-276-generic-logging-adapter.md"
    - "docs/specs/issue-276-generic-logging-adapter.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #276 logging adapter: bounded writer ownership and build-time sink resilience

## Context and Problem Statement

`observability::logging` の非 RT writer に queue-full drop・partial write・write/rotation 失敗・
flush 失敗を Health で**区別して計測**する責務がある(Issue #276 受入)。
`tracing_appender::non_blocking` は bounded channel を提供するが、queue-full 時の drop を
内部で握りつぶし、drop 数・失敗分類・worker join の観測手段を公開していない。
また sink 初期化の成否を `Required` / `BestEffort` で分離する必要がある。

## Considered Options

* `tracing_appender::non_blocking` + `NonBlocking::with_max_level` 等をそのまま採用 — drop 計測・
  失敗分類が実現できず受入を満たせない。
* 自前の有界 `sync_channel` + 単一 worker thread writer(`BoundedWriter`)— `try_send` の
  Full/Disconnected と `write_all`/`flush` の io error kind で受入の分類がすべて観測可能。
  実装量は増えるが小さい(1 sink = 1 worker・1 event = 1 message)。
* `crossbeam` 等の外部 bounded channel — `std::sync::mpsc::sync_channel` で足り、
  新規依存を増やさない(最小依存原則)。

## Decision Outcome

Chosen option: "自前の有界 `sync_channel` + 単一 worker thread writer", because
受入が要求する失敗分類(queue-full / WriteZero / io / flush / disconnect / shutdown timeout)を
公開観測手段なしに `non_blocking` では実現できず、標準ライブラリだけで閉じた小さな実装で
満たせるため。retention は `tracing-appender` 0.2.3+ の `max_log_files` に委譲し、
独自削除処理は実装しない。

## Implementation Plan

* Affected paths: `crates/ruviform-observability/src/logging/writer.rs`(新規)、
  `build.rs` / `health.rs`(接続)、`Cargo.toml`(optional deps)。
* Constraints: `tracing`/`tracing-subscriber`/`tracing-appender` 以外の新規実行時依存を
  追加しない。worker は `catch_unwind` せず panic 時は自然死 — sender 側は `Disconnected` で
  Failed を検出する。`unsafe` 不使用。

## Verification

* [ ] `writer.rs` mod 内 unit test で queue-full drop・WriteZero・io 失敗・disconnect→Failed・
  join timeout が区別されて計測される。
* [ ] `tests/logging.rs` で shutdown flush・health snapshot の非再帰計測を確認。
