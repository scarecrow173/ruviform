---
id: "IMPL-034R255MixedRadixFftXy"
type: "impl"
status: "completed"
title: "Issue #255 CorrelationPlan 変換長の非2冪高速サイズ化（混合基数 FFT + smooth 長選択）"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/255"
    - "https://github.com/scarecrow173/audio-lens/issues/71"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-numeric/examples/issue255_perf.rs"
      - type: "file"
        path: "docs/adr/issue-255-nonpow2-fft-transform-length.md"
      - type: "file"
        path: "docs/impl/ir/issue-255-nonpow2-fft-transform-length.md"
    modified:
      - type: "file"
        path: "crates/ruviform-numeric/src/fft.rs"
      - type: "file"
        path: "crates/ruviform-numeric/src/corr.rs"
      - type: "file"
        path: "crates/ruviform-numeric/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-numeric/README.md"
      - type: "file"
        path: "docs/designs/issue-152-f64-correlation.md"
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/issue-255-nonpow2-fft-transform-length.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-152-f64-correlation.md"
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

# Issue #255 CorrelationPlan 変換長の非2冪高速サイズ化

## Summary

`FftPlan`/`RealFftPlan` に {2,3,5,7}-smooth 合成数の変換長を受け入れる
混合基数 DIF 経路を追加し、`CorrelationPlan`・`OverlapCorrPlan` の
変換長選択を `next_power_of_two` から {2,3,5}-smooth 最小長
`fast_len_above` へ変更した。AudioLens 支配形状で変換長が
42–49% 削減され、実測時間も改善（最大 ~39% 短縮・最大形状でほぼ同等）。
検証過程で Full mode の wraparound 防止境界が `n ≥ S+L−1` では不足
（実際は `n ≥ S+2L−2` が必要）という潜在不具合を発見し、併せて修正した。

## Context

- Source: ruviform issue #255。`next_power_of_two` 選択では 2 冪境界を
  僅かに超えた形状で変換量がほぼ 2 倍になる（S=8,640 → n=16,384 等）。
- AudioLens issue #71（帯域相関の性能）は先行需要。issue #183 の
  `BandSpectrum`/`BandScratch` 共有は forward 回数を減らすが変換長選択は
  扱わず、本 Issue が変換量そのものを削る補完的位置付け。

## Implementation

### Changes

- `crates/ruviform-numeric/src/fft.rs`
  - `FftPlan` に `stages: Vec<MixedStage>`（radix 列・各段の長さ・twiddle
    オフセット）と `perm: Vec<usize>`（混合基数 digit-reversal 表）を追加。
    2 冪 `n` は従来の radix-4 DIF 経路を維持（出力 bit-exact 不変）。
  - 非2冪 smooth `n` は workspace 上で全段 in-place 実行後に `perm` で
    gather 書き戻す（inverse の 1/N スケールを畳み込む）。段 radix は
    `{7,5,3}` の奇素因数を先に、残り 2 冪部を radix-4 列＋終端 radix-2。
  - 段カーネル: radix-2/3/4 は専用 butterfly、radix-5 は Winograd 型の
    専用式（`mixed_r5`）、radix-7 のみ汎用小 DFT 段（`mixed_rn`）を使用。
  - `RealFftPlan` は内部複素 FFT が `m = n/2` のため `n` 偶数の smooth 長を
    受付（`n=1` は従来どおり恒等）。`m` 奇数時の forward packing
    （self-conjugate bin が存在しない）を修正 — 従来コードは `mid = m/2`
    を無条件に自己共役扱いしていたため n=6 や n=7,938 で破綻していた。
  - `FftWorkspace`/`RealFftWorkspace` は混合 plan に `2n+WS_SKEW` /
    `2m+WS_SKEW` を確保。`memory_bytes` に `perm`/`stages` を計上。
- `crates/ruviform-numeric/src/corr.rs`
  - `fast_len_above(v)`: `v` 以上の最小 {2,3,5}-smooth 偶数を返す
    （checked arithmetic・`None` on overflow）。
  - `CorrelationPlan::new`: 境界を `signal_len + 2·(template_len−1)`
    （= `S+2L−2`）に修正。従来の `S+L−1` は非負ラグ相関
    `c[k] = Σᵢ t[i]·s[(k+i) mod n]` の最大アクセス index
    `k+i ≤ S+2L−3` をカバーせず、pow2 の余剰で覆い隠されていた
    trailing-lag wraparound を修正（regression test 追加）。
  - `new_ncc`: `fast_len_above(signal_len)` — no-wraparound 不変条件
    「`n ≥ signal_len` の最小高速サイズ」を維持。
  - `OverlapCorrPlan`: `fast_len_above(2·window_len−1)`。
- `crates/ruviform-numeric/src/lib.rs`
  - RT テスト既存ケースが非2冪 plan（`new(64,300)` → n=432、
    `new_ncc(64,300)` → n=300）を通過し、混合基数経路の alloc-free を
    機械検証済み（テスト変更なし）。
- `crates/ruviform-numeric/README.md` / `lib.rs` doc
  - smooth 変換長の受入領域を明文化。
- `crates/ruviform-numeric/examples/issue255_perf.rs`（新規・手動計測用）
  - 旧 nextpow2 長と新 smooth 長の `RealFftPlan` forward+inverse ペアを
    release ビルドで簡易計時。
