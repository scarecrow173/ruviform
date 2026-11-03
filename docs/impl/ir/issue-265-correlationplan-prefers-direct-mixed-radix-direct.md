---
id: "IMPL-034QMvf4KmvWw53y1DczqT"
type: "impl"
status: "completed"
title: "Issue #265 CorrelationPlan prefers_direct コストモデル校正（mixed-radix 実コスト反映と direct 維持ガード）"
created: "2026-11-02"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/265"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-numeric/benches/corr.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-numeric/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-numeric/src/corr.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-255-nonpow2-fft-transform-length.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/255"
    - "https://github.com/scarecrow173/ruviform/pull/256"
    - "https://github.com/scarecrow173/audio-lens/issues/133"
    - "https://github.com/scarecrow173/audio-lens/pull/134"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #265 CorrelationPlan prefers_direct コストモデル校正（mixed-radix 実コスト反映と direct 維持ガード）

## Summary

`prefers_direct` の FFT 側コスト推定を `n·log2(n)` から radix 構成を
反映する `n·fft_cost_units(n)` へ校正し、変換長選択 `fast_len_above`
を「最小の smooth 長」から「推定コスト最小の smooth 長」へ変更した。
`fft_cost_units(n)` は `FftPlan` の混合基数 DIF と同一の段構成を数え、
radix-3/5 段に実測フィット値（いずれも radix-2 段等価 3 ユニット）を、
大きな非 2 冪 `n` には複写・digit-reversal gather・twiddle/置換表の
キャッシュ外トラフィック分（1M–2M 要素で 0→10 ユニット飽和）を加算する。
2 冪では `fft_cost_units(n) == log2(n)` となるため、Issue #255 以前の
pow2 選択と推定値が厳密に一致する。

さらに選択候補には必ず nextpow2 が含まれるため、「新選択の推定コスト
≤ 旧 pow2 選択の推定コスト」が構造的に保証され、#265 で要求された
「旧選択で direct だった形状が新選択で遅い FFT 化される」回帰ガードが
不変条件として成立する。

## Context

Issue #255 で `fast_len_above` が最小の `{2,3,5}`-smooth 変換長を選ぶ
ようになった結果、audio-lens の method 形状
`(template_len, signal_len) = (2_400_000, 2_400_240)`（`K = 241`）では
変換長が `n = 4,194,304 → 2,430,000` に縮小し、旧モデルの推定値
`8·n·log2(n)` が `L·K = 578M` を下回って `NccDirect → NccValid` に反転。
audio-lens 実経路で NCC が `134.38ms → 218.61ms`（+63%）、e2e
+10.2%・locate +17.1% の回帰として観測された（audio-lens PR #134）。

根本原因は 2 点:

1. `n·log2(n)` が混合基数経路の per-element コスト増（専用 radix-3/5
   butterfly、入力複写、digit-reversal gather、大きな twiddle/置換表）
   を反映しない。実測では `n ≈ 2.4M` の smooth 長で pow2 の ~1.3–1.8x
   の per-element コスト。
2. `NccDirect` の直接カーネルは `K ≤ 2048` で acc が L1 常駐する axpy
   形（`real[k] += template[i]·signal[k+i]`）で、tpl≈sig 形状では
   実測 ~117ms と FFT smooth 3 変換 ~200ms を大きく上回る。

## Implementation

### Changes

