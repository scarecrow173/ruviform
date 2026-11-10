---
id: "IMPL-034V68pdzoGf0crvlem0Zy"
type: "impl"
status: "completed"
title: "Issue #275 Observability integration design reconciliation and contract-test inventory"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-275-observability-integration-design.md"
    - "docs/designs/issue-275-observability-integration-design.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-275-obs-design-reconciliation.md"
      - type: "file"
        path: "crates/ruviform-observability/tests/rt_boundary.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/tests.rs"
      - type: "file"
        path: "docs/designs/issue-275-observability-integration-design.md"
      - type: "file"
        path: "docs/test-specs/issue-275-observability-integration-design.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-275-observability-integration-design.md"
    - "docs/specs/issue-275-observability-integration-design.md"
    - "docs/test-specs/issue-275-observability-integration-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-278-obs-03-tracing-export-acceptance-verification-slice.md"
    - "docs/impl/ir/issue-290-obs-08-allocation-probe.md"
    - "docs/impl/ir/issue-291-obs-09-diagnostics.md"
    - "docs/impl/ir/issue-292-obs-10-debug-inspection.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-275-observability-integration-design.md"
    - "docs/test-specs/issue-275-observability-integration-design.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-275-observability-integration-design.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #275 Observability integration design reconciliation and contract-test inventory

## Summary

Issue #275（Observability統合設計正本 — 単一クレート・
OBS-01〜11・両Consumer、2026-11-04 合意版）の
**実装照合・設計書更新・契約試験棚卸し** slice。
main 上の `crates/ruviform-observability` 実装（OBS-01〜11
の全 per-issue 実装 Issue が CLOSED）と Issue 本文の合意版
アーキテクチャを行単位で照合し、headless で検証可能な
契約試験ギャップのみを補完した。設計正本
`docs/designs/issue-275-observability-integration-design.md`
の実装 snapshot を 2026-11-10 時点へ更新した。
実装 API・schema・feature 構成の変更はない。

## Context

2026-11-09 の docs-only slice（PR #521）で spec → design →
test-spec → plan → task の文書チェーンと #101/#103 前提
2 文書への合意版改訂注記が作成済み。本 slice はその後の
状態変化 — OBS-03 #278・OBS-07 #289・OBS-08 #290・
OBS-09 #291・OBS-10 #292・OBS-11 #293 の全実装 Issue が
CLOSED となり、設計正本の「実装の現状」snapshot（当時
OPEN と記録）が陳腐化したこと — を受けた追従 slice で
ある。受入条件のうち headless 検証可能な行
（単一 crate/Feature DAG・RT call path・契約試験・
quiescence→final drain→unregister の競合安全・外部故障
耐性）について、既存試験の棚卸しと不足分の補完を行う。

## Implementation

### Changes

- `crates/ruviform-observability/tests/rt_boundary.rs`（新規）:
  RT-suitable 操作が subscriber dispatch 設置下でも
  `tracing` event を一切発行しないことを固定する
  contract test 3 件。`required-features = ["tracing-export"]`
  は `dep:tracing` を有効化する最小 feature であり、
  counting subscriber は dev-dependency
  `tracing-subscriber` の layer を `with_default` で
  scoped 注入する（library は global default を install
  しない）。各 test は control `tracing::info!` で dispatch
  の生存を確認し、0 件計数が dead subscriber でないことを
  保証する。
  - `rt_core_primitive_ops_emit_no_tracing_events` —
    `Counter`/`Gauge`/`Histogram` の RT writer/reader 操作。
  - `rt_bridge_and_record_ops_emit_no_tracing_events` —
    `RecordHeader`/`EventRecord`/`ClockContext`/
    `CorrelationLink` の const accessor と
    `RtRecorder::try_record`/`dropped`/`recorded`。
  - `rt_timing_ops_emit_no_tracing_events` —
    `TimingRecorder::record`/`dropped`/`recorded`（`lib.rs`
    が "no tracing/subscriber call" と宣言する操作）。
- `crates/ruviform-observability/src/collector/tests.rs`:
  `abandoned_leases_racing_shutdown_still_drain_partial`
  を追加。8 source の lease を `retire` なしで個別 thread
  から drop（abandon）しつつ main thread が
  `Collector::shutdown` を実行 — drop が shutdown drain
  の前/中/後のどこに落ちても、全 source が `Partial` +
  `WriterNotQuiesced` で `Removed` に到達し
  `sources_removed_total == 8` となることを固定。
  既存の `abandoned_source_drains_as_partial_writer_not_
  quiesced`（逐次）と `register_retire_and_poll_race_
  without_deadlock`（登録競合）を補完し、
  「quiescence→final drain→unregister が競合を含めて
  安全」の abandon×shutdown 組合せを直接検証する。