- `docs/adr/issue-255-nonpow2-fft-transform-length.md`（新規）
  - Option B（既定選択の変更）採用の設計判断・radix-7 除外・
    Full-mode 境界修正の記録。
- `docs/designs/issue-152-f64-correlation.md`
  - 変換長表記と Full-mode 境界の 2 箇所に Issue #255 による更新・
    修正の追記（履歴テキストは保持）。

### Decisions During Implementation

- **相関層の選択集合を {2,3,5}-smooth に限定**: radix-7 を含む候補
  （n=2,401,000=2³·5³·7⁴）は 57% の変換長にも関わらず nextpow2 より
  ~49% 遅かった（汎用 `R²` 小 DFT 段が律速）。`FftPlan` は radix-7 を
  受け入れたまま API 一般性を維持し、相関層の選択集合からのみ除外。
- **radix-5 の専用 butterfly 化**: 初回実装では radix-5/7 を汎用
  `mixed_rn`（plan 内 W_R 表の `R²` 小 DFT）に流したが、radix-5 を多く
  含む長（67,500=2²·3³·5⁴ 等）で 20–29% の速度回帰が実測された。
  Winograd 型の専用式（cos72°/cos144°/sin72°/sin144° の定数）で
  group あたりの実乗算を ~8 に削減し、全代表形状で回帰を解消した。
- **Full-mode 境界 `S+2L−2`**: 設計書（issue-152）は `n ≥ S+L−1` で
  無汚染と明記していたが、実装の非負ラグ方向 `c[k] = Σᵢ t[i]·s[(k+i) mod n]`
  では trailing 部分 overlap ラグが wrap 項 `s[k+i−n]` を実データへ
  落とす。`nextpow2` の余剰が偶発的に充足していた形状では従来実装でも
  混入していた潜在不具合のため、不変条件を所有する plan 構築層で修正。
- **`m` 奇数の real packing**: `mid = m/2` の自己共役 bin は `m` 偶数時のみ
  存在する。`m` 奇数では全非 self-conjugate bin を対に処理する分岐を追加
  （`m` 偶数の既存処理は不変）。この不具合も `n/2` が常に 2 冪=偶数だった
  従来領域では発現し得なかった。

## Related Experiments

### Adopted

（なし — `examples/issue255_perf.rs` は計測プローブであり Experiment Log
の採否対象ではない）

### Rejected

- radix-7 を相関層選択集合に含める案 — 実測で回帰のため選択集合から除外
  （`FftPlan` の受入領域からは除外せず）。

## Validation

- `mise run fmt` / `mise run fmt-check` — pass。
- `mise run check` — pass（workspace 全ターゲット）。
- `mise run lint` — pass（`cargo clippy --workspace --all-targets -- -D warnings`）。
- `cargo test -p ruviform-numeric` — 116 unit + 7 doctest 全 pass。
  新規テスト: 複素混合基数の naive DFT 突合・impulse/root・逆変換
  round trip、real FFT forward oracle 突合・round trip、非対応長拒否、
  `fast_len_above` の最小性・nextpow2 以下保証、Full-mode trailing lag
  無汚染回帰、非2冪 plan の NCC/生相関 oracle 一致。
- `mise run test` — workspace 全テストバイナリ pass・0 fail。
- `mise run test-rt` — RT テスト全 pass（非2冪 plan の alloc-free 含む）。
- 実測（`issue255_perf` example、release ビルド、forward+inverse ペア）:
  | shape | 旧 n | 新 n | 長比率 | 時間比率 |
  | --- | --- | --- | --- | --- |
  | S=8,640 ncc-valid | 16,384 | 8,640 | 52.7% | ~61% |
  | S=66,150 anchor-mid | 131,072 | 67,500 | 51.5% | ~66% |
  | S=2,205,236 band-ncc | 4,194,304 | 2,211,840 | 52.7% | ~90% |
  | S=2,400,240 anchor-ncc | 4,194,304 | 2,430,000 | 57.9% | ~98% |
  - pow2 セルフチェック n=65,536: 比率 ~1.02（同等）。
- review-protocol: CONFIRMED P1 1 件（`new` rustdoc の旧境界表記）
  → 修正済み。残存 P0/P1 なし、verdict: ACCEPT WITH NON-BLOCKING NOTES。

## Risks

- 混合基数経路は radix-4 経路より 1 要素あたりのコストが高い（置換表
  gather・小基数段）。{2,3,5} 選択集合では代表形状で実時間回帰なしを
  確認したが、形状によっては長比率ほど時間改善しない（2,430,000 はほぼ
  同等）。radix-7 は `FftPlan` 受入領域内だが相関層では使わない。
- AudioLens 側の bit-exact golden は FFT 演算順の変化で再生成が必要
  （pre-release 許容済み・本 Issue のスコープ外、consumer 追従は別 wave）。
- 混合基数経路の数値特性は 2 冪経路と厳密には異なる（演算順差による
  丸め）。単体テストの oracle 突合は許容差内で検証済み。

## Follow-ups

- AudioLens 側 golden 再生成と `BandSpectrum` 等 consumer 追従は
  別リポジトリ/別 wave の作業。
- radix-5/7 以外の小基数（例: radix-11）は需要が確定するまで追加しない
  （YAGNI）。