- `crates/ruviform-numeric/src/corr.rs`
  - `fft_cost_units(n)` 新設: `n = 2^a·3^b·5^c·rem` に対し
    `a + 3b + 3c + 6·log2(rem) + mem(n)` を返す。`mem` は非 2 冪
    `n > 1<<20` で `(n−2^20)/2^20` を [0,1] に clamp した 0–10
    ユニット。2 冪ではちょうど `log2(n)`。
  - `prefers_direct`: FFT 推定を
    `DIRECT_NCC_COST_RATIO·n·log2(n)` → `·n·fft_cost_units(n)`。
    `K ≤ 2048` 上限は不変。
  - `fast_len_above`: 同一の候補列挙（`t = 3^b·5^c ≤ v`,
    `n = 2^⌈log2(v/t)⌉·t`）から `n·fft_cost_units(n)` の argmin を返す
    （同コストは小さい `n` 優先）。nextpow2 が常に候補かつ
    nextpow2 超過候補は `u(n) > log2(n)` で必敗のため
    `n ≤ nextpow2(v)` は不変。
  - `OverlapCorrPlan` の呼出ごと direct/FFT ルーティングも
    `15·n·⌈log2 n⌉` → `15·n·fft_cost_units(n)` に統一。
  - テスト: mode pin を回帰形状群（2,400,000/2,400,240、
    2,205,000/2,205,236、551,250/551,309、1,000,000/1,000,120、
    100,000/100,101 → `NccDirect`）と FFT 優位形状（8,192/8,400、
    4,410/7,938 等 → `NccValid`）で再固定。`fast_len_above` テストは
    argmin 契約 + 「選択コスト ≤ nextpow2 コスト」ガード不変条件の
    oracle 検証へ書き換え。
- `crates/ruviform-numeric/benches/corr.rs` 新設: `fft_len`
  群（pow2 vs smooth の per-element 実コスト、units 再フィット用）と
  `ncc_plan` 群（回帰形状の E2E `ncc` 計測）の criterion ベンチ。
- `crates/ruviform-numeric/Cargo.toml`: criterion dev-dep + `[[bench]]`。

### Decisions During Implementation

- **係数を実測フィット値に固定**: radix-3/5 段の単位コストを
  `radix−1`（2/4）ではなく実測フィットの 3/3 にした。per-element 時間を
  `us/elem` で並べると radix-3/5 段は radix-2 段の ~2.8–3.5x で、
  単純な flop 数見積りより専用 butterfly の実コストが支配的だった。
- **大 `n` の mem 項は飽和型**: `n ≥ 2M` 要素で観測される +5–12
  ユニットの超過分は段数に比例せず、作業領域が L3 を超えることへの
  一律的な追加分として `n ∈ (1M, 2M]` で線形 ramp → 10 で飽和させた。
- **選択は argmin、ガードは構造的不変条件**: 「新旧選択の推定コスト
  比較」を別ロジックで書くより、候補集合に nextpow2 を常に含めて
  argmin する形にすると「新選択 ≤ 旧選択の推定コスト」が自明に
  成立し、ガードはテストで不変条件として検証する形にできた。
- **直接カーネルの実測は plan 内部 axpy 形で取る**: 当初
  `ncc_direct_into`（公開の scalar 曲線 API）で直接計算を計測したが、
  アクセスパターンが異なり ~731ms と全く別の値になった。
  `CorrelationPlan::NccDirect` の実カーネル（axpy 形）は ~117ms で、
  audio-lens の 134ms と整合する。ベンチも `plan.ncc` 経路で計測する。

## Related Experiments

### Adopted

- ローカルプローブ計測（`examples/issue265_probe.rs`、検証後に
  `benches/corr.rs` へ恒久化して削除）:

  | shape (L, S, K) | direct (axpy) | pow2 3t | smooth 3t | 選択 |
  | --- | --- | --- | --- | --- |
  | 2,400,000 / 2,400,240 / 241 | 117ms | 196ms (4,194,304) | 200ms (2,430,000) | NccDirect |
  | 2,205,000 / 2,205,236 / 237 | 104.5ms | 201ms (4,194,304) | 164ms (2,211,840) | NccDirect |
  | 551,250 / 551,309 / 60 | 7.0ms | 40.9ms (1,048,576) | 22.7ms (552,960) | NccDirect |
  | 1,000,000 / 1,000,120 / 121 | 28.7ms | 42.7ms (1,048,576) | 56.7ms (1,012,500) | NccDirect |
  | 100,000 / 100,101 / 102 | 2.1ms | 3.3ms (131,072) | 3.6ms (101,250) | NccDirect |
  | 8,192 / 8,400 / 209 | 340μs | 321μs (16,384) | 200μs (8,640) | NccValid |
  | 4,410 / 7,938 / 3,529 | 3.5ms | 143μs (8,192) | 173μs (8,000) | NccValid |

  `fft_cost_units` での推定値と実測の大小関係が全形状で一致する。

