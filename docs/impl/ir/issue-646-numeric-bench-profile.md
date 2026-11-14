---
id: "IMPL-034XEUw3tBbjwqbNReqULG"
type: "impl"
status: "completed"
title: "issue-646-numeric-bench-profile"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "devin-worker"
relations:
  source: []
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
    - "docs/tasks/issue-646-numeric-bench-profile.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
    adopted:
      - "docs/impl/exp/issue-646-numeric-bench-profile.jsonl"
    rejected: []
---

# issue-646-numeric-bench-profile

## Summary

`crates/ruviform-numeric` の lib test target が bench profile
（`debug_assertions` off）でコンパイル不能となる不具合を修正した。
原因は 2 点: (A) `lib.rs` の `#[global_allocator] AllocDisabler` に
`#[cfg(debug_assertions)]` が無く、`assert_no_alloc` の
`disable_release` feature が `not(debug_assertions)` で同型を
cfg 外にするため E0425、(B) `corr.rs` の `assert_eq!(<f64 式>, [])`
10 サイトが criterion 経由の `serde_json`
`PartialEq<Value> for f64` 実装と競合し E0282/E0283。
併せて `docs/development-environment.md` の bench / opt-in lane
記述を `mise.toml` 正本へ追従させた。

## Context

- spec: `docs/specs/issue-646-numeric-bench-profile.md`（approved）
- design: `docs/designs/issue-646-numeric-bench-profile.md`（approved）
- plan: `docs/plans/issue-646-numeric-bench-profile.md`（approved）
- test-spec: `docs/test-specs/issue-646-numeric-bench-profile.md`
  （approved）
- task: `docs/tasks/issue-646-numeric-bench-profile.md`
- Issue: `scarecrow173/ruviform#646`（PR `scarecrow173/ruviform#645`
  の G05 bench lane 整備中に発見された潜在不具合）

## Implementation

### Changes

- `crates/ruviform-numeric/src/lib.rs` — `#[global_allocator]
  static ALLOC: assert_no_alloc::AllocDisabler` の直上に
  `#[cfg(debug_assertions)]` を付与。dev/test profile では
  `AllocDisabler` が arm され `rt_` テストの alloc-free 強制を
  維持し、bench/release profile では宣言自体が cfg 外となる。
  ワークスペース内 22 サイトの先例（代表例:
  `crates/ruviform-math/src/dsp.rs`）と同形・同位置。
- `crates/ruviform-numeric/src/corr.rs` — 曖昧な
  `assert_eq!(<f64 式>, [])` 10 サイトを
  `assert_eq!(<式>, [] as [f64; 0])` へ型付け。
  `Vec<f64>`（`ws.a_re` 等 5 サイト・one-shot `ncc()`/
  `gcc_phat()` unwrap 2 サイト）と `&[f64]`（`NccCurve::values`・
  plan の `gcc_phat`/`cross_correlation` unwrap 3 サイト）の
  双方に `PartialEq<[f64; 0]>` impl が存在し一様に書ける。
  同ファイル L1814 既存の `Vec::<f64>::new()` 慣例サイトは
  元来曖昧でないため変更しない。
- `docs/development-environment.md` — `bench` 行の対象列挙を
  現行 5 crate（math SIMD PoC・rt RT observability PoC・
  viz LOD bench・numeric `corr` cost-model・io-e2e
  `io_composition` baseline）へ更新し、opt-in lane 一覧へ
  `check-io-e2e`/`lint-io-e2e`/`test-io-e2e`/
  `test-io-e2e-corpus`/G01 `large_staged` を追記。
  `mise.toml` は変更していない。

### Decisions During Implementation

- **`[] as [f64; 0]` を採用**: 当初 `assert!(x.is_empty())` を
  検討したが `cargo clippy -- -D warnings` で
  `clippy::assert_is_empty` が 10 件発火し棄却。clippy 自身の
  推奨形である型付き空配列比較が `Vec<f64>`/`&[f64]` 双方に
  使え、assert_eq の失敗時 diff 表示も維持される。
  rustc 単体で `Vec<f64>`・`&[f64]` 双方への適合を事前確認済み。
- **`#[cfg(debug_assertions)]` のみで gate**: `assert_no_alloc`
  の `assert_no_alloc()` 呼出し自体は `disable_release` 下で
  no-op 関数として残る設計のため、型宣言側の gate だけで
  bench profile の E0425 が解消する。`#[cfg(test)]` の外側に
  置く必要はなく、rt_tests module 内の宣言位置を維持した。
- **`mise.toml` 非変更**: `bench-smoke` 対象拡張
  （`-p ruviform-numeric -p ruviform-io-e2e` 追加）は
  `scarecrow173/ruviform#631` が同ファイルを占有中のため
  別 slice の残作業として PR 本文に明記。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-646-numeric-bench-profile.jsonl` —
  実装過程の観察記録（bench profile 再現、`disable_release`
  の cfg 動作、is_empty 形の clippy 棄却と型付き空配列への
  転換）。

### Rejected

- `assert!(x.is_empty())` — `clippy::assert_is_empty` が
  `-D warnings` で発火（実測で棄却）。
- serde_json 依存の除去 — criterion の非 optional 依存であり
  bench 計測系へ波及するため、assert 側を直す方が境界として
  正しい。

## Validation

canonical 検証（`mise run` / `mise exec`、Windows 11 x64・
rustc 1.99.1・cargo 1.99.1）:

- `mise run fmt-check` — pass
- `mise run check ruviform-numeric` — pass（`--locked
  --all-targets`）
- `mise run lint ruviform-numeric` — pass（`-D warnings`）
- `mise run test ruviform-numeric` — pass（116 tests +
  7 doctests。`rt_tests::rt_processing_paths_do_not_allocate` /
  `rt_tests::rt_spectral_paths_do_not_allocate` が dev profile で
  実際に収集・実行され `AllocDisabler` 有効を確認）
- `mise exec -- cargo bench -p ruviform-numeric --no-run` —
  pass（Issue AC・補助証拠。bench profile で lib test target と
  `corr` bench がコンパイル）
- `mise exec -- cargo bench --workspace --no-run` — pass
  （workspace 全 bench target のコンパイル、他 crate の潜在
  bench 破損なし）
- `mise exec -- prek run --all-files` — pass

証拠: `evidence/rf646/`（`bench-numeric-no-run-before.log` に
修正前の E0425/E0282/E0283、`bench-numeric-no-run-after.log`・
`bench-workspace-no-run.log`・`verification.log` に修正後の
全検証ログ）

## Risks

- bench profile では `assert_no_alloc` が no-op となるのは
  crate の `disable_release` 設計どおり。RT 検査の権威は
  dev/test profile の `rt_` lane（Issue #150 系）にあり、
  bench profile での計測用途とは目的が異なる。
- `[] as [f64; 0]` 型付けは「空であること」の検証意味を
  維持するが、serde_json 型の `PartialEq` 実装が将来変わった
  場合の再検証は lint/test lane が担保する。

## Follow-ups

- `bench-smoke` 対象への `ruviform-numeric`・`ruviform-io-e2e`
  追加は `scarecrow173/ruviform#631` 占有解消後の別 slice
  （`mise.toml` 変更を要する）。
- CI `bench-smoke` が numeric/io-e2e を含まない現状では、
  同種の bench profile 破損は `cargo bench --workspace
  --no-run` の手動実行でしか検出できない — 恒久的な検出
  lane 化は Issue #631 系の bench-smoke 拡張に委ねる。
