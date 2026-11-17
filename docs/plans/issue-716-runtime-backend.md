---
id: "PLAN-DEnlx1Y8rOJvquR4lNGQFt"
type: "plan"
status: "approved"
title: "Issue #716 runtime ScriptBackend — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/tasks/issue-716-runtime-backend.md"
  depends-on:
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-716-runtime-backend.md"
    - "docs/impl/ir/issue-716-runtime-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-716-runtime-backend.md"
  derived-by:
    - "docs/tasks/issue-716-runtime-backend.md"
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
  defers: []
  deferred-by: []
---

# Issue #716 runtime ScriptBackend — 実装計画

## Overview

`crates/ruviform-script-luau` に runtime backend を追加する
5 step 計画。docs chain（discovery/spec/design/plan/tasks/
test-specs/adr）→ 実装（runtime.rs + lib.rs + tests）→
検証（mise check/lint/test + fmt-check + prek）→ evidence +
impl-ir/exp → review → PR の順。

## Steps

### Step 1 — docs chain 整備（本計画の前提文書群）

- `docs/{discovery,specs,designs,plans,tasks,test-specs,
  adr,impl}/issue-716-runtime-backend*.md` を作成
- 完了条件: 全文書が front matter + 内容を持ち、命名が
  `issue-716-runtime-backend` 厳密一致。

### Step 2 — runtime backend 実装

- `src/runtime.rs` 新設: `LuauBackend`/`LuauSession`/lift/
  budget/cancel/artifact（設計書どおり）
- `src/lib.rs`: `mod runtime;` + `pub use runtime::LuauBackend`
  + crate docs 更新（非目標記述の除去・runtime 追加の記述）
- `README.md`: runtime backend 対応の記述更新
- `Cargo.toml`: mlua コメントの「runtime は別 slice」記述を
  実態に更新（依存自体は変更なし）

### Step 3 — deterministic OS-L1 test 追加

- `tests/runtime.rs` 新設（test-specs の行列どおり）
- 必要に応じ `tests/registry.rs` へ ScriptBackendRegistry
  resolve→eval を追加（runtime.rs 内に収める方針 — registry.rs
  は analyzer 専用のため）

### Step 4 — 検証

- `mise run check ruviform-script-luau`
- `mise run lint ruviform-script-luau`
- `mise run test ruviform-script-luau`
- `mise run test ruviform-script`（非回帰 — 直接依存先）
- `mise run fmt-check`
- `mise exec -- prek run --all-files`
- 全て exit code を記録。失敗は修正して再検証。

### Step 5 — evidence・impl 文書・レビュー・PR

- `evidence/rf716/verification.md`（+ 必要に応じ
  `runtime-backend-verification.md`）— consumer gate 委譲明記
- `docs/impl/ir|exp/issue-716-runtime-backend*.md` + `-exp.jsonl`
- review-protocol レビュー（blocker ゼロまで）
- `git merge origin/main`（最終検証前）→ Conventional Commits
  → push → `gh pr create`（日本語本文・検証表・NotRun/Unsupported
  明記・docs index 未再生成の明記）

## Risks

- **mlua API 差異**: interim（synvelis）と同一 mlua 0.12.2
  feature 構成のため API 面は実証済み。`set_memory_limit`/
  `set_interrupt`/`ChunkMode` の signature を実装時に
  `cargo doc`/ソースで再確認する。
- **`cast_possible_truncation` lint**: pedantic で warn→deny。
  `f as i64` は範囲保証済みだが lint は構文的に発火する —
  `#[allow]` ではなく `i64::try_from` は f64 非対応のため、
  範囲チェック後の `as` に局所 `#[expect]` 相当の扱いを
  検討（interim は `#[allow]`+理由コメント — 同方針で対処、
  lint 隠蔽ではなく根拠明記）。
- **interrupt 飲み込み**: pcall 内 interrupt は hit flag で
  post-eval 検査（設計済み）。
- **`cargo` 直接実行は補助証拠のみ** — canonical は mise。
