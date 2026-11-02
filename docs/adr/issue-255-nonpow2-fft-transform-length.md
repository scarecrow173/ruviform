---
id: "ADR-rgzm45MqKPlpzqO7Uhbqeq"
type: "adr"
status: "accepted"
title: "Issue #255 CorrelationPlan 変換長の非2冪高速サイズ化 — 既定選択の変更と混合基数 FFT の採用"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/255"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-numeric/examples/issue255_perf.rs"
      - type: "file"
        path: "docs/adr/issue-255-nonpow2-fft-transform-length.md"
    modified:
      - type: "file"
        path: "crates/ruviform-numeric/src/fft.rs"
      - type: "file"
        path: "crates/ruviform-numeric/src/corr.rs"
      - type: "file"
        path: "crates/ruviform-numeric/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-numeric/README.md"
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
  related:
    - "docs/impl/ir/issue-183-bandfilter-shared-spectrum-and-per-call-scratch-api.md"
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
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #255 CorrelationPlan 変換長の非2冪高速サイズ化 — 既定選択の変更と混合基数 FFT の採用

## Context and Problem Statement

`CorrelationPlan::new` / `new_ncc` / `OverlapCorrPlan::new` は変換長を
`next_power_of_two` に固定しており、`RealFftPlan` も 2 冪専用だった。
signal 長が 2 冪境界を僅かに超える形状（AudioLens 支配形状: S=8,640 →
16,384、S≈2.4M → 4,194,304）では FFT 演算量がほぼ 2 倍に膨らむ。
issue #255 は変換長を 2 冪以外の高速サイズ（小素数合成数）から選べる
ようにし、no-wraparound 不変条件（NccValid: n ≥ S、Overlap: n ≥ 2L−1、
Full: 全ラグ無汚染）を維持したまま過剰を削減することを要求する。

なお本対応の検証過程で、Full mode の従来境界 `n ≥ S+L−1` は出力契約を
実際には保証していないことが判明した。本実装の相関は
`c[k] = Σᵢ t[i]·s[(k+i) mod n]` の plus 方向 index であり、`S ≤ k` の
ラグ域で wrap 項 `s[k+i−n]` が実データ（index < L ≤ S）を指す。
全返却ラグ `k ≤ S+L−2` を無汚染にする真の境界は
`n ≥ S+2L−2`（最大 index アクセス `k+i ≤ S+2L−3 < n`）であり、
`nextpow2(S+L−1) < S+2L−2` となる形状（例: L=S=8 → 16 < 22、
L=S=100 → 256 < 298）では旧実装でも trailing lag が混入し得た
潜在不具合である。

## Decision Drivers

* 契約不変: 返却ラグ域・正規化・corr0 系の値契約は FFT 長に依存しない。
  `gcc_phat` の位相正規化だけが FFT 長依存であると既に明記済み。
* 下位層の一般性: `FftPlan` / `RealFftPlan` は用途非依存の公開
  プリミティブであり、受け入れ領域の拡大は既存の 2 冪入力を壊さない
  additive な変更であること。
* 性能の主目標は変換長削減（理論上限 ~30–47%）であり、非2冪経路は
  2 冪経路ほどのチューニングを要求しない（絶対性能より総演算量の
  削減が支配的）。
* RT 契約: transform は引き続き alloc なし・有界（plan 構築時に全
  係数・置換表を確保）。
* panic-free・bit-exact 規約: 公開 API は Result で検証失敗を返し、
  scalar 演算順は決定的。2 冪入力の変換結果は本変更で bit-exact に
  維持する（golden 互換）。

## Considered Options

* Option A: opt-in API を追加し、既定選択は 2 冪のまま維持する。
  （例: `CorrelationPlan::new_ncc_smooth` や長さ選択ストラテジ引数）
* Option B: 既定の変換長選択を「境界以上の最小の高速サイズ」に変更する。
  既存 API 形状は不変で、選択のみが賢くなる。
* Option C: `FftPlan`/`RealFftPlan` は 2 冪のまま据え置き、相関層だけで
  Bluestein や分割 FFT を自前実装する。

## Decision Outcome

Chosen option: "Option B — 既定選択を最小の高速サイズへ変更", because it
gives the benefit to every caller without new API surface, the observable
contract (lag ranges, normalization semantics, error domain) is unchanged,
and pre-release status plus the issue's explicit latitude ("opt-in API でも
既定選択の変更でもよい") make the transparent change the simplest coherent
design.

### 変換長の領域

