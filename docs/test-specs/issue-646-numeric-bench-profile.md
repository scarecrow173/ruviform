---
id: "TSPEC-034XEUpXxSbzU4hyVtyUFm"
type: "test-spec"
status: "approved"
title: "Issue #646 ruviform-numeric bench profile compile テスト仕様"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-646-numeric-bench-profile.md"
    - "docs/designs/issue-646-numeric-bench-profile.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/646"
  defers: []
  deferred-by: []
metadata:
  issue: 646
---

# Issue #646 ruviform-numeric bench profile compile テスト仕様

## Purpose

`ruviform-numeric` の lib test target が bench profile で
コンパイルできること、および dev/test profile で RT alloc 検査
（`rt_` 強制）が維持されることを保証する。本 spec が守る意図は
「bench profile = 計測用途・dev profile = RT 契約検証用途」という
profile 別責務の分離であり、bench profile で `assert_no_alloc`
が no-op 化するのは `assert_no_alloc` crate の `disable_release`
設計どおりの期待動作。RT 契約の権威は dev/test profile の
`rt_` lane（Issue #150 系）にあり、本 spec はその lane が
退行しないことを pin する。

## Feature

ruviform-numeric の profile 別コンパイル契約 — lib test target は
全 profile（dev/test/bench）でコンパイル可能であり、alloc 検査は
debug assertions が有効な profile でのみ arm される。

## Rules

- **R1**: `#[global_allocator] static ALLOC:
  assert_no_alloc::AllocDisabler` は `#[cfg(debug_assertions)]`
  でゲートされ、`not(debug_assertions)`（bench/release）では
  存在しない。ワークスペース内の他 `AllocDisabler` サイトと
  同一のゲート形に揃える。
- **R2**: `assert_no_alloc()` 呼出しを含む `rt_` テスト関数は
  profile に依存せずコンパイルできる（bench profile では
  crate 側 no-op として残る）。
- **R3**: `assert_eq!(<f64 を含む式>, [])` の形は使わない。
  空判定は右辺を `[] as [f64; 0]` と型付けして書き、serde_json
  の `PartialEq<Value> for f64` 実装が dev-dep グラフに存在
  しても型推論が曖昧にならない。`assert!(x.is_empty())` は
  clippy `assert_is_empty`（`-D warnings`）に抵触するため
  使わない。
- **R4**: テストの意図・網羅性は変更しない — assert の書き換えは
  「空であること」の検証を維持したまま構文のみを変える。
- **R5**: dev/test profile では `AllocDisabler` が arm され、
  `rt_processing_paths_do_not_allocate` /
  `rt_spectral_paths_do_not_allocate` が alloc を検知できる
  状態を維持する。

## Examples

- **E1（bench profile コンパイル）**: Given `cargo bench
  -p ruviform-numeric --no-run`、When bench profile で lib test
  target をコンパイル、Then E0425/E0282/E0283 が発生せず exit 0。
- **E2（workspace 非回帰）**: Given `cargo bench --workspace
  --no-run`、When workspace 全 lib test target を bench profile
  でコンパイル、Then exit 0（他 crate の潜在破損が無い）。
- **E3（RT lane 維持）**: Given `mise run test ruviform-numeric`
  （dev/test profile）、When lib test を実行、Then 116 tests が
  Pass し `rt_` テストが `AllocDisabler` 下で alloc を検知
  できる状態を維持する。
- **E4（doc 一致）**: Given `mise.toml` の task 定義、When
  `docs/development-environment.md` の bench/opt-in 記述と照合、
  Then 記述が正本と一致する（`mise tasks` で実在確認）。

## Guarantees

- bench profile での lib test target コンパイルが恒久的に
  成立する（`AllocDisabler` 不在・`assert_eq!` 曖昧の両系統を
  解消）。
- dev/test profile の RT alloc 検査が弱化しない。
- `docs/development-environment.md` が `mise.toml` 正本と
  drift しない記述へ更新される。

## Non-goals

- bench の実計測値・性能回帰判定。
- `bench-smoke` 対象拡張（`scarecrow173/ruviform#631` の別 slice）。
- serde_json / criterion の依存構成変更。
- 他 crate のテストコードにある同型 `assert_eq!(x, [])` サイトの
  一括修正（本 slice は ruviform-numeric のみ。workspace bench
  --no-run の結果で他 crate 破損が無いことは確認する）。

## Risk

- `#[cfg(debug_assertions)]` を誤った位置へ置くと dev profile
  で `AllocDisabler` が arm されず RT 検査が無言で無効化される
  — 位置は先例（`#[global_allocator]` 直上）に厳密に揃え、
  `mise run test ruviform-numeric` の 116 tests 維持で検証する。
- `is_empty()` 置換で assert の失敗メッセージが変わるが、検証
  意味は同一でありテスト網羅性への影響はない。
