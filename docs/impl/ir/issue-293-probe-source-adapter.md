---
id: "IMPL-4bN8kQ2wX7vL3pR6tY9mD1s"
type: "impl"
status: "completed"
title: "Issue #293 OBS-11 ProbeSource adapter + 統合 edge 強化 実装記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/293"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe/source.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_probe_source.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe.rs"
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/tests/integration.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/integration/runner.rs"
      - type: "file"
        path: "crates/ruviform-observability/README.md"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-293-probe-source-adapter.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/292"
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/223"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #293 OBS-11 ProbeSource adapter + 統合 edge 強化 実装記録

## Summary

Issue #293（OBS-11）の残存 slice — `ruviform_observability::debug::
ProbeSource` ポート（#292）の具象 adapter を `ruviform-audio` に
実装し、統合 edge（reconfigure/filter-change・secret
non-exposure・GUI pull 境界）を強化し、observability MSRV lane
（Rust 1.89）を追加した。dependency 方向は `ruviform-audio →
ruviform-observability` の一方向を維持（observability は audio に
依存しない — `probe.rs` 冒頭の契約通り具象 adapter は audio 側責務）。

## Context

[implement-issue-293-probe-source-adapter.md](../../tasks/implement-issue-293-probe-source-adapter.md)
の実装スライス。上流: Issue #293 spec/design/plan/test-spec と前段 slice
（`integration` feature/`ObservationRunner`、task
`implement-issue-293-observability-integration-runner.md` の impl
record）。消費する既存契約: `debug::ProbeSource`/
`ProbeDeckSnapshot`/`ProbeReading`（#292）、audio 側 `pair()` の
`ProbeBroker`/`ProbeDeck`/`ProbeCollector`（#185）、`ProbeClockMark`/
`ProbeTimeContext`/`wire_clock_mark`/`bump_epoch`（#309）、
rebind/capture boundary policy（#308）。

## Implementation

### Changes

- `crates/ruviform-audio/src/typed/probe/source.rs` 新設 —
  `ProbeSourceAdapter`。`pair()` の非 RT 半分（`ProbeBroker` 制御面 +
  `ProbeCollector` drain 面）を `Arc<Mutex<SharedState>>` で束ね、
  `Clone` 可能な host-facing handle として `ProbeSource` を実装。
  - `decks()`: telemetry channel のみ drain して per-probe 累積
    （`recorded`/`last_status`/RT 確定 `applied_mode`）へ畳み込み、
    `chunks_out` は**消費しない** — capture PCM は
    `drain_events()`（`ProbeCollector::drain` と同じ telemetry→chunks
    順・chunk 即時 free pool 還元）経由でのみ host が受け取る。
    `dropped`/`lost_blocks` は deck-global の `ProbeCounts` を
    ミラー（probe 帰属不能な drop を偽装しない）。`generation` は
    broker が最後に staging した plan generation、未解決 probe は
    旧 generation を保持して skew を inspector に見せる。
    `clock` は drain 済み record の最新 mark（未観測は `UNKNOWN`
    fail-closed）。`confidentiality = Internal`。
  - control 面: `install`/`set_mode`/`capture`/`stop_capture`/
    `remove`/`set_capture_policy`/`on_publish`/`point`/`len`/
    `with_time_context(_mut)`/`wire_clock_mark`/`bump_epoch`/
    `counts`/`drain_events`/`lock_recoveries_total`。
    すべて非 RT（alloc/lock 許容）— RT 側 `ProbeDeck` には一切
    触れない。
- `crates/ruviform-audio/src/typed/probe.rs` — `ProbeEntry` に
  `applied_mode`/`bound_generation`/`availability`/`last_status`/
  `recorded` を追加。`install` で初期化、`on_publish` は
  Begin/Stage/Commit 全 enqueue 成功後にのみ解決状態を確定
  （途中失敗で旧世代を偽らない）。`mod source` +
  `pub use ProbeSourceAdapter`。
- `crates/ruviform-audio/Cargo.toml` — dev-dependencies に
  `ruviform-observability` `features = ["integration"]`（test のみ
  `ObservationRunner` を使うため。library 依存は feature なしのまま
  — downstream への feature 漏れなし）。
- `crates/ruviform-audio/tests/typed_probe_source.rs` 新設 — 8 test:
  deck/readings 写像（opaque id・mode・availability・generation・
  clock・status・累積 counter）、`Inferred`/`Unavailable` の
  non-flattening、Off/Capture status による applied mode reconcile、
  `decks()` が capture chunk を消費しないこと、rebind の
  generation/availability/skew、`DebugInspector::add_probe_source`
  + `inspect`、`ObservationRunner::debug_mut()` 配線、install error
  の typed 化。`assert_impl_all!(Send, Sync)`。
- `crates/ruviform-observability/tests/integration.rs` — edge 強化:
  - `reconfigure_reports_live_sources_and_requires_reregistration`:
    live source が `non_migrated.sources_requiring_reregistration=1`
    で報告され、旧 pipeline から消え `registration_port()` への
    再登録で新 pipeline が観測する。
  - `reconfigure_filter_change_applies_to_new_pipeline_only`:
    `history_max_samples=0`/`stale_after` 変更が新 pipeline にのみ
    効き、旧 history が漏れない。
  - `debug_contract` module: stub `ProbeSource` を runner
    `debug_mut()` に登録し `inspect` で Probes section が出ること、
    `DebugReport.notes` が secret/media/path/credential/prompt/
    stderr 非搭載と payload 非 export を宣言することを contract
    test 化。
- `crates/ruviform-observability/src/integration/runner.rs` +
  `README.md` — GUI/consumer 購読境界を明文化: 正本は pull API
  （`collector_handle()`/`snapshot()`/`latest_batch()`/`history()`/
  `series()`）のみ。push `Subscription`/fan-out/callback は OBS-07
  no-fan-out 規約により**設計上存在しない**（欠落機能ではない）。
- `mise.toml` — `test-observability-msrv` 追加
  （`mise exec rust@1.89.0 -- cargo test -p ruviform-observability
  --all-features`、`CARGO_TARGET_DIR=target/msrv-observability`）。
  `test-ofx-msrv`/`test-ui-test-msrv` の先例に倣う。

### Decisions

- Adapter は `Arc<Mutex<..>>` 共有状態を持つ `Clone` handle —
  collector handle（OBS-07）と同じ non-RT 共有パターン。登録用
  clone と control/drain 用 original が同一 state を見る。
- `decks()` は telemetry のみ drain し capture chunk は絶対に
  消費しない — inspection が record stream を横取りしない。
  どちらが先に drain しても失うのは window 分の record のみで、
  cumulative counter は累積なので inspection の完全性は保たれる。
- `mode` は RT 確定値（status stream reconcile）を報告 —
  command 受理≠適用を区別する。`Installed` status は binding の
  staged mode を持たないため broker の commanded mode に解決する
  （install 時点では両者は一致）。
- `dropped`/`lost_blocks` は probe 帰属不能な deck-global 値を
  全 reading にミラー — 分割偽装をしない。`counts()` で
  `unobserved`/`unbound` も取得可能。
- `ProbeEntry` の broker 側 metadata（`bound_generation`/
  `availability`）は `on_publish` の atomic commit 成功後にのみ
  更新 — rebind 途中失敗で新旧が混ざらない。
- Feature DAG（`collector`/`diagnostics`/`debug-tools` 分割）は
  調査の結果変更しない: 3 subsystem は常時 compile で依存追加ゼロ、
  opt-in は `logging`/`tracing-export`/`allocation-probe`/`integration`
  の additive feature が担っており、always-on core + additive
  opt-in が現行設計の意図と判断（根拠は #293 spec/design の
  feature 節）。変更はスコープ外として記録のみ。

## Related Experiments

### Adopted

（なし）

### Rejected

（なし）

## Validation

| Lane | Command | Result |
| --- | --- | --- |
| fmt | `mise run fmt-check` | pass |
| check | `mise run check` | pass |
| lint | `mise run lint` | pass |
| test | `mise run test` | pass |
| test-rt | `mise run test-rt` | pass |
| MSRV | `mise run test-observability-msrv` | pass（rust@1.89.0・`--all-features`・288 unit + 15 integration + 28 logging + doctests） |
| adapter tests | `cargo test -p ruviform-audio --test typed_probe_source` | 8/8 pass |
| integration tests | `cargo test -p ruviform-observability --features integration --test integration` | 9/9 pass |
| doctest | `cargo test -p ruviform-audio --doc` | pass（`ProbeSourceAdapter` rustdoc example 含む 2/2） |

注: `mise run test` の初回実行で `ruviform-plugin-host` の
`wedged_child_write_converges_and_teardown_is_bounded` が spawn
timeout で 1 件失敗したが、単独再実行・workspace 再実行ともに
pass — 本 slice と無関係の timing flake（plugin-host の child
process spawn 計測）と判断し、最終 lane は全緑。

## Risks

- `decks()` と `drain_events()` は同一 telemetry/chunk channel を
  共有するため、どちらが先に drain してもその window の record は
  一方にしか見えない — cumulative counter は累積なので損失には
  ならないが、record 単位の観測は host が drain 責任を一本化する
  必要がある（rustdoc に明記済み）。
- `applied_mode` は status stream の reconcile であり、status
  record が channel-full で drop された場合は直前の確定値を保持
  する — loss は `dropped`/`lost_records` に計数され偽装しないが、
  mode の一時的な staleness はあり得る（bounded lossy channel の
  固有の制約）。
- `ProbeEntry` の broker metadata（`bound_generation`/
  `availability`）は `on_publish` の enqueue 成功後にのみ更新
  するが、RT 側 commit が失敗・遅延した場合は broker の見え方と
  RT の実 binding が一時的に乖離し得る — status stream
  （`RebindDropped`/`Unbound`）がそれを surface する設計。

## Follow-ups

- AudioLens #138・Synvelis #81 の consumer 統合（pull API 経由の
  probe inspection 利用を含む）— #281/#219 cross-repo gate。
- RT no-alloc/latency の実機計測・実 OS（Linux/macOS）検証 —
  #219/#223 gate。
- cross-repo gate（#281/#219/#294）・外部 consumer 受入 — NotRun。
- Feature DAG（`collector`/`diagnostics`/`debug-tools` feature
  分割）の再検討 — 現状 always-on が意図と判断したが、consumer
  側の要件が固まった時点で再評価。

## NotRun / Deferred

- AudioLens #138・Synvelis #81 consumer 統合・受入 — NotRun
  （#281/#219 cross-repo gate）
- RT no-alloc/latency の実機計測・実 OS 検証 — NotRun（#219/#223）
- cross-repo gate（#281/#219/#294）・外部 consumer 受入 — NotRun
- 実 hardware/device testing — NotRun
