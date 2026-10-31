---
id: "IMPL-034Priac0A8rL49rDsjzUM"
type: "impl"
status: "in-progress"
title: "Issue #168 scalar direct overlap-NCC curve API"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/168"
    - "https://github.com/scarecrow173/audio-lens/issues/102"
  changes:
    added: []
    modified:
      - "crates/ruviform-numeric/src/corr.rs"
      - "docs/impl/ir/README.md"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #168 scalar direct overlap-NCC curve API

## Summary

audio-lens #102（R2 契約提案）が要求する「全 lag 走査して各 lag で直算
`corr0` を返す」曲線 API を `ruviform_numeric::corr` に追加する。
既存 `OverlapCorrPlan` / `CorrelationPlan` は FFT 分子経路（および
prefix 二乗和分母）を持ち、実行時コストモデルが direct/FFT を
選択するため、呼出側から「必ず直算」を契約できず AL の直算実装と
bit-exact に置換できない。本 Issue では plan/コストモデルを介さない
scalar 直算曲線 API を別系として提供する。

## Context

- Source: ruviform issue #168 / audio-lens issue #102（R2 契約提案）。
- AL 側参照実装（読み取りのみ、変更禁止）:
  - `al-analysis/src/cheap.rs` `measure_stereo` — IPD lag sweep:
    `lag ∈ [−max_lag, +max_lag]` で `corr0(xs[lag..], ys[..n−lag])`
    （正 lag は xs を後方へずらす向き）。
  - `al-analysis/src/reference.rs` `local_lag` —
    `OverlapCorrPlan::curve`（`curve[i]` ↔ `lag = i − max_lag`）を消費。
    `al-locate/examples/bench_overlap_corr.rs` の `direct_scan` が
    直算等価物（`lag ≥ 0`: `corr0(x[..L−a], y[a..])`、`|lag| ≥ L` は `0.0`）。
  - `al-locate/src/methods.rs` `band_ncc` / `pool.ncc` —
    `CorrelationPlan::new_ncc(..).ncc(..)`（template-in-signal の
    有効ラグ域 NCC 曲線）。

## Implementation

### Changes

（実装後に記入）

### Decisions During Implementation

調査で確定した事実と、それに基づく設計判断:

- `vec::corr0` の定義: `(Σxy, Σx², Σy²)` を `corr0_parts_seq` の
  同一逐次一走査で求め、`Σx² > 0 かつ Σy² > 0` のときのみ
  `Σxy / (Σx²·Σy²).sqrt()`。縮退・NaN エネルギーは `0.0`。
- 既存 `OverlapCorrPlan` の direct 経路カーネル `overlap_direct` は
  この式と完全に同じ（slicing・評価順・縮退マップまで一致）であり、
  AL の `direct_scan` とも bit-exact。ただし `curve` は呼出ごとに
  コストモデルで direct/FFT を切り替えるため、呼出側は直算を
  契約できない。
- `CorrelationPlan` の `NccDirect` mode も FFT を持たないが、
  窓エネルギーは prefix 二乗和差分 `cumulative[k+L] − cumulative[k]`、
  正規化は `v / (√Ex_template · √e)` で、いずれも `corr0` の
  `Σ / √(Σx²·Σy²)` と FP 評価が異なる → 直算 `corr0` 置換として
  bit-exact ではない。
- よって別系 API とし、分子・分母とも `corr0_parts_seq` と同一の
  逐次評価順で求める scalar 曲線関数を新設する。workspace を要さず
  （scratch 不要）、`_into` 形は alloc-free（RT-suitable）、
  `Vec` 返却形は non-RT one-shot。

## Related Experiments

### Adopted

（なし — 既知の解法であり Experiment Log は作らない）

### Rejected

（なし）

## Validation

（実施後に記入）

## Risks

（記入）

## Follow-ups

（記入）
