---
id: "PLAN-034XEUqvt3QlpD8X3zyJV2"
type: "plan"
status: "approved"
title: "Issue #646 ruviform-numeric bench profile compile fix — implementation plan"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/646"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-646-numeric-bench-profile.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-306-io-lane-completion.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-646-numeric-bench-profile.md"
    - "docs/designs/issue-646-numeric-bench-profile.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-646-numeric-bench-profile.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/646"
    - "https://github.com/scarecrow173/ruviform/issues/645"
  defers: []
  deferred-by: []
metadata:
  issue: 646
---

# Issue #646 ruviform-numeric bench profile compile fix — implementation plan

## Goal

`ruviform-numeric` の lib test target を bench profile で
コンパイル可能にし、`cargo bench --workspace` 系入口を全 crate で
成立させる。あわせて `docs/development-environment.md` の
bench/opt-in lane 記述を `mise.toml` 正本へ追従させる。

## Scope

単一 implementation stream。コード修正は `ruviform-numeric`
1 crate の 2 ファイル + 文書 1 ファイル + 内部文書チェーン +
evidence のみで、並列分割の必要はない。

## Upstream Documents

- `docs/specs/issue-646-numeric-bench-profile.md`

## Design Inputs

- `docs/designs/issue-646-numeric-bench-profile.md`

## File Map

- `crates/ruviform-numeric/src/lib.rs` — `#[cfg(test)] mod
  rt_tests` の `#[global_allocator] AllocDisabler` へ
  `#[cfg(debug_assertions)]` 付与。
- `crates/ruviform-numeric/src/corr.rs` — 曖昧 `assert_eq!` 10
  サイト（L1326-1330・L1794-1809・L1810-1811）を
  `assert_eq!(<式>, [] as [f64; 0])` へ型付け。
- `docs/development-environment.md` — `bench` 行の対象列挙更新 +
  opt-in lane 一覧へ io-e2e lane 群追記。
- `evidence/rf646/` — 再現・修正後のコンパイルログ証跡。
- `docs/{specs,designs,plans,tasks,test-specs,impl}/issue-646-*` —
  本 slice の内部文書チェーン。

## Implementation Sequence

- [x] `cargo bench -p ruviform-numeric --no-run` で失敗を再現し
      エラー一覧を `evidence/rf646/` に記録
- [x] `lib.rs` に `#[cfg(debug_assertions)]` を付与
- [x] `corr.rs` の曖昧 assert を `is_empty()` へ置換
- [x] `cargo bench -p ruviform-numeric --no-run` → exit 0 を確認
- [x] `cargo bench --workspace --no-run` → exit 0 を確認
      （他 crate 潜在破損なし）
- [x] `docs/development-environment.md` の drift 解消
- [x] `mise run fmt-check` / `check` / `lint` / `test
      ruviform-numeric` で dev profile 非回帰を確認
- [x] 文書チェーン・evidence・docs index 再生成

## Dependencies and Risks

- `mise.toml` は `scarecrow173/ruviform#631` が占有中 — 本 slice
  では変更しない（`bench-smoke` 対象拡張は別 slice の残作業）。
- `[] as [f64; 0]` 型付けは assert 意図を維持するが、
  clippy `assert_is_empty` の制約で `assert!(is_empty())`
  形は使えない（design の Alternatives 参照）。
- bench profile では `assert_no_alloc` が no-op — crate 設計
  どおりであり、RT 検査の権威は dev/test profile の `rt_` lane。

## Task Breakdown

- `docs/tasks/issue-646-numeric-bench-profile.md`（単一 task）。

## Verification

- [x] `mise exec -- cargo bench -p ruviform-numeric --no-run` →
      exit 0（補助証拠: canonical = mise タスクだが bench 系は
      mise タスク `bench`/`bench-smoke` が実計測/スモーク入口で
      `--no-run` コンパイル検証は cargo 直接実行が必要）
- [x] `mise exec -- cargo bench --workspace --no-run` → exit 0
- [x] `mise run check ruviform-numeric` → clean
- [x] `mise run lint ruviform-numeric` → clean
- [x] `mise run test ruviform-numeric` → 116 tests Pass
- [x] `mise run fmt-check` → Pass
- [x] `mise exec -- prek run --all-files` → Pass（導入済みの場合）

## Open Questions / Gaps

- dev profile で `assert_eq!` 曖昧が顕在化しなかった正確な
  rustc/クレートグラフ上の機序は未解明（bench profile でのみ
  エラー再現を確認済み）。恒久脆弱性として修正済みであり、
  機序解明は本 slice の受入に不要と判断。
- `bench-smoke` への numeric/io-e2e 追加は `scarecrow173/ruviform#631`
  の slice に委譲（PR 本文「未対応の作業」に明記）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 task・変更量小）
- Independent work streams: none
- User approval required before dispatch: no（本 slice は
  coordinator 確定の受入条件に基づく是正タスク）
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- `scarecrow173/ruviform#631` が `mise.toml` を占有中のため
  `bench-smoke` 対象拡張は本 PR に含めない。merge 後は
  `mise run bench`（全量）が `--no-run` ではなく実計測でも
  成立する想定（`bench-smoke` 経路は #631 側 slice で拡張）。
