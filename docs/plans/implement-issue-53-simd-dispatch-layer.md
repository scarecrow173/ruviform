---
id: "PLAN-034JQFeYbZCBHPYSBfQe7B"
type: "plan"
status: "approved"
title: "Issue #53 SIMD dispatch 層の実装計画"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/simd-dispatch-layer.md"
  implemented-by:
    - "../tasks/simd-dispatch-token-and-kernels.md"
    - "../tasks/simd-dispatch-api-routing.md"
    - "../tasks/simd-dispatch-verification.md"
    - "../tasks/simd-dispatch-closeout.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/simd-dispatch-layer.md"
    - "docs/designs/simd-dispatch-layer.md"
    - "../adr/simd-ulp.md"
  derived-by:
    - "../tasks/simd-dispatch-token-and-kernels.md"
    - "../tasks/simd-dispatch-api-routing.md"
    - "../tasks/simd-dispatch-verification.md"
    - "../tasks/simd-dispatch-closeout.md"
  verifies: []
  verified-by:
    - "../test-specs/simd-dispatch-layer-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #53 SIMD dispatch 層の実装計画 Implementation Plan

## Goal

Issue #53 — `ruviform-math` に本番 SIMD dispatch 層を実装する: prepare 固定の不透明 `SimdToken`・8 カーネル公開 API・カーネル/サイズ別スカラールーティング・`fearless_simd` の `[dependencies]` 昇格・本番精度契約の doc 明記。

## Scope

単一実装ストリーム（`ruviform-math` 1 クレート内の変更 + その検証）。engine 層接続・AVX-512 有効化・denormal 対策は後続 Issue。

## Upstream Documents

- `docs/specs/simd-dispatch-layer.md`（SPEC-034JQubfZRC7nRmTsYvLiq・approved）
- `docs/adr/simd-ulp.md`（ADR-034JQvAkxQaMfRbMHDRLoM・accepted — 精度契約形）
- `docs/adr/dsp-simd-backend-adoption.md`（決定 1–3 — 二層構造・バックエンド・等価基準）

## Design Inputs

- `docs/designs/simd-dispatch-layer.md`（DESIGN-034JQxPUJjVzr5aekaU1B8・approved — トークン形・ルーティング閾値・API 署名）

## File Map

- `crates/ruviform-math/Cargo.toml` — `fearless_simd` を `[dependencies]` 昇格（feature なし）、dev-dep 側に `force_support_fallback` + `fearless_simd_macros` を維持
- `crates/ruviform-math/src/dsp.rs` — 新設: `SimdToken`/`Sel` enum・`detect()`/`force_*`・8 カーネル公開関数・内部 `#[simd]` `*_impl`・ルーティング閾値 const・精度契約 doc・単体テスト
- `crates/ruviform-math/src/lib.rs` — `pub mod dsp` 登録 + crate doc の「public API 未確定」文言更新
- `crates/ruviform-math/src/simd_poc.rs` — `dsp` が参照するスカラー実装・係数型の整理（re-export 経路のみ。実験実装は凍結）
- `crates/ruviform-math/tests/simd_equiv.rs` + `tests/simd_poc/*` — matrix に本番経路列を追加
- `Cargo.lock` — 昇格に伴う追跡更新

## Implementation Sequence

- [ ] T1: dep 昇格 + `dsp` モジュール新設（`SimdToken`/`Sel`/`detect`/`force_*`/`level` + `#[simd]` 8 カーネル本体を fearless_impl から移植）— コンパイル到達まで
- [ ] T2: 公開カーネル関数 + ルーティング閾値 + panic-free 化（共通プレフィックス/channels クランプ）+ 精度契約 doc 明記
- [ ] T3: 検証 — 単体テスト（forced token 全サイズ契約）・matrix 本番列追加・`rt_`+`assert_no_alloc` テスト・全量検証列
- [ ] T4: 文書クローズ（spec AC・design verification・ADR verification・task ステータス・監査）

## Dependencies and Risks

- 前提: workspace MSRV 1.89（本ブランチは `scarecrow173/issue-52-msrv-raise` 上にスタック — fearless_simd の `[dependencies]` 昇格要件）
- リスク: `#[simd]` マクロが dev-dep `fearless_simd_macros` 由来 — 本番コードに proc-macro 依存を持ち込む点を確認（`fearless_simd` 本体に再輸出があればそちらを使う。なければ macros を `[dependencies]` にも追加）
- リスク: `#[non_exhaustive]`/`cfg` ゲートで Sel variant が対象プラットフォームで空にならないこと（Fallback 常設で担保）
- リスク: ルーティングが forced-token 経路の小 N 検証をマスクする — 単体テストで `*_impl` 直接呼出しを併設して解消

## Task Breakdown

- Implementation Sequence の T1–T4 をそのままタスク化（T1→T2→T3→T4 逐次依存）

## Verification

- [ ] `SimdToken::detect()` が本機で `"avx2"` を返す
- [ ] forced 各 token で全 8 カーネル・全サイズ帯がスカラー参照と契約適合（単体テスト）
- [ ] `tests/simd_equiv.rs` の本番列（`prod_detected`/`prod_avx2`/`prod_sse4_2`/`prod_fallback`/`prod_neon` probe-gated）がグリーン
- [ ] `rt_dsp_kernels_no_alloc`（`assert_no_alloc`）がグリーン
- [ ] 公開 API に fearless_simd 型が露出しない（`cargo doc` シグネチャ確認）
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke` パス
- [ ] doc 監査で本チェーン起因 findings 0

## Open Questions / Gaps

- なし（API 形・閾値・契約形は design/ADR で確定済み）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: ルーティング閾値の確定値（design §ルーティング表のデータ根拠との一致）、panic-free 化の境界処理、forced-token コンストラクタの `#[doc(hidden)]` 露出範囲
