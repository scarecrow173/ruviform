---
id: "ADR-034Z0u1KVcuWt7EQla0t16"
type: "adr"
status: "accepted"
title: "Issue #727 bounded 非 RT job executor primitive の配置 — ruviform-exec 採用"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
    - "docs/specs/issue-727-job-executor.md"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
  changes:
    added:
      - type: "doc"
        path: "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
    modified:
      - type: "file"
        path: "docs/designs/shared-foundations-and-modularization-policy.md"
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
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/issue-716-runtime-backend.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/offline-execution-driver.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/727"
  derived-by:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-727-job-executor.md"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "docs/designs/offline-execution-driver.md"
    - "crates/ruviform-exec/src/lib.rs"
    - "crates/ruviform-exec/src/job.rs"
    - "crates/ruviform-exec/src/offline.rs"
    - "crates/ruviform-exec/README.md"
    - "evidence/rf727/verification.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Issue #727 bounded 非 RT job executor primitive の配置 — ruviform-exec 採用

## Context and Problem Statement

synvelis#518（production job system 配線 — async scheduler・
preemption・結果 join）は、bounded な非 RT job executor
primitive を必要とするが、Ruviform 側には存在しない。
`crates/ruviform-exec` は `OfflineJob`（1 job = 1 専用 thread
の薄い接着層）と `OfflineDriver`（`run_parallel` 呼び出しごとの
scoped worker、永続 pool なし）を提供するが、複数 job を
bounded に受け付ける executor / scheduler primitive を意図的
に提供していない（`src/lib.rs` 冒頭・`src/job.rs`・
`README.md` で「registry / executor / 永続化はアプリ責務」
と明記）。

Issue #155（headless CLI foundation）では executor / スレッド
プールの共有化を「実需が出た時点で追加可能」として後送りして
おり、sv#518 がその最初の実需に相当する。本 ADR は、
この primitive を Ruviform の共通基盤として置くか、
Synvelis 等の consumer 側に置くかの責務判断を決裁する。

判断の核心は、bounded executor primitive と Job Registry の
切り分けである。synvelis-driven 実行境界 ADR Decision 6 は
「汎用実行契約 = Ruviform、Job Registry・製品固有 semantics =
Synvelis」と確定済みであり、executor がどちら側の契約に
属するかを本 Issue で決める必要がある。

## Decision Drivers

* **汎用実行契約の所在**: bounded worker pool・bounded
  queue・協調 cancel・結果 join は consumer 非依存の実行
  機構であり、synvelis-driven 実行境界 ADR Decision 6 の
  「汎用実行契約 = Ruviform」に整合する。
* **実需の実在**: sv#518 が async scheduler・preemption・
  結果 join を要求する最初の実需。shared-foundations
  L336-337 の「汎用利用者の実在」トリガを満たす。
  al-job（audio-lens）は現状 smoke test 利用のみだが潜在
  consumer として記録する。
* **既存 primitive との合成**: `CancellationToken`・
  `Progress`・`JobScope`・`JobState`/`JobEvent`/`JobOutcome`/
  `JobError` は既に job / driver 共用の契約として存在し、
  executor はこれらを再利用する薄い層で済む。
* **依存方向・層制約**: baseline のクレート表で
  `ruviform-exec` は Primitives 層・非 RT 専用・許可依存は
  `ruviform-math`/`ruviform-rt` 等のみ。executor は新規依存
  なしに `std::thread`/`std::sync` で実装可能。
* **module-first 原則**: shared-foundations の分割判断基準
  （L299-312）では、独立 compile/OS/配布境界がなければ
  crate 分割しない。同一依存層・同一 RT contract・同一
  consumer（synvelis-render は既に `ruviform-exec` 依存）
  のため module 追加が妥当。
* **境界の明確さ**: Job Registry・job ID・永続化・retry/
  resume・storage quota・user-facing scheduling は
  consumer 責務のまま残し、primitive は実行機構のみに
  限定することで責務侵食を防ぐ。

## Considered Options

* **Option 1**: Ruviform `ruviform-exec` に bounded executor
  primitive を module として追加する。
* **Option 2**: Ruviform に新規 crate（例: 専用 executor
  crate）を追加する。
* **Option 3**: consumer 側（Synvelis の job system 層）に
  executor を実装する。
* **Option 4**: 既存 `OfflineJob` を拡張して pool 化する
  （1 job = 1 thread 契約を変更する）。

## Decision Outcome

