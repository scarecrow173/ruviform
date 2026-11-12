---
id: "IMPL-034UpDoz64058mto7srnOJ"
type: "impl"
status: "completed"
title: "Issue #290 OBS-08 allocation-probe 実装記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-observability/src/profiling/"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-290-observability-allocation-probe.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-290-observability-allocation-probe.md"
    - "docs/designs/issue-290-observability-allocation-probe.md"
    - "docs/test-specs/issue-290-observability-allocation-probe-tests.md"
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

# Issue #290 OBS-08 allocation-probe 実装記録

## Summary

Issue #290（OBS-08）のうち CountingAllocator + AllocationScope の
slice を、`allocation-probe` feature で gate した
`ruviform_observability::profiling` モジュールとして実装した。
`CountingAllocator<A>` は 4 つの `GlobalAlloc` API を内側 `A` へ
そのまま委譲しつつ、呼出/成功/失敗/要求バイト数と live/peak
要求バイトをプロセス累積（atomic）と呼出スレッドの TLS scope
（Cell）の両方に記録する。`AllocationScope`/`AllocationWindow` は
const 初期化 TLS の intrusive linked list でネスト・非 LIFO drop・
スレッド終了・`mem::forget` に耐える。crate は `#[global_allocator]`
を一切 install しない。RT Timing・CPU sampling・Tracy・consumer
回帰評価は本 slice 対象外（Issue 残件）。

## Context

`docs/tasks/implement-issue-290-observability-allocation-probe.md`
の実装スライス。上流: `docs/specs/`・`docs/designs/`・
`docs/test-specs/issue-290-observability-allocation-probe-tests.md`。
既存契約: `ruviform-rt::alloc_counter`（`#[cfg(test)]` 限定・
イベント数のみ）は変更せず共存。OBS-09 `diagnostics` とは module
境界で分離（main merge で併存確認済み）。

## Implementation

### Changes

- `crates/ruviform-observability/Cargo.toml`: `allocation-probe = []`
  feature を追加（依存ゼロ）。
- `crates/ruviform-observability/src/lib.rs`: feature-gated
  `pub mod profiling;` と crate doc（module 一覧・RT contract・
  dependency policy）への additive 記述。
- `src/profiling/mod.rs`: モジュール doc — 計測対象（要求バイト、
  RSS/実ヒープ/leak 非主張）、スレッド限定の明示、RT contract 表、
  `#[global_allocator]` は host 責務、`alloc_counter`/`play_sine`
  共存判断、CPU sampling/Tracy nonRT 制約、オーバーヘッド構造。
- `src/profiling/stats.rs`: `OpStats`/`AllocationStats`（公開
  snapshot、`Clone+Copy+Debug+Default+PartialEq+Eq`）、`Op`、
  `AllocationCounters` trait（record_alloc/dealloc/realloc の
  単一実装 — global/scope で意味論を共有）、飽和 helper
  （`compare_exchange` CAS ループ — `fetch_update` は deprecated、
  `try_update` は MSRV 1.89 超過のため）。
- `src/profiling/allocator.rs`: `GlobalStats`（AtomicI64 current +
  AtomicU64 peak + op 毎 AtomicU64×4）、`CountingAllocator<A>`
  （`const fn new`/`inner`/`stats`、4 API の unsafe impl —
  委譲後に global stats → `for_each_active_scope` の順で記録）。
- `src/profiling/scope.rs`: const-init `HEAD: Cell<usize>`、
  `ScopeNode`（parent/active/Cell カウンタ群）、`AllocationScope`
  （`Box<ScopeNode>` + `PhantomData<*const ()>` で !Send+!Sync）、
  `AllocationWindow<'a>`（`&'a mut` + `PhantomData<*mut ()>`）、
  `enter`/`stats`/`reset`/`measure`/Drop unlink（head 一致または
  predecessor の parent 付替えで任意順 unlink、`try_with` Err は
  計数なしに縮退）。
