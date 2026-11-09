---
id: "PLAN-034UoVUbDw0ACJnYRUdjWo"
type: "plan"
status: "approved"
title: "Implement Issue #290 OBS-08 allocation-probe — CountingAllocator/AllocationScope Implementation Plan"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-290-observability-allocation-probe.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-290-observability-allocation-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-290-observability-allocation-probe.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-290-observability-allocation-probe-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
  deferred-by: []
---

# Implement Issue #290 OBS-08 allocation-probe — CountingAllocator/AllocationScope Implementation Plan

## Goal

`ruviform-observability` に `allocation-probe` feature で gate された
`profiling` モジュールを実装し、`CountingAllocator<A>`（4 GlobalAlloc
API + 飽和計数）と TLS `AllocationScope`/`AllocationWindow`（ネスト・
同時・teardown 対応の計測ウィンドウ）を提供する。

## Scope

単一の実装ストリーム。RT Timing・CPU sampling 実機検証は本 plan の
対象外（spec/design の Out of Scope / Deferred に記録済み）。

## Upstream Documents

- `docs/specs/issue-290-observability-allocation-probe.md`（approved）

## Design Inputs

- `docs/designs/issue-290-observability-allocation-probe.md`（approved）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-observability/Cargo.toml` | `allocation-probe = []` feature 追加 | 依存追加なし・feature のみ |
| `crates/ruviform-observability/src/lib.rs` | `#[cfg(feature = "allocation-probe")] pub mod profiling;` + module doc bullet | additive のみ |
| `crates/ruviform-observability/src/profiling/mod.rs` | モジュール契約・RT contract 表・re-export | 新規 |
| `crates/ruviform-observability/src/profiling/stats.rs` | `OpStats`/`AllocationStats` スナップショット型 | 新規 |
| `crates/ruviform-observability/src/profiling/allocator.rs` | `CountingAllocator<A>` + global atomics 計数 + 4 API impl | 新規 |
| `crates/ruviform-observability/src/profiling/scope.rs` | `AllocationScope`/`AllocationWindow` + TLS chain + ScopeNode | 新規 |
| `docs/test-specs/issue-290-observability-allocation-probe-tests.md` | 検証意図の正本（Feature/Rule/Example） | 新規 |
| `docs/tasks/implement-issue-290-observability-allocation-probe.md` | 実装スライスの追跡 | 新規 |
| `docs/impl/ir/issue-290-observability-allocation-probe.md` | 実装記録（IR） | 実装後に作成 |

## Implementation Sequence

- [ ] 1. `Cargo.toml` に `allocation-probe = []` feature を追加し、
  `lib.rs` に cfg gate 付き `pub mod profiling;` と doc bullet を追加
- [ ] 2. `stats.rs`: `OpStats`/`AllocationStats`（derive + doc）を実装
- [ ] 3. `allocator.rs`: `GlobalStats`（atomics）・計数ヘルパー・
  `CountingAllocator<A>`（const new/inner/stats）・4 API の
  `unsafe impl GlobalAlloc` を実装
- [ ] 4. `scope.rs`: TLS `HEAD`・`ScopeNode`・`AllocationScope`
  （new/enter/stats/reset/measure/drop）・`AllocationWindow`
  （stats/drop・!Send）・unlink walk を実装
- [ ] 5. `mod.rs`: モジュール doc・re-export・RT contract 表を整備
- [ ] 6. `allocator.rs`/`scope.rs` の `#[cfg(test)]` ユニット試験を
  実装（後述 Verification Matrix の全項目）
- [ ] 7. `cargo test -p ruviform-observability --features
  allocation-probe` で試験を通し、clippy/fmt を整える
- [ ] 8. `mise run fmt-check`/`check`/`lint`/`test`（default
  feature）で非回帰を確認
- [ ] 9. `git fetch origin main && git merge origin/main` で最新 main
  を取り込み、docs index を再生成してから最終検証
- [ ] 10. IR 文書を作成し、コミットを論理単位に分けて push・PR 作成

## Dependencies and Risks

- 依存: なし（外部クレート追加なし。`ruviform-rt`/`ruviform-media-core`
  の既存依存のみ・本モジュールは直接使わない）。
- Risk: allocator 経路での panic/alloc/再帰 → const-init TLS +
  `try_with` + Cell/atomic のみで構造的に排除（design の禁止事項）。
- Risk: `mem::forget`/非 LIFO drop での dangling → node は scope の
  Box が所有し unlink を window/scope 両 drop で冪等実行、
  `active` flag で二重 link 防止（design の不変条件）。
- Risk: zero-size layout を有効入力と誤認 → `GlobalAlloc` 契約で UB
  のため試験対象外・文書化のみ（spec AC-1 改訂済み）。
- Risk: `assert_no_alloc` は実割当を検出するため、計数経路の
  no-alloc 検証には実割当しない mock inner（`ProbeAlloc`）を使う。

## Task Breakdown

- T1: feature gate + stats 型 + allocator 本体 + allocator 試験
- T2: scope/window + TLS chain + scope 試験
- T3: mod.rs 仕上げ + feature-enabled 検証 + IR + コミット/PR

T1→T2 は scope が allocator の計数呼出し先（TLS walk）を必要とする
ため順序依存。T3 は両者完了後。

## Verification Matrix

| Step | Verification command or criteria | Pass condition |
| --- | --- | --- |
| 1 | `cargo check -p ruviform-observability --features allocation-probe` | exit 0 |
| 2-5 | `cargo check -p ruviform-observability --features allocation-probe` | exit 0 |
| 6 | `cargo test -p ruviform-observability --features allocation-probe` | 全試験 pass（4 API 委譲・失敗/zero/realloc/align・ネスト/同時/teardown/非 LIFO・no-alloc・no-panic・飽和） |
| 7 | `cargo clippy -p ruviform-observability --features allocation-probe --all-targets -- -D warnings` + `cargo fmt --check` | exit 0 |
| 8 | `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` | 全て exit 0（default feature 非回帰） |
| 8b | `cargo tree -p ruviform-observability --features allocation-probe -e normal` で外部依存非増加を確認 | tracing 系等が増えない |
| 9 | `git fetch origin main && git merge origin/main` 後に再検証 | conflict なし・検証維持 |

補助証拠の位置づけ: cargo 直接実行は補助証拠（canonical = mise
タスク）。feature 有効の検証は mise タスクに存在しないため
cargo 直接実行が唯一の手段であり、タスク仕様どおり補助証拠として
記録する。

## Open Questions / Gaps

- `realloc` の in-place 成否は内側 `A` 依存で区別しない（new_size
  ベース計数）— design で確定済み。
- scope の `current_bytes` 負ドリフト（window 前確保ブロックの解放）
  は `i64` で正直に表現 — design で確定済み。
- RT Timing・CPU sampling 実機・#281 証拠は本 plan 対象外
  （spec/design の Deferred 記録どおり）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一ストリーム・ Orca worker が
  直接実装）
- Independent work streams: none
- User approval required before dispatch: no（worker contract により
  review-protocol を承認代替として使用済み）

## Review Handoff

- レビュアーは spec/design の承認済み内容と実装の一致、特に
  allocator 経路の no-alloc/no-panic 不変条件と TLS chain の
  unlink 不変条件（dangling 不可）を重点確認されたい。
- `docs/adr/realtime-contract-and-verification-policy.md` と
  `docs/designs/rt-verification-lane-design.md` が RT 契約の正本。
