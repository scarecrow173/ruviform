---
id: "TASK-034XEUr2TZSWvSNX2TudTp"
type: "task"
status: "done"
title: "Issue #646 ruviform-numeric bench profile compile fix — lib.rs/corr.rs/docs 是正"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/646"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-numeric/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-numeric/src/corr.rs"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-646-numeric-bench-profile.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-646-numeric-bench-profile.md"
    - "docs/specs/issue-646-numeric-bench-profile.md"
    - "docs/designs/issue-646-numeric-bench-profile.md"
    - "docs/test-specs/issue-646-numeric-bench-profile.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-646-numeric-bench-profile.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-646-numeric-bench-profile.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #646 ruviform-numeric bench profile compile fix — lib.rs/corr.rs/docs 是正

## Work

`crates/ruviform-numeric` の lib test target が bench profile
（debug_assertions off）でコンパイル不能となる既存不具合を修正
する。`lib.rs` の `AllocDisabler` へ `#[cfg(debug_assertions)]`
を付与し、`corr.rs` の曖昧 `assert_eq!(<f64 式>, [])` 10 サイトを
`[] as [f64; 0]` 型付けへ置き換える。併せて
`docs/development-environment.md` の bench lane / opt-in lane
記述を `mise.toml` 正本へ追従させる。`mise.toml` 自体は
`scarecrow173/ruviform#631` 占有中のため変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-646-numeric-bench-profile.md`（approved）
- `docs/designs/issue-646-numeric-bench-profile.md`（approved）
- `docs/plans/issue-646-numeric-bench-profile.md`（approved）
- `docs/test-specs/issue-646-numeric-bench-profile.md`（approved）
- `POLICIES.md`

## Dependencies

- `assert_no_alloc = "1.1"` の `disable_release` feature 設計
  （`not(debug_assertions)` で `AllocDisabler` 型を cfg 外にする）
- criterion 0.8.2 dev-dep グラフ経由の `serde_json`
  （`PartialEq<Value> for f64` 無条件実装が型推論を曖昧化）
- ワークスペース内 `#[cfg(debug_assertions)]` 先例 22 サイト
  （代表例: `crates/ruviform-math/src/dsp.rs`）

## Affected Files

- `crates/ruviform-numeric/src/lib.rs` — `#[cfg(debug_assertions)]`
  付与のみ
- `crates/ruviform-numeric/src/corr.rs` — assert 書き換え 10 サイト
- `docs/development-environment.md` — bench / opt-in lane 記述
- `docs/{specs,designs,plans,tasks,test-specs,impl}/
  issue-646-numeric-bench-profile.*` — 内部 doc chain
- `evidence/rf646/` — 再現・検証ログ
- `docs/*/README.md` — doc-maintenance index 再生成（生成物）

## Done When

- [x] `mise exec -- cargo bench -p ruviform-numeric --no-run` が
  exit 0（bench profile で lib test target がコンパイルできる）
- [x] `mise exec -- cargo bench --workspace --no-run` が exit 0
  （他 crate の潜在 bench 破損が無い）
- [x] `mise run test ruviform-numeric` で 116 tests 維持
  （dev profile で `rt_` 強制の `AllocDisabler` が有効なまま）
- [x] `mise run check ruviform-numeric` / `lint ruviform-numeric`
  が clean
- [x] `docs/development-environment.md` の bench lane 一覧と
  opt-in lane 一覧が `mise.toml` 正本と一致

## Verification

- [x] `mise run fmt-check` — Pass
- [x] `mise run check ruviform-numeric` — Pass（`--locked
  --all-targets`）
- [x] `mise run lint ruviform-numeric` — Pass（`-D warnings`）
- [x] `mise run test ruviform-numeric` — Pass（116 tests）
- [x] `mise exec -- cargo bench -p ruviform-numeric --no-run` —
  Pass（Issue AC・補助証拠）
- [x] `mise exec -- cargo bench --workspace --no-run` — Pass
- [x] `mise exec -- prek run --all-files` — Pass
- [ ] `*-msrv`・`*-full`・3 OS 横断 — 本 slice の明示対象外
