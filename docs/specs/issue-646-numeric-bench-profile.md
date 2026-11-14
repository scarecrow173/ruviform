---
id: "SPEC-034XEUi9KFfbJmIcNkynUU"
type: "spec"
status: "approved"
title: "Issue #646 ruviform-numeric lib test bench profile compile fix"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-306-io-lane-completion.md"
    - "docs/impl/ir/issue-265-correlationplan-prefers-direct-mixed-radix-direct.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-150-f64-offline-numeric.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/646"
    - "https://github.com/scarecrow173/ruviform/issues/645"
    - "https://github.com/scarecrow173/ruviform/issues/265"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/150"
  defers: []
  deferred-by: []
metadata:
  issue: 646
---

# Issue #646 ruviform-numeric lib test bench profile compile fix

## What Should Be Built

`ruviform-numeric` の lib test target が bench profile
（`debug_assertions` off）でコンパイル不能な既存不具合を修正し、
`cargo bench --workspace` 系の入口が workspace 全体で成立する
状態にする。あわせて `docs/development-environment.md` の bench /
opt-in lane 記述を `mise.toml` 正本へ追従させる drift を解消する。

具体的な修正は 2 点 + 文書 1 点:

1. `crates/ruviform-numeric/src/lib.rs` — `#[cfg(test)]` 内の
   `#[global_allocator] static ALLOC: assert_no_alloc::AllocDisabler`
   に `#[cfg(debug_assertions)]` を付与する。
   `assert_no_alloc@1.1` の default feature `disable_release` は
   `not(debug_assertions)` で `AllocDisabler` 型自体を cfg 外に
   するため、bench profile では E0425 となる。ワークスペース内の
   他 22 サイト（代表例: `crates/ruviform-math/src/dsp.rs`）は全て
   同ゲート済みで、この 1 箇所だけが外れ値。
2. `crates/ruviform-numeric/src/corr.rs` — `assert_eq!(<f64 式>, [])`
   の曖昧サイト 10 箇所を `assert_eq!(<式>, [] as [f64; 0])` へ
   置き換える。criterion 0.8.2 経由の `serde_json` が
   `PartialEq<Value> for f64` を無条件実装するため、
   `Vec<f64>`/`&[f64]` と空配列リテラル `[]` の比較で要素型が
   一意に推論できず E0282/E0283 となる
   （rust#89254 / serde-json#958 と同型）。右辺を
   `[f64; 0]` へ型付けすれば `f64: PartialEq<f64>` のみが
   要求され曖昧性が消え、`Vec<f64>`・`&[f64]` の双方に
   `PartialEq` impl が存在するため一様に使える
   （clippy `assert_is_empty` の推奨形でもある）。
3. `docs/development-environment.md` — `bench` 行の対象記述へ
   `ruviform-viz`（Issue #157）・`ruviform-numeric` の `corr`
   （Issue #265）・`ruviform-io-e2e` の `io_composition`
   （Issue #306 G05）を追記し、opt-in lane 一覧へ io-e2e lane 群
   （`check-io-e2e`/`lint-io-e2e`/`test-io-e2e`/`test-io-e2e-corpus`/
   G01 `large_staged`）を追記する。

## Why Now

`scarecrow173/ruviform#306`（PR `scarecrow173/ruviform#645`）の
Worker が G05 bench lane 整備中に `cargo bench --workspace` 系の
実走で発見した既存の潜在不具合。CI `bench-smoke` は
`ruviform-math`/`ruviform-rt`/`ruviform-viz` のみを対象とし、
`--bench corr` フィルタでも lib test target を通らないため、
Issue #265 で `corr` bench が導入されて以降この破損は検出経路を
持たなかった。`mise run bench`（全量計測入口）が現状 Fail する
状態を解消する。

## Users and Value

- Ruviform の開発者: `mise run bench` / `cargo bench --workspace`
  が実行可能になり、`ruviform-numeric` の `corr` ベンチ
  （Issue #265 コストモデル校正）と `ruviform-io-e2e` の
  `io_composition` ベンチ（Issue #306 G05 性能 baseline）が
  計測できる。
- CI/レビュー: bench profile での lib test target コンパイルが
  検証され、将来の bench 追加時に同型の潜在破損を踏みにくくなる
  （`#[cfg(debug_assertions)]` ゲートの先例への統一）。

## Scope

### In Scope

- `crates/ruviform-numeric/src/lib.rs` の `AllocDisabler` への
  `#[cfg(debug_assertions)]` 付与（既存 22 サイトと同形）。
- `crates/ruviform-numeric/src/corr.rs` の曖昧 `assert_eq!` サイトの
  型付け書き換え（assert の意図・網羅性は不変）。
- `docs/development-environment.md` の bench lane / opt-in lane
  記述の drift 解消（`mise.toml` 正本との一致）。
- 検証証跡（`evidence/rf646/`）と内部文書チェーン。

### Out of Scope

- `mise.toml` の一切の変更 — `bench-smoke` 対象拡張
  （`-p ruviform-numeric -p ruviform-io-e2e` 追加）は
  `scarecrow173/ruviform#631` が同ファイルを占有中のため別 slice。
- 他 crate への bench 追加、性能回帰の判定基準。
- `AGENTS.md`、`POLICIES.md`、`.github/workflows/`、`*-msrv` 系
  lane、3OS 横断検証。
- テストの削除・skip・assert 意図の変更。

## Requirements

- bench profile（`debug_assertions` off）で `ruviform-numeric` の
  lib test target がコンパイルできること。
- dev/test profile で `rt_` 強制（`AllocDisabler` 有効）と
  既存テスト数（116 tests）が維持されること。
- `assert_eq!(<f64 式>, [])` 型の曖昧サイトを恒久的に解消し、
  serde_json が dev-dep グラフに存在しても bench profile で
  コンパイルできること。
- `docs/development-environment.md` の bench / opt-in 記述が
  `mise.toml` のタスク定義と一致すること。

## Acceptance Criteria

- [x] `mise exec -- cargo bench -p ruviform-numeric --no-run` が exit 0
- [x] `mise exec -- cargo bench --workspace --no-run` が exit 0
      （他 crate の潜在 bench 破損も無いこと）
- [ ] `mise run test ruviform-numeric` で 116 tests 維持
      （dev profile で `rt_` 強制が残ること）
- [ ] `mise run check ruviform-numeric` / `lint ruviform-numeric` clean
- [x] `docs/development-environment.md` の bench lane 一覧 drift を
      解消（viz・numeric(#265)・io-e2e(io_composition/#306 G05)・
      opt-in io-e2e lane 群の記載追従）
- [ ] `mise run fmt-check` Pass

## Deferred Design Concerns

- `bench-smoke` の対象拡張（`ruviform-numeric`・`ruviform-io-e2e`
  の追加）— `scarecrow173/ruviform#631` が `mise.toml` を占有中の
  ため本 slice では扱わない。#631 側の slice で対応される想定。

## Sources and Relations

- Issue: <https://github.com/scarecrow173/ruviform/issues/646>
- 発見経緯: PR `scarecrow173/ruviform#645`（`scarecrow173/ruviform#306`
  G05 bench lane 整備）本文にエラー一覧が記録済み。
- 起点: `scarecrow173/ruviform#265` の corr bench 導入
  （`docs/impl/ir/issue-265-correlationplan-prefers-direct-mixed-radix-direct.md`）。
- RT lane 関連: `scarecrow173/ruviform#150`（numeric RT lane、
  `rt_` 強制維持の確認先）。
- bench lane 記録: `docs/specs/issue-306-io-lane-completion.md`。
