---
id: "TASK-034Jf1Cl3xf8K9wcSuvFYK"
type: "task"
status: "todo"
title: "ベンチハーネス骨格と dev-dependencies"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-math/benches/simd_poc.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-math/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "../plans/dsp-simd-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/dsp-simd-poc.md"
  blocks:
    - "implement-simd-poc-candidate-backends.md"
    - "implement-criterion-benchmarks.md"
  supersedes: []
  superseded-by: []
  related:
    - "../specs/dsp-simd-poc-adr.md"
    - "../designs/dsp-simd-poc.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/bench-tasks-and-ci-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# ベンチハーネス骨格と dev-dependencies

## Work

plan `PLAN-034JdOWI3vblJW11kIueY0`（[../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md)）の Phase A / Task T1。`crates/ruviform-math` に PoC 用 dev-dependencies と criterion ベンチハーネスの骨格を導入し、MSRV 検証手順と CI スモーク分離を確立する。

- [ ] A-1. dev-dep 導入前のコンパイルコスト baseline を記録: `mise run check`（`cargo check --workspace --all-targets`）の所要時間を計測し、Phase G（[write-simd-adoption-adr.md](write-simd-adoption-adr.md)）のコンパイルコスト前後比較の基準値として記録する（dev-dep 導入後では baseline を取れないため本タスクの先頭で実施）
- [ ] A-2. `crates/ruviform-math/Cargo.toml` に `[dev-dependencies]`（`criterion = { version = "0.8.2", default-features = false }`、`fearless_simd = { version = "1.1", features = ["force_support_fallback"] }`、`fearless_simd_macros = "0.1"`、`pulp = { version = "0.22", features = ["x86-v4", "macro"] }`、`multiversion = "0.9"`、`approx = "0.5"`）と `[[bench]] name = "simd_poc", harness = false` を追加（`[dependencies]` は空のまま）。バージョン方針は caret 指定とし、追跡済み `Cargo.lock` を再現性の錨とする（`=x.y.z` exact pin は採用しない）。各 dev-dep の追加理由・feature 選定根拠は Cargo.toml のインラインコメント（設計書 dev-dependencies 節のコメントを転記）と PR 説明文に記録する
- [ ] A-3. `benches/simd_poc.rs` に最小 criterion 骨格（`criterion_group!` / `criterion_main!` + `std::hint::black_box` を使う placeholder ベンチ 1 本）を作成
- [ ] A-4. `mise run check` で dev-dep 解決・`Cargo.lock` 再生成・全ターゲット型チェックを確認
- [ ] A-5. `mise exec -- rustup toolchain install 1.85` で 1.85 ツールチェーンを調達し、`mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーンであることを確認（MSRV 検証手順の確立。`cargo +1.85` は rustup proxy 前提のため `rustup run` 経由の呼出形に統一）
- [ ] A-6. `mise.toml` に `bench-smoke` タスク（`cargo bench -p ruviform-math -- --test`）を追加し、`.github/workflows/ci.yml` の `bench` ジョブを `mise run bench-smoke` へ変更（CI スモーク分離方針）

コミット分割（plan Task Breakdown どおり 2 コミット）: ①dev-dep + harness 骨格（A-1〜A-4・A-5 の手順確立）、②`bench-smoke` タスク + ci.yml スモーク分離（A-6）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md) — `PLAN-034JdOWI3vblJW11kIueY0`（approved）。Phase A の正本
- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`。成果物・受け入れ基準の正本
- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（approved）。dev-dep 選定・配置・MSRV 検証手順の根拠
- [../test-specs/bench-tasks-and-ci-smoke.md](../test-specs/bench-tasks-and-ci-smoke.md) — `TSPEC-034Jeo1j5l7fBhYc8iYsTN`。`bench-smoke` タスクと CI スモーク分離の検証意図の正本

## Dependencies

- 前提タスク: なし（依存グラフのルート。plan `depends-on` の dev-foundation-bootstrap が提供する mise 検証タスク・ワークスペース構成を利用）
- 後続をブロック: [implement-simd-poc-candidate-backends.md](implement-simd-poc-candidate-backends.md)（dev-dep 解決が前提）、[implement-criterion-benchmarks.md](implement-criterion-benchmarks.md)（ベンチハーネスが前提）
- 外部前提: crates.io 上の候補クレート群・rustup 経由の 1.85 ツールチェーン調達（plan Risk 3）
- [implement-simd-poc-scalar-kernels.md](implement-simd-poc-scalar-kernels.md)（T2）とは並列実施可能（B は pure std で dev-dep 非依存）

## Affected Files

- `crates/ruviform-math/Cargo.toml` — `[dev-dependencies]` と `[[bench]] harness = false` 追加（`[dependencies]` は空のまま）
- `crates/ruviform-math/benches/simd_poc.rs` — 新規（最小 criterion 骨格）
- `Cargo.lock` — dev-dep 追加に伴う再生成（mise pin の 1.99.0 上で再生成してから `rustup run 1.85 ... --locked` 検証に入る）
- `mise.toml` — `bench-smoke` タスク追加
- `.github/workflows/ci.yml` — `bench` ジョブを `mise run bench-smoke` へ変更

## Done When

- [ ] `crates/ruviform-math/Cargo.toml` に dev-deps 6 本と `[[bench]] name = "simd_poc", harness = false` が宣言され、`[dependencies]` が空のまま維持されている
- [ ] `Cargo.lock` が再生成済みで、baseline 計測値（A-1）の記録がある
- [ ] `mise.toml` に `bench-smoke` タスクが存在し、ci.yml の `bench` ジョブが `mise run bench-smoke` を呼ぶ
- [ ] MSRV 検証手順（`rustup run 1.85 ... --lib --locked`）が確立・記録されている

## Verification

- [ ] `mise run check` がグリーン（dev-dep 解決・全ターゲット型チェック）
- [ ] `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーン
- [ ] `mise run bench-smoke` がスモーク完走（placeholder ベンチが `--test` モードで実行される）