- `src/profiling/testutil.rs`: `ProbeAlloc`（API 別呼出記録・
  失敗注入・aligned fake ptr、実メモリ不使用）と `call_*` helper
  （`#[expect(unsafe_code)]` 集約）。
- テスト: allocator 16 件 + scope 13 件 + stats 4 件 + doctest 1 件
  = 34 件。

### Decisions During Implementation

- **`AllocationScope` を !Send+!Sync に変更（設計との差分）**:
  設計初版は `Send` 想定だったが、`mem::forget(window)` で window を
  漏らした scope の node は enter したスレッドの TLS chain に残り、
  scope を別スレッドへ move して drop すると unlink が誤った TLS を
  走査し node が dangling（UAF）+ `Cell` データ競合になる。
  `PhantomData<*const ()>` で型レベルに閉じ、設計書・risk 表も
  同期した。計測は「そのスレッドで new→enter」が自然な形と一致。
- **CAS ループを自前実装**: `fetch_update` は toolchain 1.99 で
  deprecated、`try_update` は MSRV 1.89 未満。`compare_exchange_weak`
  の飽和 CAS ループで global current/peak を更新（peak は各操作の
  post-value を `fetch_max` — 中間値は必ずいずれかの操作の new と
  して観測されるため競合下でも正確）。
- **厳密 provenance**: node アドレスは `expose_provenance`/`usize`
  保存・`with_exposed_provenance` で復元（`as` cast による
  provenance 喪失と `cast_ptr_alignment` を回避、MSRV 1.89 で利用可）。
- **計数は inner 呼出「後」に一括実行**: request/result を分けず、
  inner が返った後に `record_*` を 1 回呼ぶ。順序は観測に影響せず、
  inner が panic した場合（契約違反）に部分計数が残らない。
- **realloc は分解しない**: ProbeAlloc の API 別カウンタで
  `realloc_calls==1` かつ `alloc/dealloc` 不変を検証。bytes は
  `new_size`、current は `new-old` の delta（失敗時は旧 block 生存
  で無変更）。
- **テストは `#[global_allocator]` を使わない**: lib test binary は
  既に `AllocDisabler` を global allocator に持つため、計測は
  `CountingAllocator` インスタンスの `GlobalAlloc` メソッド直接
  呼出で行う（testutil の `call_*`）。

## Related Experiments

### Adopted

（なし）

### Rejected

（なし）

## Validation

実施環境: Windows 11 x64・Git Bash・Rust 1.99.0（mise pin）。

| Check | コマンド | 結果 |
| --- | --- | --- |
| feature check | `cargo check -p ruviform-observability --features allocation-probe` | PASS（補助証拠） |
| feature test | `cargo test -p ruviform-observability --features allocation-probe` | PASS — 34 profiling tests + doctest（補助証拠） |
| feature clippy | `cargo clippy -p ruviform-observability --features allocation-probe --all-targets -- -D warnings` | PASS（補助証拠） |
| dep tree | `cargo tree -p ruviform-observability --features allocation-probe -e normal` | PASS — 新規外部依存なし（補助証拠） |
| fmt | `mise run fmt-check` | PASS（exit 0） |
| workspace check | `mise run check` | PASS（exit 0、`cargo check --workspace --all-targets`） |
| workspace lint | `mise run lint` | PASS（exit 0、clippy `-D warnings`） |
| workspace test | `mise run test` | PASS（exit 0、`cargo test --workspace`、181 suite ok / 0 failed） |

merge 後（origin/main 取込・OBS-09 diagnostics 併存）に
`cargo test -p ruviform-observability --features allocation-probe`
再実行: 175 tests PASS。最終 `mise` 4 タスクは merge 後の tree で
再実行して記録する（PR 本文に実測値を記載）。

### 未実施（理由付き）

