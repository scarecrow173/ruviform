---
id: "DESIGN-034XEUiEWDih8GYy2D0ytt"
type: "design"
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
  implements:
    - "docs/specs/issue-646-numeric-bench-profile.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-646-numeric-bench-profile.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-306-io-lane-completion.md"
    - "docs/impl/ir/issue-265-correlationplan-prefers-direct-mixed-radix-direct.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-646-numeric-bench-profile.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/646"
    - "https://github.com/scarecrow173/ruviform/issues/645"
  defers: []
  deferred-by: []
metadata:
  issue: 646
---

# Issue #646 ruviform-numeric lib test bench profile compile fix

## Context

`cargo bench -p ruviform-numeric --no-run`（bench profile =
release 継承・`debug_assertions` off）で lib test target が
14 件のエラーでコンパイル不能。dev/test profile では同コードが
通るため、profile 依存の 2 系統の原因が重なっている。

### 原因 A — `AllocDisabler` の cfg ゲート欠落

`crates/ruviform-numeric/src/lib.rs` の `#[cfg(test)] mod
rt_tests` が `#[global_allocator] static ALLOC:
assert_no_alloc::AllocDisabler` を無条件で宣言している。
`assert_no_alloc@1.1` は default feature `disable_release` を持ち、
`not(debug_assertions)`（= bench/release profile）では
`AllocDisabler` 型自体が `#[cfg]` で除去される
（同 crate の `assert_no_alloc()` 関数は no-op 化して残る）。
結果、bench profile で E0425（`cannot find type/value
AllocDisabler`）×2。

ワークスペース内の `#[global_allocator] AllocDisabler` サイトは
22 箇所あり、全て `#[cfg(debug_assertions)]` ゲート済み
（代表例: `crates/ruviform-math/src/dsp.rs` の tests module）。
`ruviform-numeric` のこの 1 箇所だけが外れ値で、Issue #150 の
RT lane 導入時から潜在していた。

### 原因 B — `assert_eq!(<f64 式>, [])` の型推論曖昧

`crates/ruviform-numeric/src/corr.rs` のテストに
`assert_eq!(ws.a_re, [])` 等 10 箇所がある。右辺 `[]` は
`[?T; 0]` で、`Vec<f64>: PartialEq<[?T; 0]>` の成立には
`f64: PartialEq<?T>` が必要。dev-dep の criterion 0.8.2 が
引く `serde_json` が `impl PartialEq<Value> for f64` を
無条件実装するため、`?T` が `f64` と `serde_json::Value` の
2 候補となり E0282/E0283（rust#89254 / serde-json#958 と同型）。

この曖昧性は profile 非依存に見えるが、bench profile でのみ
顕在化した — dev profile での `cargo test` コンパイルは成功
しており（`mise run test ruviform-numeric` で 116 tests 維持）、
serde_json の impl 参加は bench profile 側のユニット構成で
のみ問題となる。恒久的な脆弱性として両 profile で安全な形へ
揃える。

## Scope

### In Scope

- `lib.rs`: `#[global_allocator]` へ `#[cfg(debug_assertions)]`
  を付与（dsp.rs 等の先例と同形・同位置）。
- `corr.rs`: 曖昧 10 サイトを `assert_eq!(x, [] as [f64; 0])`
  へ置換（`Vec<f64>`・`&[f64]` 双方に `PartialEq<[f64; 0]>`
  impl が存在し一様に書ける。assert の意図「空であること」は
  不変で、assert_eq の失敗時 diff 表示も維持される）。
- `docs/development-environment.md`: `bench` 行の対象列挙を
  現行 5 crate（math/rt/viz/numeric/io-e2e）へ更新し、opt-in
  lane 一覧へ io-e2e lane 群を追記。

### Out of Scope

- `mise.toml`（`bench-smoke` 対象は `scarecrow173/ruviform#631`
  が占有中の別 slice）。
- perf 判定基準・他 crate bench・msrv lane・3OS 横断。

## Design