- `crates/ruviform-observability/Cargo.toml`: 上記
  `[[test]] rt_boundary` target を `required-features =
  ["tracing-export"]` で登録（optional dep 追加なし・
  `Cargo.lock` 無変更）。
- `docs/designs/issue-275-observability-integration-design.md`:
  「実装の現状」snapshot を 2026-11-10 へ更新（#278/#289/
  #290/#291/#292/#293 を CLOSED に修正、#281/#294 OPEN・
  AL #138/SV #81 CLOSED を維持）。本 slice の契約試験
  追加と残る NotRun 項目を snapshot 節へ追記。
  front matter `updated` を 2026-11-10 に更新。
- docs: 本 IR のみ。spec/test-spec/plan/task の chain は
  契約・Rule の追加・変更がないため既存のまま
  （test-spec Risk 節の snapshot 日付記述は設計書側の
  更新に追従する範囲で許容）。

### Decisions During Implementation

- **新規 spec/design/plan/task chain は作らない**:
  `issue-275-*` の 5 文書チェーンは 2026-11-09 slice で
  approved 済みであり、本 slice はその設計正本に対する
  実装照合・証拠補完である。先行する受入再照合 slice
  （#278 PR #539・#291 PR #540・#292 PR #542）と同じく
  IR + contract test のみを成果物とした。
- **追加試験は 2 件のみに限定**: 棚卸しの結果、Issue
  #275 受入行のうち headless 検証可能な項目は既存の
  per-module test・crate `tests/` contract test・`rt_`
  レーンでほぼ充足済みだった。未充足だったのは (a)
  RT 操作の `tracing`/Subscriber 非発行を dispatch 設置下
  で直接固定する試験（`rt_` レーンは no-alloc のみ検証）、
  (b) abandon×shutdown 競合の直接試験、の 2 点のみであり、
  それ以外の「不足」は cross-repo・実機 RT・compile
  matrix 全組合せ等の NotRun 項目であって in-repo 試験で
  補えない。
- **`tracing-export` feature を rt_boundary の gate に採用**:
  `dep:tracing` を有効化する最小 feature。`logging` を
  要求すると subscriber 構築系の依存まで引くため、
  「RT path が tracing を呼ばない」という契約の検証に
  必要な最小構成を選んだ。
- **tracing_export の panic 伝播方針は変更しない**:
  `TracingExporter` は host subscriber 自体の panic を
  `catch_unwind` せず伝播させる（host の責務境界）、
  `TracingFindingSink` のみ sink panic を隔離する、
  という既存の非対称を棚卸しで確認し、そのまま維持した。

## Acceptance mapping（Issue #275 受入行・本 slice 対象）

| # | 受入行 | 判定 | 根拠 |
| --- | --- | --- | --- |
| 1 | 単一 crate/Feature DAG コンパイル確認・core 使用時に外部 Subscriber/Logging 非強制 | Pass（in-repo 分） | `Cargo.toml` `default = []`・optional dep は `logging`/`tracing-export` のみ・`cargo check -p ruviform-observability`（default）で外部依存なしを確認。feature 組合せの compile matrix 全網羅は #281 G0 → NotRun |
| 2 | RT call path に alloc/blocking lock/OS IO/`tracing`/Subscriber/無制限 queue/制御されない Drop がない | Pass（in-repo 分）+ NotRun | no-alloc/no-lock は `rt_*` レーン `assert_no_alloc` 系 test、`tracing`/Subscriber 非発行は本 slice の `tests/rt_boundary.rs` 3 test、bounded queue/drop 計数は `rt_bridge`/`rt_timing` test で固定。実機負荷下の保証は #281 → NotRun |
| 3 | Counter/Gauge/Histogram・Collector Snapshot・Diagnostics・Debug・CountingAllocator・Tracing Export・Logging の契約試験 | Pass | `core`/`collector`/`diagnostics`/`debug`/`profiling`/`tracing_export`/`logging`/`integration`/`rt_bridge`/`rt_timing`/`record` の unit test + `tests/{integration,logging,rt_boundary}.rs` contract test が全 module を網羅（棚卸しで欠落 module なし） |
| 4 | Source quiescence→final drain→unregister の競合安全・loss/stale/clock/epoch 非偽装 | Pass | 既存 lifecycle/drain/validity test に加え、本 slice の `abandoned_leases_racing_shutdown_still_drain_partial` で abandon×shutdown 競合を直接固定。stale/loss/clock/epoch 非偽装は collector/rt_bridge/rt_timing/diagnostics の既存 test で網羅 |
| 5 | AudioLens 旧 Logging 既定 OFF/`AL_LOG`/stderr/json/inspect stdout/daily 7 世代/機密・f64/samply 維持 | #281 NotRun | AL #138 CLOSED は Consumer repo 側受入。parity Golden 比較・実機検証は cross-repo gate（#281/#219/#294）→ NotRun |
| 6 | Synvelis Rust Host/Multiwindow/Playback/Plugin/Gateway で Subscriber/RT 安全/認可・Audit 分離 | #281 NotRun | SV #81 CLOSED は Consumer repo 側受入。Host 統合の実機検証は #281 G4 → NotRun |
| 7 | 外部 Provider/Exporter 故障で RT/Job 正本・Source のメトリクス/Query を損なわない | Pass（in-repo 分） | `issue-276` sink failure mode/health・`issue-293` drain/error isolation・`issue-291` sink panic isolation の既存 test（棚卸しで充足確認・追加不要） |
| 8 | MSRV・Linux/macOS/Windows・Cross-Repo #219/#281 に Evidence（未実行は NOT RUN） | 部分 Pass + NotRun | `mise run test-observability-msrv`（MSRV 1.89 + all features）は本 slice で実行。Linux/macOS・cross-repo evidence は #219/#294 → NotRun |
| 9 | #101/#103 の古い Logging 保留・クレート抽出前提を合意版として設計書更新し影響変更を記録 | Pass | 2026-11-09 slice で `foundation-observability-minimal-api.md`・`shared-foundations-and-modularization-policy.md` へ改訂注記済み（本 slice で変更不要・実在を再確認） |