Chosen option: "**Option 1** — Ruviform `ruviform-exec` に
bounded executor primitive を module として追加する",
because 汎用実行契約は Ruviform の責務（synvelis-driven 実行
境界 ADR Decision 6）であり、sv#518 が最初の実需として
実在する。依存方向・RT contract・consumer 依存が既存
`ruviform-exec` と完全に一致するため module 追加で十分で、
crate 分割トリガに該当しない。consumer 側実装（Option 3）
は同じ bounded executor を Synvelis が自前で持つことになり、
将来の第 2 consumer（al-job 等）で重複実装と境界移行
コストを生む。新規 crate（Option 2）は独立境界の必要が
なく module-first 原則に反する。`OfflineJob` 拡張
（Option 4）は「1 job = 1 専用 thread」の契約を変え、
既存 consumer（headless CLI 等）の前提を崩す。

### Consequences

* Good, because consumer は Job Registry・スケジューリング
  方針・永続化・UI 配線に集中でき、cancel/preemption/結果
  join の契約が既存 `CancellationToken`/`Progress`/
  `JobScope` と整合する。
* Good, because bounded 保証（worker 数・queue 容量・
  in-flight 上限）が `ExecutorConfig` で構築時に固定され、
  RT audio path と完全に分離される。
* Good, because API 契約は
  `docs/designs/issue-727-bounded-nonrt-job-executor.md` に
  記録され、実装 Issue が契約に沿って進められる。
* Bad, because 協調 cancel は body が観測するまで効かず、
  無限 loop や長時間の非協調区間を持つ job は shutdown を
  block する。強制 preemption は提供しない（Rust の実行
  モデル上、安全な強制中断は不可能）。
* Bad, because blocking `submit` を pool worker 上の job body
  から呼ぶと deadlock し得る。contract で禁止を明記するが、
  consumer 側の誤用リスクは残る。
* Bad, because `OfflineJob` と `QueuedJob` で observe surface
  がほぼ重複する。実装時に内部 helper 共通化を検討するが、
  公開 API の統合は行わない。

### Confirmation

* 実装 Issue で `ruviform-exec` に executor module が追加
  され、`docs/designs/issue-727-bounded-nonrt-job-executor.md`
  の API 契約（`ExecutorConfig`/`JobExecutor`/`QueuedJob`/
  `SubmitError`/`ExecutorError`）に沿うことを確認する。
* sv#518 側で `RenderExecutor` port の実装として
  `JobExecutor` を注入し、async scheduler・preemption・
  結果 join が機能することを consumer 側で確認する
  （consumer gate — 本 ADR の範囲外）。
* docs-only slice のため、cargo 系検証は NotRun。文書検証は
  `doc_status` lint・`route_graph`・`prek` で実施済み
  （`evidence/rf727/` 参照）。

## Implementation Plan

* Affected paths: `crates/ruviform-exec/src/`（新規 module
  追加予定 — 実装は別 Issue）、`crates/ruviform-exec/README.md`・
  `crates/ruviform-exec/src/lib.rs`（export 追記予定）。本
  slice で変更するのは docs のみ。
* Patterns to follow: `OfflineJob`/`OfflineDriver` の
  `JobScope`/`CancellationToken`/`Progress`/`JobState`/
  `JobEvent`/`JobOutcome`/`JobError` 契約、bounded
  `sync_channel` による back-pressure、`catch_unwind` による
  panic 分離、`#[non_exhaustive]` 公開 enum。
* Patterns to avoid: Job Registry・job ID・永続化・retry/
  resume の Ruviform への取り込み、強制 preemption、RT audio
  path との混在、外部 pool crate への依存追加、
  `OfflineJob` の pool 化（1 job = 1 thread 契約の破壊）。
* Dependencies or configuration: 新規依存なし
  （`std::thread`/`std::sync` のみ）。Cargo.toml /
  Cargo.lock の変更は実装 Issue で判断するが、外部 crate
  追加は不要の見込み。

## Verification

* [x] 責務判断が二者択一で結論づけられ、根拠が記録された
  （本 ADR の Decision Outcome）。
* [x] ruviform 側採用の場合の API 設計が作成され、bounded
  性・preemption・cancel の契約が定義された
  （`docs/designs/issue-727-bounded-nonrt-job-executor.md`）。
* [x] sv#518 からの参照が stable URL で可能
  （`relations.source` に sv#518 URL を記載）。
* [x] shared-foundations L282 が指摘する baseline
  「スレッドプール／ワーカープール」stale 表記の扱いを記録
  （下記「baseline stale 表記の扱い」参照）。
* [ ] 実装後の動作検証 — 実装 Issue の範囲（本 slice は
  docs-only）。

## baseline stale 表記の扱い

shared-foundations-and-modularization-policy.md L282 は、
baseline `ruviform-architecture-design-baseline.md` の
「スレッドプール／ワーカープール」表記が Issue #24 の実装
（`run_parallel` ごとの `std::thread::scope` scoped worker）
より前の記述であり、次回 baseline 改訂時の更新候補と指摘
している。本 ADR の判断（bounded executor を `ruviform-exec`
module として追加）は、この stale 表記と整合する —
baseline が記述する「ワーカースレッドプール」はまだ存在
しないが、本 ADR 採用後の実装で `ruviform-exec` 内に
bounded pool が追加される予定であり、baseline 改訂時に
「Primitives 層 `ruviform-exec` に bounded 非 RT job
executor を持つ」旨を反映すべきである。