- `cargo test` での `allocation-probe` + `logging` 同時 feature:
  本 slice は feature 独立（`profiling` は logging 非依存）。
  `--all-features` 全組合せは canonical lane 外。
- MSRV 1.89 での直接 build: toolchain は 1.99.0 pin。使用 API は
  全て 1.89 以前に安定化したもの（`expose_provenance` 1.84、
  `next_multiple_of` 1.73、const thread_local 1.59）だが、
  MSRV lane 実測は #223 lane の責務として残す。
- 実機 overhead 計測・OS/feature matrix: #281 gate（cross-repo、
  本 slice 対象外）。
- RT Timing・CPU sampling・Tracy・AudioLens/Synvelis 回帰:
  Issue #290 の残件（PR 本文に slice 境界を明記）。

## Audit（2026-11-09、chore/obs-08-09-10-audit、base `0d07471d`）

Issue #290 受入条件の行ごとの照合結果。実装・試験は main（PR #506
`7816a22`）と本ブランチの `rt_timing` slice による。

| 受入行 | 判定 | 証跡 |
| --- | --- | --- |
| CountingAllocator 4 GlobalAlloc API・失敗/zero/realloc/align/unsafe invariant 試験 | 充足（既存） | `src/profiling/allocator.rs` 16 tests、`stats.rs` 4 tests（#506） |
| TLS Scope nested/concurrent/thread teardown/process-wide gap 検証 | 充足（既存） | `src/profiling/scope.rs` 14 tests（#506） |
| Allocator 内部に alloc/panic/tracing/重い lock/再帰なし（静的・動的） | 充足（既存） | allocator/scope tests + 設計文書の不変条件節 |
| 既存 `ruviform-rt::alloc_counter`/Example の移行/共存判断文書化・RT lane 保持 | 充足（既存） | 本 IR Decisions + `mise run test-rt` PASS |
| RT Timing の Clock/overrun/Callback budget 意味論と Histogram 統合試験 | **本 PR で実装** | `src/rt_timing/`（21 tests: `assert_no_alloc`・drop 帰属・clock anomaly 分離・budget 意味論・generation・Collector→`MetricValue::Histogram`）。docs chain: `docs/{specs,designs,test-specs,plans,tasks}/issue-290-observability-rt-timing*`、IR `issue-290-obs-08-rt-timing.md` |
| CPU sampling 手順と Tracy 等任意 nonRT Layer の制約を文書化 | 充足（既存） | `docs/designs/issue-290-observability-allocation-probe.md`「CPU sampling / 任意 nonRT Layer の制約」節 |
| Enabled/Disabled で AudioLens 分析精度・Synvelis 再生性能の回帰評価 | NotRun | cross-repo（#281 gate）・本タスク対象外 |
| MSRV/feature matrix/OS ごとの証拠を #281 へ提示 | NotRun | cross-repo gate・本タスク対象外 |

## Risks

- `mem::forget(window)` 後の scope は再 enter 不可（panic）で
  drop/再作成が回復手段 — 設計どおり文書化済み。
- TLS dtor 内の割当は `try_with` Err で計数なしに縮退（panic しない
  ことを `tls_destructor_allocation_does_not_panic` で検証）。
- global stats の snapshot は torn read になり得る（各 op 内の
  順序は保持、文書化済み）。
- scope カウンタは累積（window 毎 reset なし）— `measure` または
  `reset` 差分で区間計測する設計。

## Follow-ups

- Issue #290 残件: ~~RT Timing（Clock/overrun/budget + Histogram）~~
  → `issue-290-obs-08-rt-timing.md` で実装済み（2026-11-09 audit
  ブランチ）。CPU sampling 手順の実機検証、Tracy 等 nonRT layer、
  AudioLens/Synvelis 回帰・#281 証拠は引き続き残件。
- `ruviform-rt::alloc_counter` / `play_sine` の移行判断は
  第三 consumer または broader-than-test 需要が出た時点で再検討
  （本 slice は共存を選択・文書化済み）。
