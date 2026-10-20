---
id: "TASK-034Jf3FazZdLzFAFTDXUa3"
type: "task"
status: "done"
title: "スカラー参照カーネルと共通型"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-math/src/simd_poc.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-math/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/dsp-simd-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/dsp-simd-poc.md"
  blocks:
    - "implement-simd-poc-candidate-backends.md"
    - "add-simd-equivalence-matrix-tests.md"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# スカラー参照カーネルと共通型

## Work

plan `PLAN-034JdOWI3vblJW11kIueY0`（[../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md)）の Phase B / Task T2。`crates/ruviform-math/src/simd_poc.rs` にスカラー参照カーネル 8 本・共通型・deterministic 入力生成を実装し、`lib.rs` へ `#[doc(hidden)]` でモジュール配線する。

- [x] B-1. `src/simd_poc.rs` を新規作成し、共通型（`BiquadCoeffs` / `SineCoeffs` / `SineState` / `pub const FIR_TAPS: usize = 64`）とカーネルシグネチャ抽象形（設計書 Data and Control Flow 節どおり: `add` / `scale` / `clamp_unit` / `fade` / `dot` / `fir` / `biquad` / `sine`）を定義。`simd_poc` モジュールに pub enum を置かない（`KernelId` 等の識別子は `&'static str` または構造体定数で代替 — 設計書 Non-Functional Constraints の公開 enum 規約適用）
- [x] B-2. 8 カーネルのスカラー参照実装を pure std ループで記述（autovec を意図的に抑制しない・`#[inline]` 不指定・`unsafe` 0・長さ不一致は `debug_assert`）。clippy pedantic 系 lint（`cast_precision_loss` / `suboptimal_flops` / `imprecise_flops` / `wildcard_imports` 等）との衝突は、意図的抑制を `#[expect(...)]`（未対応なら `#[allow(...)]`）＋理由コメントで対象 item に局所化する（ファイル・モジュール単位の lint allow は置かない）
- [x] B-3. deterministic 入力生成（xorshift64*・固定 seed・`[-1.0, 1.0)` 一様乱数）と denormal 値生成（`f32::from_bits(0x0040_0000)` / `f32::MIN_POSITIVE * 0.5`）を実装
- [x] B-4. `src/lib.rs` に `#[doc(hidden)] pub mod simd_poc;` を追加し、`simd_poc` のモジュール doc に「実験 API・本番 RT 契約の対象外・入力生成 helper は allocate する」旨を明記
- [x] B-5. `mise run check` / `mise run lint` / `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーンであることを確認

コミット粒度は plan 規約どおり 1 タスク 1 コミット。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md) — `PLAN-034JdOWI3vblJW11kIueY0`（approved）。Phase B の正本
- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`。成果物・受け入れ基準の正本
- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（approved）。カーネル API 抽象形・等価性契約・配置の正本

## Dependencies

- 前提タスク: なし（依存グラフのルート。pure std のみで dev-dep 非依存のため [add-bench-harness-and-dev-deps.md](add-bench-harness-and-dev-deps.md)（T1）と並列実施可能）
- 後続をブロック: [implement-simd-poc-candidate-backends.md](implement-simd-poc-candidate-backends.md)（共通型・参照 API が前提）、[add-simd-equivalence-matrix-tests.md](add-simd-equivalence-matrix-tests.md)（スカラー参照が比較基準）
- MSRV 検証は T1 で確立する `rustup run 1.85 ... --lib --locked` 手順を使用（T1 未完了なら B-5 の MSRV 確認は T1 完了後に実施）

## Affected Files

- `crates/ruviform-math/src/simd_poc.rs` — 新規（共通型・8 カーネルのスカラー参照実装・deterministic / denormal 入力生成）
- `crates/ruviform-math/src/lib.rs` — `#[doc(hidden)] pub mod simd_poc;` 追加（crate doc は変更しない）

## Done When

- [x] 8 カーネル（`add` / `scale` / `clamp_unit` / `fade` / `dot` / `fir` / `biquad` / `sine`）と共通型が `#[doc(hidden)] pub mod simd_poc` 配下に公開されている
- [x] `src/simd_poc.rs` が pure std（候補クレート import なし）・`unsafe` 0・pub enum なし
- [x] deterministic 入力生成（固定 seed）と denormal 値生成が実装されている
- [x] モジュール doc に実験 API・RT 契約対象外・入力生成 helper の allocate 旨が明記されている

## Verification

- [x] `mise run check` がグリーン
- [x] `mise run lint` がグリーン（`unsafe_code` ゲート通過・lint 局所化のみ）
- [x] `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーン
