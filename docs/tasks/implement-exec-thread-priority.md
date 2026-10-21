---
id: "TASK-034JxOaMsaSwEIVfvU9MC8"
type: "task"
status: "done"
title: "ruviform-exec スレッド優先度機構の実装"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-exec/src/thread_priority.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-exec/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-exec/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-rt-foundation.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-rt-foundation.md"
    - "implement-rt-core-types.md"
  blocks:
    - "sync-rt-foundation-docs-and-verify.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-rt-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/thread-priority-lifecycle.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# ruviform-exec スレッド優先度機構の実装

## Work

plan `PLAN-034JxNegJktSAJZ2LuO0N8` の Phase E。design `DESIGN-034JgsK9khgDsPqG04Sp6i` の `thread_priority` API sketch を正本として、OS 提供の RT スレッド優先度付与（Windows MMCSS・macOS time-constraint・Linux `SCHED_FIFO` 系）を `ruviform-exec` の `non-RT only` API として実装する。

- [x] `crates/ruviform-exec/Cargo.toml` に `audio_thread_priority = { version = "0.38", default-features = false }` を追加（`with_dbus`/`dep:dbus` 経路を排除。`Cargo.lock` を更新）
- [x] `thread_priority.rs`: `ThreadPriorityParams`（private フィールド。`audio(buffer_frames, sample_rate) -> Self` — 純粋な値構築 `RT-suitable`・用途は非 RT セットアップ区間と doc 記載）
- [x] `promote_current_thread_to_realtime(params) -> Result<ThreadPriorityGuard, PriorityError>` — `**RT contract:** non-RT only`。self-promotion 呼び出しモデル（exec 管理スレッド: spawn 時セットアップ／デバイスコールバック由来スレッド: RT 契約区間開始前の初回コールバック preamble）と「active な RT 契約区間内からの呼び出しは契約違反」を doc に verbatim 記載。失敗は `Err(PriorityError)` で返しパニックしない。`default-features = false` 構成で非特権 Linux（rtkit/`CAP_SYS_NICE`/`RLIMIT_RTPRIO` なし）では典型的に失敗し `Err` が返る仕様上の失敗挙動と、対象外プラットフォームでの no-op 成功可能性を doc に記載
- [x] `demote_current_thread_from_realtime(guard) -> Result<(), PriorityError>` — guard を消費し二重 demote を型で防止（`non-RT only`）
- [x] `ThreadPriorityGuard`（opaque・内部ハンドル非公開）: `Drop` で demotion を best-effort 試行（失敗は捨てる — 検出が必要なら `demote` を使う旨 doc 記載。`Drop` が syscall 伴うため RT 契約区間内での生成・破棄を禁止する旨記載）
- [x] `PriorityError`: enum レベル `#[non_exhaustive]`・`PromotionFailed(&'static str)`/`DemotionFailed(&'static str)`・`Debug`/`Clone`/`Copy`/`PartialEq`/`Eq`/`Hash` + `Display` + `core::error::Error`。exec に crate 共通 `Error`/`Result` は作らない（失敗系統 1 系統のみ — YAGNI）
- [x] `lib.rs` に `mod thread_priority;`（private）+ `pub use thread_priority::{demote_current_thread_from_realtime, promote_current_thread_to_realtime, PriorityError, ThreadPriorityGuard, ThreadPriorityParams};` を追加。crate doc に thread_priority 責務を追加し Contract source を更新（「RT プリミティブは持たない」記述は維持 — promotion は RT プリミティブではなく非 RT の OS スレッドサービス）
- [x] 外部型非露出: `audio_thread_priority` の型・エラー・feature 名が公開 API に出ないこと（依存 feature 名の再露出に相当する独自 feature も追加しない）
- [x] promotion スモークテスト: spawn したスレッドから `promote` を呼び `Ok`/`Err` いずれでもパニックせず `Result` が返ることを確認（**`rt_` を含まない命名** — 例: `promotion_returns_result`。非特権 Linux では `Err` が期待値だがプラットフォーム依存のため成否は assert しない）

**`rt_` 命名規約（重要）**: 本機構は `non-RT only` API であり RT 契約テストではない。テスト関数名・モジュールパス・ヘルパー関数名は **`rt_` 部分文字列を含まない**こと — `assert_*`・`start_*` は `rt_` を内包するため禁止（`expect_*`・`spawn_*` 等を使う）。`mise run test-rt`（`cargo test --workspace rt_`）の部分文字列フィルタに本テストが混入しないことを確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-rt-foundation.md](../plans/implement-rt-foundation.md) — `PLAN-034JxNegJktSAJZ2LuO0N8`（approved）。Phase E の正本
- [../designs/rt-foundation-api-design.md](../designs/rt-foundation-api-design.md) — `DESIGN-034JgsK9khgDsPqG04Sp6i`（approved）。thread_priority API sketch 正本
- [../adr/rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md) — `ADR-034Jcra57UlCiVE1Poq2DO`（accepted）。`audio_thread_priority` 採用・`default-features = false`・exec 配置の決定記録

## Dependencies

- 前提: `implement-rt-core-types.md`（RT ラベル・enum 方針・`rt_` 命名規約の共有のみ。実装依存なし — `implement-rt-spsc-channel.md` 〜 `implement-rt-snapshot-exchange.md` と並行可能）
- 後続をブロック: `sync-rt-foundation-docs-and-verify.md`
- `unsafe` 不使用（`audio_thread_priority` の safe API のみ利用）

## Affected Files

- `crates/ruviform-exec/src/thread_priority.rs` — 新規
- `crates/ruviform-exec/src/lib.rs` — crate doc・`mod`/`pub use` 追加
- `crates/ruviform-exec/Cargo.toml` — `audio_thread_priority = { version = "0.38", default-features = false }` 追加
- `Cargo.lock` — `audio_thread_priority` の記録

## Done When

- [x] 5 つの公開 API（`promote_*`/`demote_*`/`ThreadPriorityParams`/`ThreadPriorityGuard`/`PriorityError`）が design sketch どおり crate root から再公開される
- [x] `promote` が `Result` を返しパニックしないこと・self-promotion モデル・`non-RT only`・非特権 Linux 失敗挙動が API doc に記載される
- [x] promotion スモークテストが `rt_` を含まない命名で存在し、`cargo test --workspace rt_` で選択されない
- [x] `audio_thread_priority` 型・エラー・feature 名が公開 API に露出していない（シグネチャレビュー）
- [x] `PriorityError` に `#[non_exhaustive]`、全公開 API に `**RT contract:**` ラベルと doctest

## Verification

- [x] `mise exec -- cargo test -p ruviform-exec` — 単体テスト + doctest 合格
- [x] `mise exec -- cargo test --workspace rt_` — promotion スモークテストが選択されない（`rt_` 非含有命名の確認）
- [x] `mise exec -- cargo clippy -p ruviform-exec --all-targets -- -D warnings` — 0 warnings
- [x] `mise run fmt-check` — 差分なし
- [x] `mise exec -- cargo metadata --format-version 1 --no-deps` — `ruviform-exec` の依存に `audio_thread_priority`（`default-features = false`）が記録される