**結論**: baseline 本文の改訂は本 slice の範囲外
（baseline 本文改訂は別 Issue）。shared-foundations L282
の指摘自体は正確であり、stale な行番号参照
（L240/L352/L392 → 実際は L262/L375/L415 付近）のみを
最小修正し、本 ADR への参照を追加する。baseline 本文の
「ワーカースレッドプール」表記の改訂要否は **改訂が必要**
と判断するが、実施は baseline 改訂 Issue に委ねる。

## Pros and Cons of the Options

### Option 1: ruviform-exec module 追加

* Good, because 汎用実行契約の所在が synvelis-driven 実行
  境界 ADR Decision 6 と整合し、consumer は job system
  semantics に集中できる。
* Good, because 既存 `CancellationToken`/`Progress`/`JobScope`/
  `JobState`/`JobEvent`/`JobOutcome`/`JobError` を再利用でき、
  API surface が `OfflineJob` と整合する。
* Good, because 依存追加ゼロ・RT 分離・bounded 保証が
  `ExecutorConfig` で構築時に固定される。
* Bad, because 協調 cancel の限界と submit deadlock リスクを
  contract で明示する必要がある。
* Bad, because `OfflineJob` との observe surface 重複が生じ、
  内部共通化の検討が必要になる。

### Option 2: 新規 crate 追加

* Good, because executor を独立した compile/配布単位にできる。
* Bad, because 依存方向・RT contract・consumer が既存
  `ruviform-exec` と完全に一致し、独立境界の必要がない。
  shared-foundations の module-first 原則に反し、crate 数を
  不必要に増やす。
* Bad, because `ruviform-exec` の既存 primitive
  （`CancellationToken`/`Progress`/`JobScope`/`JobState` 等）
  への依存が発生し、結局 `ruviform-exec` かその依存先を
  参照する構造になる — 分離の利益が薄い。

### Option 3: consumer 側（Synvelis）実装

* Good, because Synvelis の job system semantics（Registry・
  retry・永続化）と executor を一体で設計でき、product-specific
  な最適化がしやすい。
* Bad, because bounded executor の実行機構自体は consumer
  非依存の汎用契約であり、synvelis-driven 実行境界 ADR
  Decision 6 の「汎用実行契約 = Ruviform」に反する。
* Bad, because 将来の第 2 consumer（al-job 等）で重複実装が
  発生し、cancel/progress/bounded queue の契約が consumer
  ごとに分岐するリスクがある。
* Bad, because sv#518 が必要とする `CancellationToken`/
  `Progress`/`JobScope` は既に `ruviform-exec` に存在し、
  consumer 側で再実装すると既存 primitive との整合を
  別途維持する必要がある。

### Option 4: OfflineJob を pool 化

* Good, because 既存 `OfflineJob` の surface をそのまま pool
  worker で使い回せる。
* Bad, because `OfflineJob` の「1 job = 1 専用 thread」契約を
  変更し、既存 consumer（headless CLI・`OfflineDriver` 内部
  利用等）の前提を崩す。
* Bad, because pool 化により `OfflineJob::spawn` が queue
  満杯で block する可能性が生じ、CLI 等の単純な利用経路に
  executor lifecycle 管理を強いる — 責務が混濁する。

## More Information

* Issue: https://github.com/scarecrow173/ruviform/issues/727
* Consumer: https://github.com/scarecrow173/synvelis/issues/518
* 関連 ADR:
  `docs/adr/synvelis-driven-generic-execution-boundaries.md`
  （Decision 6 — 汎用実行契約 = Ruviform / Job Registry =
  Synvelis）
* 関連設計:
  `docs/designs/issue-727-bounded-nonrt-job-executor.md`（API
  契約）、`docs/designs/offline-execution-driver.md`、
  `docs/designs/issue-155-headless-cli-foundation.md`（executor
  後送りの経緯）、`docs/designs/shared-foundations-and-
  modularization-policy.md`（module-first・分割判断基準）
* 実装参照: `crates/ruviform-exec/src/lib.rs`・`src/job.rs`・
  `src/offline.rs`・`crates/ruviform-exec/README.md`
* consumer 参照（read-only）:
  `synvelis/crates/synvelis-render/src/render_job/
  orchestrator.rs`（`RenderExecutor` port・同期
  `run()`/`drive()`・`ruviform_exec::CancellationToken` 使用）、
  `audio-lens/crates/al-job/`（smoke test 利用のみ）
