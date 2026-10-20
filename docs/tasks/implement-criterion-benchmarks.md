---
id: "TASK-034Jf48NGXlyYcluiTnpOR"
type: "task"
status: "todo"
title: "criterion ベンチ実装"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-math/benches/simd_poc.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/dsp-simd-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/dsp-simd-poc.md"
    - "add-bench-harness-and-dev-deps.md"
    - "implement-simd-poc-candidate-backends.md"
  blocks:
    - "run-simd-bench-and-collect-results.md"
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

# criterion ベンチ実装

## Work

plan `PLAN-034JdOWI3vblJW11kIueY0`（[../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md)）の Phase E / Task T5。T1 で骨格を置いた `benches/simd_poc.rs` に criterion ベンチ本体を実装する（`BenchmarkId` 規約・状態持ちカーネルの `iter_batched`・時間予算設定・環境情報出力）。

- [ ] E-1. `benches/simd_poc.rs` を実装: `#[allow(dead_code)] #[path = "../tests/simd_poc/mod.rs"] mod simd_poc_support;` で共有 mod を include し、カーネルごと `BenchmarkGroup` + `group.throughput(Throughput::Elements(len as u64))` + `BenchmarkId::new(<impl>, "{input}/{len}")` を構成
- [ ] E-2. ベンチ matrix を実装（サイズ `64/256/1024/4096` + 端数 `100/1000/1023`、入力 `normal` + `denormal` — `add`/`scale`/`clamp`/`fade` は `normal` のみ、`biquad` は `channels = 8` 固定で frames を同系列、計測識別子は設計書 methodology 節の一覧どおり `fearless_sse2`/`pulp_v2` を除外）
- [ ] E-3. `biquad` / `sine` は `iter_batched` / `iter_with_setup` で反復ごとに状態を初期化（normal 側は非 denormal の決定的初期状態、denormal-cliff 観測側は denormal 状態を維持・再注入する setup）
- [ ] E-4. PoC 用の `BenchmarkGroup` 時間設定（`warm_up_time` / `measurement_time` / `sample_size` 削減 — 全量 ≈400–560 ケースを criterion 既定の 1h 超から ≈20–30 分以内へ短縮する目安）と quick-run filter 規約（`mise exec -- cargo bench -p ruviform-math -- <filter>`）を実装。run 冒頭に OS・CPU 型名・コア数・rustc 版・criterion 版・`Level::new()` / `pulp::Arch::new()` 検出結果を標準出力する
- [ ] E-5. `mise run bench-smoke`（`cargo bench -p ruviform-math -- --test`、スモーク）と `mise exec -- cargo bench -p ruviform-math -- dot`（quick-run 例）が実行できることを確認

コミット粒度は plan 規約どおり 1 タスク 1 コミット。共有 mod 完成後は [add-simd-equivalence-matrix-tests.md](add-simd-equivalence-matrix-tests.md)（T4）と並行実施可能。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md) — `PLAN-034JdOWI3vblJW11kIueY0`（approved）。Phase E の正本
- [../test-specs/bench-tasks-and-ci-smoke.md](../test-specs/bench-tasks-and-ci-smoke.md) — `TSPEC-034Jeo1j5l7fBhYc8iYsTN`。ベンチ・検証タスクと CI スモーク挙動の契約保証の正本
- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`。ベンチハーネス存在・`mise run bench` 計測可能の受け入れ基準
- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（approved）。ベンチ methodology（matrix・識別子・時間予算）の正本

## Dependencies

- 前提タスク: [add-bench-harness-and-dev-deps.md](add-bench-harness-and-dev-deps.md)（T1 — `[[bench]]` 宣言・criterion 導入が前提）、[implement-simd-poc-candidate-backends.md](implement-simd-poc-candidate-backends.md)（T3 — 共有 mod・候補実装が前提）
- 後続をブロック: [run-simd-bench-and-collect-results.md](run-simd-bench-and-collect-results.md)（T6 — 計測実行の前提）
- `#[path]` 共有 mod は二重コンパイルを伴う非標準構成。bench 側 `mod` 宣言への `#[allow(dead_code)]` を第一回避策とし、`mise run lint` 不通過時は独立 PoC support クレート案へ切替（plan Risk 11 / Open Questions）

## Affected Files

- `crates/ruviform-math/benches/simd_poc.rs` — 変更（T1 の placeholder 骨格 → ベンチ本体実装）

## Done When

- [ ] `mise run bench-smoke` がスモーク完走し、`mise exec -- cargo bench -p ruviform-math -- dot`（quick-run 例）の filter が機能する
- [ ] `target/criterion/<group>/<impl>/<input>/<len>/new/estimates.json` の出力階層が `BenchmarkId` 規約どおり生成される
- [ ] run 冒頭に環境情報（OS・CPU 型名・コア数・rustc 版・criterion 版・dispatch 検出結果）が標準出力される
- [ ] `biquad` / `sine` が `iter_batched` / `iter_with_setup` で状態初期化され、denormal 崖観測側の setup が denormal 状態を維持・再注入する

## Verification

- [ ] `mise run bench-smoke`（= `cargo bench -p ruviform-math -- --test`、スモーク）が完走する
- [ ] `mise exec -- cargo bench -p ruviform-math -- dot`（quick-run 例）が実行できる
- [ ] `ls target/criterion/` で `BenchmarkId` 規約どおりの出力階層（`<group>/<impl>/<input>/<len>/new/estimates.json`）を確認