### Rejected

- **`prefers_direct` のみ校正して変換長選択を最小 smooth のまま残す**:
  mode 判定は直るが Full mode（`n ≥ S+2L−2`）では
  `7,290,000` のような高コスト混合長を選び続け、audio-lens の
  Full-mode 系（anchor GCC-PHAT）で `8,388,608` pow2 より遅い FFT を
  残す。argmin 化で `n·u` が pow2 より高い長さは選ばれなくなり、
  Full mode でも旧 pow2 と同等以上の選択に戻る。
- **radix-7 を選択集合へ戻す**: 汎用小 DFT の per-element コストが
  高い実測（#255 で `2³·5³·7⁴` が nextpow2 より ~49% 遅延）は変わらず。
  `fft_cost_units` の `OTHER_RADIX_STAGE_UNITS` は防御的な重みとして
  残すが選択集合は `{2,3,5}` のまま。

## Validation

- `git merge origin/main` — Already up to date（変更点なし）。
- `mise run fmt-check` / `mise run check` / `mise run lint` — green。
- `mise run test` — ワークスペース全 115 suite green・0 failure
  （corr 内の mode pin・argmin/guard oracle・direct/FFT 一致・
  no-wraparound・正規化・lag 域の既存契約テストを含む）。
- `cargo bench -p ruviform-numeric --bench corr`（本機・release）:

  | 計測 | 値 |
  | --- | --- |
  | `ncc_plan` 2,400,000/2,400,240（NccDirect） | **134.95ms**（audio-lens の旧 direct 実測 134.38ms と一致） |
  | `ncc_plan` 2,205,000/2,205,236（NccDirect） | 122.70ms |
  | `ncc_plan` 551,250/551,309（NccDirect） | 8.96ms |
  | `ncc_plan` 100,000/100,101（NccDirect） | 2.52ms |
  | `ncc_plan` 8,192/8,400（NccValid=FFT、直接は ~340μs） | 204.10μs |
  | `ncc_plan` 4,410/7,938（NccValid=FFT） | 166.56μs |
  | `fft_len` pair: 4,194,304（pow2） | 129.02ms |
  | `fft_len` pair: 2,457,600（2¹⁵·3·5²、new_ncc 選択） | 132.19ms |
  | `fft_len` pair: 2,430,000（旧最小 smooth 選択） | 136.17ms |
  | `fft_len` pair: 2,211,840（#255 headline） | 113.29ms |
  | `fft_len` pair: 4,251,528（2³·3¹²） | 276.26ms |

  3 変換 ≈ pair×1.5 とすると回帰形状での FFT 経路は ~194–204ms で、
  plan 選択の direct 経路 135ms が実測でも正しく、モデル推定順序
  （2,457,600 ≤ pow2 < 2,430,000 < 4,251,528）と計測順序が一致する。

## Risks

- `fft_cost_units` の係数（3/3/10）は本機（開発機 Ryzen 系）の実測
  フィット。消費者機でキャッシュ階層が異なると mem 項の境界
  （1M–2M 要素）がずれる可能性はあるが、argmin 化により旧 pow2
  推定を超える選択は構造的に排除されるため、悪化方向の回帰は起きない。
- `bench` 群は criterion の `Throughput::Elements` で per-element を
  出すため将来の再校正にも使えるが、CI の `bench-smoke` は
  `ruviform-math`/`ruviform-rt` のみを対象とするため本ベンチは
  ローカル計測用である。

## Follow-ups

- audio-lens PR #134 での e2e 再計測（pin 更新後に method/locate の
  +10%/+17% 回帰が解消することを消費者側で確認）。