* `FftPlan` / `Fft`: `n` は `{2,3,5,7}`-smooth（素因数が 2・3・5・7 のみ。
  2 冪は部分集合）を受け入れる。非2冪 smooth `n` は新しい混合基数経路で
  処理し、2 冪 `n` は従来の radix-4 DIF 経路を維持する（2 冪入力の出力は
  bit-exact 不変）。radix 8 は採用しない — 一般小 DFT では radix-4 系の
  specialized butterfly より遅く、stage 数の削減効果も限定的なため。
* `RealFftPlan` / `RealFft`: `n = 1` または「`n/2` が
  `{2,3,5,7}`-smooth な偶数」を受け入れる（偶数/奇数 index への分割が
  `m = n/2` の複素 FFT を要求するため）。奇数 `n > 1` は
  `Error::InvalidValue("n")`。
* `CorrelationPlan` / `OverlapCorrPlan` の変換長は「境界以上の最小の
  `{2,3,5}`-smooth サイズ」。NccValid は `n ≥ S`、Overlap は `n ≥ 2L−1` の
  不変条件をそのまま維持する。Full は契約の真の境界 `n ≥ S+2L−2`
  に修正する（Context 節の潜在不具合を同時に修正。`gcc_phat`/有効ラグ域
  `k ≤ S−L` は `n ≥ S+L−1` でも既に無汚染だったため、影響は
  `cross_correlation` の trailing lag のみ）。返却ラグ域と
  無汚染保証は不変。
* 選択集合から radix-7 を外す根拠は実測: 汎用小 DFT 段（radix-5/7）は
  radix-4 融合経路より 1 要素あたり高コストで、7⁴ を含む
  n=2,401,000 は nextpow2=4,194,304 の radix-4 経路より ~49% 遅かった
  （長さは 43% 短いのに）。{2,3,5} でも実用上の選択密度は十分
  （AudioLens 支配形状で 3–50% 削減を維持）であり、radix-7 は
  `FftPlan` の受け入れ領域には残すが相関層の選択対象とはしない。
* `bands`/`stft`/`conv` 等の他の FFT 利用層は対象外（独自の長さ契約を
  持ち、issue スコープ外）。`FftPlan` の領域拡大により後続で追従可能。

### Consequences

* Good: AudioLens 支配形状で変換長が削減される（実測: S=8,640 → 47.3%、
  S=66,150 → 48.5%、S=2,205,236 → 47.3%、S=2,400,240 → 42.1%）。
  変換時間も実測で改善（forward+inverse ペア: 61%/66%/90%/98%。
  `examples/issue255_perf` で再現可能）。radix-5 の専用 butterfly 化
  （Winograd 型、~8 実乗算/group）により汎用小 DFT 時の回帰を解消した。
* Good: Full mode の trailing lag wraparound（`nextpow2(S+L−1)` が
  `S+2L−2` を下回る形状）という潜在不具合を境界修正で解消した。
* Neutral: FFT 長半減に伴い `new_ncc` のコストモデル境界が移り、
  ベンチ形状 (2,205,000, 2,205,236) の選択が NccDirect → NccValid に
  反転する（値契約は同一で性能選択のみの変化）。
* Good: `fft_len()` / `memory_bytes()` で選択を観測可能（従来どおり）。
* Good: 2 冪 plan は従来経路のままで golden 互換・性能不変。
* Bad: 非2冪経路は radix-4 融合・キャッシュ傾斜・bit 反転置換の最適化が
  ない分、同長の 2 冪より 1 要素あたりのコストが高い（置換表 gather・
  入力複写・素数 radix の一般 DFT）。変換長削減がこれを上回ることが
  期待されるが、ベンチ計測で実測確認する。
* Bad: 変換長が変わる形状では出力の FP 丸めが変わる（数学契約は不変）。
  AudioLens 側の bit-exact golden は再生成が必要（pre-release 許容済み）。
* Neutral: `CorrelationWorkspace`/`OverlapCorrWorkspace` のバッファは
  `plan.n` 追従のため変更なし。`NccDirect` 等の直算経路は無関係。

### Implementation Notes

* 混合基数経路は DIF（decimation-in-frequency）: 段 radix は
  `{7,5,3}` の奇素因数を先に、残りの 2 冪部を radix-4 列＋必要なら終端
  radix-2 で処理する。radix 2/3/4/5 は専用 butterfly（radix-5 は
  Winograd 型）、radix 7 のみ plan 内小 DFT 表による一般段
  （`mixed_rn`）を使う。
* 置換は bit 反転ではなく混合基数 digit-reversal の置換表
  （`perm: Vec<usize>`）を plan に保持し、workspace 上の全段完了後に
  gather で書き戻す（inverse の 1/N スケールを畳み込む）。
* 変換は常に skew 付き workspace 経由（入力複写→全段 in-place→gather）。
  `FftWorkspace` は混合基数 plan では `n < WS_MIN` でも `2n + WS_SKEW`
  を確保する。
