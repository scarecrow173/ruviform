---
id: "IMPL-034Priac0A8rL49rDsjzUM"
type: "impl"
status: "completed"
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
      - type: "file"
        path: "crates/ruviform-numeric/src/corr.rs"
      - type: "file"
        path: "crates/ruviform-numeric/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-numeric/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
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

- `crates/ruviform-numeric/src/corr.rs`
  - モジュール doc: 直算曲線 family の位置づけと plan 系 FFT 経路との
    FP 差分・使い分けを追記（規約に `_into` 形の RT-suitable 契約も明記）。
  - `ncc_direct_len`（内部 helper）: 出力長 `min(max_lag, S−L) + 1`、
    退化 shape では `0`（`CorrelationPlan` と同じクランプ規約）。
  - `ncc_direct_into(template, signal, max_lag, out)` — template-in-signal
    の直算 NCC 曲線。`out[k] = corr0(template, signal[k..k+L])` と
    bit-exact。分子・分母とも `corr0_parts_seq` と同一逐次評価順で
    累積し、正規化は `Σx² > 0 かつ Σy² > 0` のときのみ
    `sxy / (Σx²·Σy²).sqrt()`。template エネルギーは 1 回だけ評価
    （全ラグ同一値で評価順も同一のため bit-exact を損なわない）。
    `out.len()` の不一致は `Err(Error::InvalidValue("length"))`。
    RT-suitable（alloc なし・workspace 不要）。
  - `ncc_direct(template, signal, max_lag)` — one-shot `Vec` 版（non-RT）。
  - `overlap_ncc_direct_into(x, y, max_lag, out)` — 等長窓の両側 lag
    直算曲線。`out[i]` は `lag = i − max_lag` で overlap 区間
    `L−|lag|` 上の `corr0`（`|lag| ≥ L` は `0.0`）。既存 direct 経路
    カーネル `overlap_direct` をそのまま再利用し、AL の `direct_scan`
    と slicing・向き・縮退マップが一致。`x`/`y` 不等長は
    `Err("y")`、`2·max_lag+1` overflow は `Err("max_lag")`、
    `out.len()` 不一致は `Err("length")`。RT-suitable。
  - `overlap_ncc_direct(x, y, max_lag)` — one-shot `Vec` 版（non-RT）。
  - 契約テスト 6 件追加（`corr::tests`）。
- `crates/ruviform-numeric/src/lib.rs`
  - `rt_processing_paths_do_not_allocate` に 2 つの `_into` API の
    alloc-free 検証を追加（`assert_no_alloc` 配下で呼出）。
- `crates/ruviform-numeric/README.md`
  - `corr` の責務記述に直算曲線 family と Issue #168 を追記。

### Normalization 契約（AL 実装との突き合わせ）

| 要素 | `ncc_direct` | `overlap_ncc_direct` | AL 直算実装 |
| --- | --- | --- | --- |
| 分子 | `Σᵢ template[i]·signal[k+i]`（i 昇順 `+=`） | `Σᵢ xs[i]·ys[i]`（overlap 区間・i 昇順 `+=`） | `corr0` の `sxy` と同一 |
| 分母 | `√(Σ template² · Σ window²)` | `√(Σ xs² · Σ ys²)`（overlap 区間） | `corr0` の `(ex·ey).sqrt()` と同一 |
| 端点処理 | `signal[k..k+L]` の全域一致のみ（k ≤ S−L） | overlap 長 `L−|lag|` に縮退、`|lag| ≥ L` は `0.0` | `direct_scan` と同一 |
| ゼロ分散/縮退 | `Σx² > 0 かつ Σy² > 0` でなければ `0.0`（NaN も `0.0`） | 同左 | `corr0` と同一 |
| 中心化 | なし（non-centered corr0） | なし | なし |
| 評価順 | `corr0_parts_seq` と同一（逐次・非 FMA） | `overlap_direct` カーネル = `corr0_parts_seq` | `vec::corr0` と bit-exact |

- `ncc_direct` は `CorrelationPlan::ncc` の lag 方向・`min(max_lag, S−L)+1`
  クランプを共有するが、正規化は plan 系の `√Ex_template · √e`・prefix
  二乗和分母とは別式であり、同一ラグ値は数学的に同値・FP は非一致。
- `overlap_ncc_direct` の lag 向きは `OverlapCorrPlan::curve` /
  AL `direct_scan` と同一（正 lag: `x[..L−a]` vs `y[a..]`）。
  AL `cheap.rs` IPD sweep は逆方向（正 lag: `xs[a..]` vs `ys[..n−a]`）の
  ため、`overlap_ncc_direct(y, x)` もしくは index 反転で同値に写像する
  （`corr0` の引数対称性により bit-exact）。テストで両写像を固定した。

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

- `mise run fmt` — pass（`cargo fmt --all`、差分なし）。
- `mise run check` — pass（workspace `--all-targets`）。
- `mise run lint` — pass（`clippy -D warnings`、pedantic 含む）。
- `mise run test` — pass（workspace 87 suite 全件 0 failed、
  ruviform-numeric 83 件 + doctest 4 件）。
- `mise run build` — pass。
- `mise run test-rt` — pass（`rt_processing_paths_do_not_allocate` が
  `assert_no_alloc` 配下で 2 API の alloc-free を検証）。
- `git fetch origin main && git merge origin/main` — 実施済み
  （merge 21d1cfc、clean merge。upstream の numeric/linalg・
  Cargo.lock 更新を取り込み、最終検証は merge 後の tree で実施）。

未実施の検証:
- `bench`/`bench-smoke` — 本変更は性能経路を追加しない（直算のみ）ため
  未実施。直算のコスト特性は AL 側 `bench_overlap_corr.rs` が担当。
- AL 側での実置換（audio-lens #102 側の作業）— ruviform 側の API 提供が
  本 Issue の範囲であり、AL の呼出変更は audio-lens の対応 Issue で実施する。

## Risks

- `ncc_direct` は `Σ template²` を全ラグでホイストする等価性に依存する
  （評価順不変で bit-exact）。テストが全ラグ `corr0` exact equality を
  pin しており、将来の実装修正時の退行はテストで検出される。
- `ncc_direct_into` の `out.len()` は厳密一致契約
  （`min(max_lag, S−L)+1`）。呼出側は lag 数を自分で確定させる必要が
  あるが、`ncc` と同じクランプ規約なので機械的に写像可能。
- 直算は O(L·K) で FFT 経路より大形状では遅い — これは契約上の意図
  （bit-exact 優先）であり、性能最適化は Issue 範囲外。

## Follow-ups

- audio-lens 側: IPD lag sweep（`cheap.rs`）、`local_lag`
  （`reference.rs` の `OverlapCorrPlan::curve` 消費箇所）、`pool.ncc`
  （`band_ncc`/`full_ncc`/`ipd_block_lag`）の直算 API への置換判断は
  audio-lens #102 の R2 側作業。
- `OverlapCorrWorkspace` における `with_max_lag` の `2*max_lag+1` は
  未検査算術で `usize::MAX` 入力時に wrap し得る（既存の潜在問題、
  本 Issue 非因果 — 別 Issue として報告候補）。
