---
id: "TASK-034JxObKHc0v3kUhnQiePv"
type: "task"
status: "done"
title: "RT 基盤ドキュメント同期と総合検証"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-exec/src/lib.rs"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-rt-foundation.md"
  implemented-by:
    - "../impl/ir/sync-rt-foundation-docs.md"
  depends-on:
    - "../plans/implement-rt-foundation.md"
    - "implement-rt-snapshot-exchange.md"
    - "implement-exec-thread-priority.md"
  blocks: []
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
    - "../test-specs/rt-channel-and-snapshot-contracts.md"
    - "../test-specs/rt-context-and-processor-contract.md"
    - "../test-specs/thread-priority-lifecycle.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# RT 基盤ドキュメント同期と総合検証

## Work

plan `PLAN-034JxNegJktSAJZ2LuO0N8` の Phase F。rt プリミティブ実装（タスク 1〜4）と exec 優先度機構（タスク 5）の完了を受け、ドキュメント記述の同期・規約監査・ワークスペース総合検証を実施する。

- [x] `crates/ruviform-rt/src/lib.rs` crate doc の最終 wording: 責務（`Processor`/`RtProcessor` 契約・`RtContext` capability・`MasterClock`・SPSC・退役・スナップショット交換・事前確保型受け渡し構造の RT-safe プリミティブ。スケジューリング・I/O を持たない）・Allowed dependency layers（なし — `rtrb`/`arc-swap` は private 実装詳細で型非露出）・RT contract（全公開 API は `RT-suitable` または `non-RT only` の明示ラベル）・Contract source（ベースライン + RT 契約 ADR + 実装方式 ADR）を実装済み状態で確認・最終化
- [x] `crates/ruviform-exec/src/lib.rs` crate doc の最終 wording: thread_priority 責務の記載確認（「RT プリミティブは持たない」維持・Contract source 更新）
- [x] `AGENTS.md` の「math・rt・exec は引き続きスケルトン」記述を実装済み状態へ同期（rt・exec をスケルトンから外す。`test-rt` タスク行の「現時点では RT テスト未実装」注記も更新）
- [x] `README.md` の rt/exec スケルトン記述を実装済み状態へ同期（クレート一覧・ディレクトリ表）
- [x] ルート `Cargo.toml` の members コメント「Phase 1 基盤スケルトン（Issue #17）」を現状態に合わせて更新
- [x] 規約監査:
  - 全公開 enum（`Error`・`TryPushError`・`TryPopError`・`PriorityError`）に enum レベル `#[non_exhaustive]` があること（`grep -rn "pub enum" crates/ruviform-rt/src crates/ruviform-exec/src` で列挙し各直上を確認）
  - 全公開 API に `**RT contract:**` ラベルと doctest があること
  - 非 RT のテスト・doctest・ヘルパー関数名に `rt_` 部分文字列を含まないこと（`mise exec -- cargo test --workspace rt_` の選択集合を確認し、`grep -rn "fn assert_\|fn start_" crates/` で禁止パターンを grep）
  - `unsafe` が存在しないこと（`grep -rn "unsafe" crates/ruviform-rt/src crates/ruviform-exec/src`）
- [x] 外部型非露出: `cargo doc` / 公開シグネチャレビューで `rtrb`・`arc_swap`・`audio_thread_priority` の型名・エラー名・feature 名が公開 API に出ないこと
- [x] 総合検証スイート実行（Verification 節）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-rt-foundation.md](../plans/implement-rt-foundation.md) — `PLAN-034JxNegJktSAJZ2LuO0N8`（approved）。Phase F の正本
- [../specs/implement-rt-foundation.md](../specs/implement-rt-foundation.md) — `SPEC-034Jcr3dOT5Uxp4znj42II`（approved）。受け入れ基準の最終確認対象
- [../designs/rt-foundation-api-design.md](../designs/rt-foundation-api-design.md) — `DESIGN-034JgsK9khgDsPqG04Sp6i`（approved）。Verification Notes・不変条件 1–13 の確認対象

## Dependencies

- 前提: `implement-rt-snapshot-exchange.md`・`implement-exec-thread-priority.md`（全 API 面の確定後に実施）
- `mise run verify-ci` は CI 用途限定のため本タスクでは使わない（`*-ci` タスクと `target/ci/` 併存を避ける — AGENTS.md）

## Affected Files

- `crates/ruviform-rt/src/lib.rs` — crate doc 最終化
- `crates/ruviform-exec/src/lib.rs` — crate doc 最終化
- `Cargo.toml` — members コメント同期
- `README.md` — スケルトン記述同期
- `AGENTS.md` — スケルトン記述・`test-rt` 注記同期

## Done When

- [x] 両 `lib.rs` crate doc・`AGENTS.md`・`README.md`・ルート `Cargo.toml` コメントが実装済み状態と一致しスケルトン記述が残らない
- [x] 公開 enum `#[non_exhaustive]`・`**RT contract:**` ラベル・doctest・`rt_` 命名規約・`unsafe` 不在の監査が全てパス
- [x] `mise run test-rt` が意図した RT 契約テストのみを選択する（非 RT テスト・doctest が混入しない）
- [x] 検証スイート全コマンドがグリーン

## Verification

- [x] `mise run fmt-check` — 差分なし
- [x] `mise run check` — 型チェック合格
- [x] `mise run lint` — 0 warnings（`exhaustive_enums`・`unsafe_code` ゲート含む）
- [x] `mise run test` — ワークスペース全テスト合格
- [x] `mise run test-rt` — `rt_` 命名テストのみ選択・合格
- [x] `mise run bench` — 入口として成功（ベンチマーク未実装のため no-op 相当）
- [x] `mise exec -- prek run --all-files` — 全フック合格
- [x] `mise exec -- cargo metadata --format-version 1 --no-deps` — 全パッケージ `publish` が `[]`、依存リストに `rtrb`・`arc-swap`・`audio_thread_priority`（`default-features = false`）が記録される
- [x] `mise exec -- cargo test --workspace rt_` — 選択集合に非 RT テスト・doctest が混入していないことを目視確認