## Related Experiments

### Adopted

（なし）

### Rejected

（なし）

## Validation

canonical（`mise run`、worktree = `feature/275-obs-design-authority`、
`git merge origin/main`（72b00642）取込後・Windows 11・
rust 1.99.0 pinned）:

- `mise run fmt-check` — exit 0。
- `mise run check`（`cargo check --workspace --all-targets`）—
  exit 0。
- `mise run lint`（`cargo clippy --workspace --all-targets --
  -D warnings`）— exit 0。
- `mise run test`（`cargo test --workspace`）— exit 0。
- `mise run test-rt`（`cargo test --workspace rt_`）— exit 0。
- `mise run test-observability-msrv`（`mise exec rust@1.89.0 --
  cargo test -p ruviform-observability --all-features`）— exit 0
  （lib 298 + integration 15 + logging 28 + rt_boundary 3 +
  doctest 15 全 pass、MSRV 1.89 toolchain）。

補助証拠（canonical = mise、直接 cargo・feature 組合せ）:

- `cargo test -p ruviform-observability --all-features --test
  rt_boundary` — 3 tests 全 pass。
- `cargo test -p ruviform-observability --features
  tracing-export --test rt_boundary` — 3 tests 全 pass
  （`required-features` の最小 feature 単独でも target が
  build・実行されることを確認）。
- `cargo test -p ruviform-observability --lib
  abandoned_leases_racing` — 1 test pass。
- `gh issue view`（#275〜#294）— Issue 状態 snapshot
  （2026-11-10）: #275/#281/#294 OPEN、#276〜#280・
  #289〜#293 CLOSED。
- `audit_docs.js --type all` — 本 slice の docs
  （issue-275 設計書・test-spec・本 IR）に対する finding 0
  （repo 全体の既存 finding は範囲外）。
- `doc_maintenance.js plan --type all` — 本 slice 起因の
  action 0。`docs/tasks/README.md` の index rebuild 指摘
  （index-duplicate-entry 等）は merge 後も残る main 既存の
  drift であり、本 slice は task 文書を変更していないため
  対象外として未対応（証拠として記録）。

## Risks

- `tests/rt_boundary.rs` は「宣言された RT 契約に tracing
  呼出しがない」ことを固定するものであり、実機 RT 性能・
  割込み応答を保証しない（それは #281 の実機 Gate）。
- `abandoned_leases_racing_shutdown_still_drain_partial`
  は thread 間の interleaving を yield で広げるが、全順序
  を網羅するものではない — state machine の不変条件
  （Partial drain・単一 Removed）は interleaving 非依存に
  assert している。

## Follow-ups

- 両 Consumer（AudioLens #138/Synvelis #81）の実機受入・
  parity Golden・RT latency 計測 — #281 OBS-06 gate・
  #219/#294 consumer-verify（NotRun）。
- compile matrix 全組合せ（`--no-default-features`・各
  feature・full consumer・OS）— #281 G0・CI lane（NotRun）。
- 実機 RT 保証（負荷時 latency・実機環境）— #281 段階
  Gate（NotRun）。