### 決定 1: `#[cfg(debug_assertions)]` を `#[global_allocator]`
の直上に置く

```rust
#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
```

- bench/release profile では static 自体が消え、`assert_no_alloc()`
  呼出しは crate 側で no-op 関数として残るためテストコードは
  そのままコンパイルできる。
- dev/test profile では従来どおり `AllocDisabler` が global
  allocator として arm され、`rt_` テストの alloc 検知が維持
  される。プロファイルで「検査が効く/効かない」が切り替わるのは
  `assert_no_alloc` crate の設計どおりであり、22 サイトの先例と
  同一の意味論。

### 決定 2: `[] as [f64; 0]` への型付け

対象は `Vec<f64>`（`ws.a_re` 等・`ncc()`/`gcc_phat()` の戻り値）
と `&[f64]`（`NccCurve::values`・plan の `gcc_phat`/
`cross_correlation` 戻り値）が混在する。右辺 `[]` を
`[f64; 0]` へ型付けすれば `f64: PartialEq<f64>` のみが要求
され、serde_json の `PartialEq<Value> for f64` と競合しない。
`Vec<f64>`・`&[f64]` の双方に `PartialEq<[f64; 0]>` impl が
存在するため全サイトで同一形が使え、clippy
`assert_is_empty`（`assert!(x.is_empty())` を assert_eq へ
誘導する lint）の推奨形とも一致する。

当初 `assert!(x.is_empty())` を検討したが `cargo clippy
-- -D warnings` で `assert_is_empty` が 10 件発火し棄却。
同ファイル L1814 既存の `Vec::<f64>::new()` 慣例サイトは
曖昧でないため変更しない（最小の整合的変更）。

### 決定 3: development-environment.md の追記位置

- `bench` 行（mise タスク表）: 「現在は math と rt のみ」の
  記述が 3 crate 分 drift していたため、現行 5 crate を
  bench 名・Issue 番号付きで列挙。
- opt-in lane 一覧（`## mise タスク体系` 末尾の catch-all
  bullet の次）: io-e2e lane 群は CI 専用 job を持たない
  local/手動 lane（`test-io-msrv` のみ MSRV lane で CI 対象）
  であることを明記し、G08 corpus / G01 large_staged の
  `#[ignore]` opt-in 性も記す。

## Alternatives Considered

- **`assert!(x.is_empty())`**: `clippy::assert_is_empty` が
  `-D warnings` で発火し lint 不通（実測で確認・棄却）。
- **`Vec::<f64>::new()` 明示**: `&[f64]` サイトには使えず、
  サイトごとに型が違うため一様性に欠ける。`[] as [f64; 0]` は
  両型で使える。
- **`assert_eq!(x, Vec::<f64>::new())` / `&[] as &[f64]`**:
  型は明示できるがサイトごとに右辺の型が分かれ、冗長。
- **serde_json 依存の除去（criterion feature 調整）**:
  serde_json は criterion の非 optional 依存であり、dev-dep
  構成の変更は bench 計測系へ波及する。assert 側を直す方が
  境界として正しい。
- **`#[cfg_attr]` や feature による分岐**: 不要な複雑さ。
  先例どおり `#[cfg(debug_assertions)]` のみ。

## Risks

- `[] as [f64; 0]` への型付けは assert_eq の比較対象を
  「型付き空配列」とするのみで、検証意味（空であること）は
  元の `[]` と同一。失敗時の diff 表示も assert_eq のまま
  維持される。
- bench profile で `assert_no_alloc` が no-op になるのは crate
  の `disable_release` 設計どおり。RT 検査の権威は dev/test
  profile の `rt_` lane（Issue #150 系）にあり、bench profile
  での計測用途とは目的が異なる。

## Verification

- `mise exec -- cargo bench -p ruviform-numeric --no-run` → exit 0
- `mise exec -- cargo bench --workspace --no-run` → exit 0
- `mise run check/lint/test ruviform-numeric` → clean / 116 tests
- `mise run fmt-check` → Pass
- 証跡: `evidence/rf646/`
