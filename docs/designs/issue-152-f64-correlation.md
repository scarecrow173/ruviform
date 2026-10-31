---
id: "DESIGN-034R152f64Corr3l4t10nXy"
type: "design"
status: "approved"
title: "Issue #152 f64 相互相関・NCC・GCC-PHAT・overlap 相関 API（ruviform-numeric `corr` モジュール）"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/152"
    - "https://github.com/scarecrow173/ruviform/issues/150"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  changes:
    added:
      - "crates/ruviform-numeric/src/corr.rs"
    modified:
      - "crates/ruviform-numeric/src/lib.rs"
      - "crates/ruviform-numeric/src/vec.rs"
      - "crates/ruviform-numeric/README.md"
      - "crates/README.md"
      - "docs/designs/ruviform-architecture-design-baseline.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "ruviform-architecture-design-baseline.md"
    - "issue-150-f64-offline-numeric.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/simd-ulp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ruviform-dsp.md"
  refines: []
  refined-by: []
  derives-from:
    - "ruviform-architecture-design-baseline.md"
    - "issue-150-f64-offline-numeric.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/blob/main/crates/al-math/src/correlation.rs"
  defers: []
  deferred-by: []
---

# Issue #152 f64 相互相関・NCC・GCC-PHAT・overlap 相関 API

## Overview

AudioLens の `al-math::correlation`（`CorrelationPlan`/`CorrelationWorkspace`/
`OverlapCorrPlan`/`OverlapCorrWorkspace` + free function）を、用途非依存の
f64 相関カーネルとして `ruviform-numeric` の **`corr` モジュール**に移植する
（Issue #152、AudioLens 移行追跡 audio-lens#95・責務マップ「#152 全量を移す」行）。

`CorrelationPlan` は template-in-signal の有効ラグ域 `0..=S−L` 上で
符号付き NCC・生相互相関・GCC-PHAT を提供し、`OverlapCorrPlan` は等長窓の
両側ラグ `−max_lag..=+max_lag` で非中心化正規化相関（overlap 区間の `corr0`）
を提供する。#150 が確立した plan（不変・共有）/workspace（`&mut` 排他・全
事前確保）/free fn（one-shot non-RT）の三層と、panic-free `Result`・
bit-exact 逐次評価・RT-suitable 明記の契約をそのまま継承する。

AudioLens 固有の意味論（`LocateMethod`・複数候補合議・anchor/window 選択・
reliability/Validity・ジョブ書き出し・ピーク選定ポリシー）は一切移さない。

## Context

- #150 が `ruviform-numeric`（Primitives 層・std のみ）を新設し、
  `fft`/`vec`/`window`/`resample`/`conv`/`linalg` を確立済み。同設計書の
  Deferred Design Concerns は `correlation`（NCC/GCC-PHAT/相互相関）を
  本クレートの `fft`/`vec` プリミティブ上に構築する想定と明記しており、
  本 Issue はその着手である。
- al-math 版は pulp SIMD dispatch + panic 契約（`assert!`）+ TLS 前提の
  FFT workspace。#150 と同じ変換規則を適用する: panic → `Result`、TLS →
  呼出し側所有 workspace、SIMD dispatch → sequential scalar（bit-exact）。
- `al-math::correlation` の公開面は 5 API（plan/workspace 2 組 +
  free fn 3 本）と小さいが、利用側は al-locate（full_ncc/coarse_ncc/
  band_ncc/gcc_phat・plan プール）と al-analysis（band_gain の窓 NCC・
  cheap 自己相関・reference の per-window lag）の双方から呼ばれる。
  band_gain は `CorrelationPlan::new_ncc(window, window+2·max)` の
  対称ラグ探索と `vec::dot`/`sum_sq` の gain 計算を組合せて使い、
  reference は `OverlapCorrPlan` で per-window lag τ(t) を推定する。
- 受け入れ条件は「曖昧性を検証できる証拠を隠さず返す」ことを要求する。
  al-math は正規化時の `e > 0` 判定でゼロエネルギー窓を 0.0 へ写像するが、
  「無相関で 0」と「無音窓で 0」をカーブ値だけからは区別できない。
  `band_gain` はこの区別のため `sum_sq(b)` を別途再計算している。
  本 API は正規化に使ったエネルギー自体を返却し、利用側の再計算を不要にする。

## Considered Options

### 配置

| 配置 | 評価 |
| --- | --- |
| A. `ruviform-numeric::corr`（#150 の deferred 想定どおり） | 相関は f64・確保を伴う plan 構造で `ruviform-math` の「全公開関数 alloc なし」契約にも `ruviform-dsp` の「audio ドメイン型付き f32」責務にも収まらない。#150 で確立した fft/vec/plan/workspace の土台をそのまま利用でき、依存追加なし。**採用** |
| B. `ruviform-dsp` 内 f64 モジュール | #150 が同一理由で棄却済み（非オーディオ数値利用に audio-core 依存を強いる・f32 RT 系と混在）。不採用 |
| C. `ruviform-audio-analysis` | 相関は解析結果（レベル/ピーク/特徴量）ではなく汎用カーネル。audio-analysis の責務（ドメイン型付き解析 API）ではない。不採用 |

### API 形

| 選択 | 評価 |
| --- | --- |
| 出力を `&'w [f64]` / 借用 struct で返す（al-math と同じ） | 有効ラグ域が数百万点に達する full_ncc 系で出力コピーを避けられる。`&'w mut ws` 借用がカーブ生存中の workspace 再使用を型で禁止する。**採用** |
| `out: &mut [f64]` へ書き込む | 呼出側が出力バッファを所有する形。workspace と同じ形状管理を二重に要求し、借用型の方が簡潔。不採用 |
| 証拠を `NccCurve`/`OverlapCurve` 借用 struct で返す | `values` と `window_energy`/`overlap_energy` は同一 workspace 内の別バッファなので単一 struct で分割借用できる。証拠を別名 accessor にすると `&'w` 借用中に呼べない。**採用** |
| `gcc_phat` を NCC plan でも許可する | PHAT の bin 正規化は FFT 長依存のため、短縮 FFT（`new_ncc`）plan では `new` plan と異なる結果になる。al-math は panic で拒否。本 API は `Err(Error::InvalidValue("mode"))` で拒否（契約を型ではなく実行時タグで表す — plan のモードは既に shape 確定時の内部属性）。**採用** |
| `OverlapCorr` の `max_lag` を plan ではなく呼出時パラメータに維持 | 利用側は plan 共有下で呼出ごとに `max_lag` を変えない（reference は `cfg.lag_max` 固定）。ただし plan 化すると「同一窓長で探索幅だけ変える」用途で plan 増殖する。出力容量は workspace 側の問題なので、workspace に容量選択を持たせ呼出時 `max_lag` は容量内で許す。**採用** |

## API Surface

全 API は `f64` 専用。公開 enum は `Error` のみ（既存・`#[non_exhaustive]`）。
`PlanMode`・`Kernel` は crate-private。

| モジュール | API | RT 契約 |
| --- | --- | --- |
| `corr` | `CorrelationPlan::new`（Full mode: n = `nextpow2(S+L−1)`）/ `CorrelationPlan::new_ncc`（`NccValid`: n = `nextpow2(S)` または `NccDirect`: FFT なし直接計算） | non-RT only（確保） |
| `corr` | `CorrelationWorkspace::new(&plan)` | non-RT only（全バッファ事前確保） |
| `corr` | `CorrelationPlan::ncc(&mut ws, template, signal, max_lag) -> Result<NccCurve<'_>>` | **RT-suitable** — alloc なし・有界 |
| `corr` | `CorrelationPlan::gcc_phat(...) -> Result<&[f64]>` | **RT-suitable**（Full mode plan のみ） |
| `corr` | `CorrelationPlan::cross_correlation(...) -> Result<&[f64]>` | **RT-suitable** |
| `corr` | `OverlapCorrPlan::new(window_len)`（n = `nextpow2(2L−1)`） | non-RT only |
| `corr` | `OverlapCorrWorkspace::new(&plan)`（容量 `2L−1`）/ `with_max_lag(&plan, max_lag)`（容量 `2·max_lag+1`） | non-RT only |
| `corr` | `OverlapCorrPlan::curve(&mut ws, x, y, max_lag) -> Result<OverlapCurve<'_>>` | **RT-suitable** |
| `corr` | `ncc` / `gcc_phat` / `cross_correlation` / `overlap_corr_curve`（one-shot、Vec 返却） | non-RT only |
| `vec` | （追加）`pub(crate)` 共有カーネル `axpy_seq`・`corr0_parts_seq` | —（非公開） |

## Correlation Contract（明文化項目）

### `CorrelationPlan`（template-in-signal・非負ラグ）

- **ラグ意味**: `values[k]` は `signal[k..k+L]` と `template`（長さ L）の比較。
  `k` は template の信号内先頭位置（非負のみ）。`max_lag` は呼出時に
  `min(max_lag, bound)` へクランプされる（エラーではない）。
  NCC/PHAT の `bound` = `S−L`（有効ラグ域・全 overlap）、
  `cross_correlation` の `bound` = `linear_lag_max`（Full mode: `S+L−2`、
  NccValid/NccDirect: `S−L`）。`S+L−2` 以降のラグは overlap が存在せず
  線形相関として意味を持たないため Full mode の上限とする。
- **NCC（`ncc`）**: `values[k] = Σᵢ template[i]·signal[k+i] /
  (√(Σ template²) · √(Σⱼ signal[k+j]²))`。非中心化・符号付き（±1 範囲、
  反極性は負値）。正規化規則は `window_energy[k] > 0 かつ
  template_energy > 0` のときのみ商を返し、それ以外は `0.0`。
- **証拠（`NccCurve`）**: `values`（カーブ）・`window_energy[k]`
  （正規化で使用した `max(0, Σⱼ signal[k+j]²)`。NaN は `max` 適用前に
  検出して伝播）・`template_energy`（`Σ template²`）を同時に返す。
  `template_energy == 0` なら全 `values` が `0.0`（この場合も
  `window_energy` は実計算値を返し、縮退の判別を隠さない）。
- **GCC-PHAT（`gcc_phat`）**: 相互スペクトル `S[k] = conj(A[k])·B[k]` の
  各 bin を `|S[k]|` で白色化（成分ごとに `bin/|S[k]|`。`|S[k]|² ≤ 0` の
  ゼロ cross bin は `0.0`）して逆変換。Full mode plan のみ受付
  （短縮 FFT では位相正規化結果が変わるため `new_ncc` plan は
  `Err(Error::InvalidValue("mode"))`）。エネルギー規則を持たないため
  返却は `&[f64]` のみ。
- **生相互相関（`cross_correlation`）**: 正規化なしの線形相互相関分子
  `c[k] = Σᵢ template[i]·signal[k+i]` を `0..=min(max_lag, linear_lag_max)`
  で返す。Full mode では `S−L < k ≤ S+L−2` の部分 overlap ラグも
  無汚染（n ≥ S+L−1 で wraparound なし）。
- **NccValid の有効性**: FFT 長 `n = nextpow2(S)` でも返却域 `k ≤ S−L` は
  wraparound 項 `c[k−n]`・`c[k+n]` がいずれも定義域外のため無汚染
  （al-math と同一証明）。`new_ncc` plan の `ncc`/`cross_correlation` は
  `new` plan と同一契約の値を返す。
- **退化 shape**: `template_len == 0` または `signal_len < template_len`
  なら `n = 0` の退化 plan となり、全カーブ API は空スライスを返す
  （エラーではない。`signal_len < template_len` では有効ラグ域が空）。
- **max_lag クランプ**: `max_lag > bound` は `bound` へクランプ
  （`usize::MAX` 許容）。戻り長は `min(max_lag, bound) + 1`。
- **NaN/Inf**: 分子・エネルギーとも IEEE どおり伝播。`e > 0` 判定に
  NaN が来た場合は `0.0` を返す（`corr0` と同規則）。証拠フィールドは
  NaN を隠さない（`window_energy[k]` が NaN なら当該窓に NaN が存在）。

### `OverlapCorrPlan`（等長窓・両側ラグ）

- **ラグ意味**: `values[i]` は `lag = i − max_lag` の相関。`lag ≥ 0` は
  `x[..L−lag]` と `y[lag..L]`、`lag < 0` は `x[|lag|..L]` と `y[..L−|lag|]`
  の overlap 区間 `L−|lag|` 標本での非中心化正規化相関（区間内 `corr0`
  と同一式）。`|lag| ≥ L` は overlap なしとして `0.0`。
- **証拠（`OverlapCurve`）**: `values`・`overlap_energy[i]`（そのラグの
  overlap 区間での `Ex·Ey` 積。商を返す条件は各因子 `> 0` で、積 `> 0` と
  同値。`|lag| ≥ L` では `0.0`。NaN は伝播）を返す。有効 support
  （ラグごとの overlap 標本数）は `L−|lag|`（`|lag| < L` のとき）として
  式で契約化し、配列では返さない。
- **workspace 容量**: `OverlapCorrWorkspace::new` は全有効ラグ域
  `2L−1` を確保し任意の `max_lag` を許す。`with_max_lag(plan, max_lag)` は
  `2·max_lag+1` だけを確保し、それを超える `max_lag` の呼出は
  `Err(Error::InvalidValue("max_lag"))`（al-math は `Vec::resize` で呼出
  中に確保していた;本 API では処理呼出しの alloc を構造的に排除するため
  容量を workspace 構築時に移す）。`lag_max ≪ window_len` の用途
  （al-analysis reference は `lag_max=512, lag_window=8192`）では
  バッファが `2L−1`→`2·max_lag+1` へ縮む。
- **direct/FFT ルーティング**: `direct_ops = 3·out_len·L` と
  `fft_ops = 15·n·log2(n)` の仕事量比較で direct が安い場合はラグごとの
  `corr0` 逐次評価、それ以外は FFT 分子 + prefix 二乗和分母。出力契約は
  経路に依らない。
- **退化 shape**: `window_len == 0` は `n = 0` 退化 plan で全 `0.0` の
  `2·max_lag+1` 長カーブ（al-math と同じ）。

### Direct/FFT ルーティング（`NccDirect`）

`new_ncc` は有効ラグ数 `K = S−L+1` が小さい形状で FFT を持たない直接計算
plan を選ぶ（`K ≤ 2048` かつ `L·K ≤ C·n·log2(n)`）。係数 `C` と
OverlapCorr の仕事量係数は al-math 校正値を初期値とし、#152 比較記録で
scalar 実装に再校正する（カーネル両経路とも scalar のため比は保たれるが、
絶対コスト比は再測定で確認する）。選ばれた mode は出力契約に影響しない。

### scratch・メモリ

- `CorrelationWorkspace` は FFT モードで `n` 要素の実数バッファ・
  `n/2+1` bin のスペクトル 2 組・`S+1` prefix・`S−L+1` norms・`n/2+1`
  phat_energy を全確保する。`NccDirect` では FFT バッファを持たず
  `real` は `S−L+1` 出力領域（al-math と同一のメモリ削減形状）。
- `OverlapCorrWorkspace` は `n` 実数バッファ・`n/2+1` bin 2 組・
  prefix 2 本・容量分の `curve`/`overlap_energy` を確保する。
- 借用カーブ（`NccCurve`/`OverlapCurve`/`&[f64]`）は workspace 内
  バッファを指し、借用中は `&mut ws` が排他になるため再呼出し不可
  （al-math と同じ借用セマンティクス）。

## Error Model

`Error::InvalidValue(&'static str)` を継承し、公開 API は panic-free。

- `new`/`new_ncc`/`OverlapCorrPlan::new`: FFT 長の算術 overflow は
  `Err(Error::InvalidValue("length"))`（al-math は `expect` panic）。
- 形状不一致: `template`/`signal`/`x`/`y` が plan の長さと異なる、または
  workspace が plan に対応しない → `Err(Error::InvalidValue("<field>"))`。
- `gcc_phat` を `new_ncc` plan で呼出 → `Err(Error::InvalidValue("mode"))`。
- `curve` の `max_lag` が workspace 容量超過 → `Err(Error::InvalidValue("max_lag"))`。
- one-shot free fn は `Result<Vec<f64>>` を返し同じ検証を伝播する。

## 再利用単位と AudioLens 境界

| 利用側パターン | 使用 API（Ruviform） | AudioLens に残る編成 |
| --- | --- | --- |
| 全長 NCC（`full_ncc`/`coarse_ncc`） | `new_ncc(L,S)` + `ncc` | 探索範囲の切出し・peak 候補選定（`peaks_from_correlation`）・同率最大値の tie-break ポリシー・合議 |
| window NCC（`band_gain`） | `new_ncc(window, window+2·max)` + `ncc`（対称ラグは `lag = k − max` で利用側が解釈）+ `vec::dot`/`sum_sq` の gain 計算 | 窓列挙・hop・端点除外・gain/極性の意味付け・`Window` 構造体・support 区分 |
| band NCC（`band_ncc`） | bandpass（#153 STFT/帯域）後に同じ `new_ncc` + `ncc` | 帯域分割・帯域ごとの呼出編成・候補集約 |
| GCC-PHAT | `new(L,S)` + `gcc_phat` | peak 解釈・スコア化・reliability |
| per-window lag τ(t)（reference `local_lag`） | `OverlapCorrPlan` + `curve` | 窓列挙・hop・lag→時刻換算・幅/score 意味付け・Confidence 合成 |
| plan/workspace 共有 | `CorrelationWorkspace` の明示所有（`&` 共有 plan・`&mut` 排他 ws） | `al-locate` の shape キー `pool`（実行単位で plan+ws を再利用する編成）は AL 側の責務 |

証拠として `window_energy`/`overlap_energy`/`template_energy` を返すため、
利用側は「0.0 が無相関由来か無音由来か」「同率最大値が丸め誤差内の
複数ラグに存在するか（`values` は bit 決定的）」を追加計算なしに検証できる。

## Testing Strategy

- **oracle 検証**: naive `corr0` 逐点 oracle（`corr_at`）・直接内積 NCC
  oracle と Full/Valid/Direct 各経路を比較。`new_ncc` plan と `new` plan の
  NCC が有効ラグ全域で一致すること（`1e-10` 以内、丸め経路差のみ）。
  direct plan は FFT plan/バッファを持たないこと（shape 検査テスト）。
- **境界 fixture**: L=1・L=S（K=1）・max_lag=0・max_lag ≥ 有効域・
  max_lag ≥ L（overlap）・無音 template/signal・NaN 混入・同率最大
  （構造化疑似入力）・`S+L−2` の部分 overlap ラグ（Full raw）を網羅。
- **決定的入力**: al-math テストと同じ LCG 疑似ノイズ + 既知オフセット
  埋込みで argmax 一致性を検証する（同一シード・同一埋込みで AL 側と
  同じピークラグが出る）。
- **shape ルーティング pin**: al-math と同一の 6 代表形状で plan mode
  （NccValid/NccDirect）が一致することを固定（係数再校正時は実測を記録）。
- **doctest**: 公開 API の使用例を rustdoc doctest で検証。
- **RT レーン**: `rt_` テストで `assert_no_alloc` 測定窓内に
  `ncc`/`gcc_phat`/`cross_correlation`/`curve` を置き alloc なしを検証。

## AudioLens 比較記録

比較対象は #150 と同じ **AudioLens Issue #98 凍結 baseline** の
`crates/al-math`（`al98-baseline` worktree）。correlation.rs と vec.rs は
現行 main と `diff -q` で byte-identical を確認済みのため同一測定で
現行カーネル・#98 baseline の双方と突合する。

測定方法（#150 と同じ・両リポジトリ非改変）:

- `%TEMP%/r152-compare` の独立 scratch crate に両クレートを `path` 依存で
  参照し、同一決定入力を両側へ与える。al-math 側は `CorrelationPlan`/
  `OverlapCorrPlan` を plan/workspace で直接呼ぶ（free fn でも内部同一）。
- **精度**: 有効ラグ域全域で `max |a−b|`（SIMD/scalar 評価順差の丸め差。
  ~1e-12 台を期待し、al-math 自体の dispatch↔scalar 差と同程度である
  ことを併記する）。
- **性能**: per-call `Instant` 中央値。形状は al-locate 実形状（band_ncc/
  coarse_ncc/band_gain window/gcc_phat/小窓 overlap）を代表として選ぶ。
- **メモリ/alloc**: counting allocator で plan 構築・ws 構築・呼出し中
  alloc を実測（al-math は TLS FFT workspace の初回確保を含めて記録）。
- **ルーティング再校正**: 両経路（direct/FFT）を同一形状で測り、
  `prefers_direct` 係数と overlap の仕事量係数が scalar 実装でも
  実測最速側を選ぶことを確認・必要なら修正する。

記録の正本は `docs/impl/exp/issue-152-al-math-comparison-20261031.md`
（ハーネスソース・生出力は `data/`）。

## Consequences

- 良い点: al-math `correlation` の全機能（Full/NccValid/NccDirect plan、
  PHAT、overlap、direct ルーティング）が #150 の契約体系へそのまま載る。
  panic を `Result` に置き換え、TLS 暗黙確保を排除し、エネルギー証拠を
  返却面に加えたことで `band_gain` の `sum_sq` 再計算を不要にできる。
  `cross_correlation` の `linear_lag_max` で Full plan の部分 overlap ラグを
  新たに公開し、「必要なら full/partial lag」の要件をカバーする。
- 悪い点: `OverlapCorrWorkspace` に容量パラメータ（`with_max_lag`）が増え
  al-math より API が 1 メソッド増える。SIMD 未導入で FFT 経路の絶対性能は
  al-math 現行（pulp V3）より遅い（#150 比較で定量化済みの退行と同じ）。
- トレードオフ: `gcc_phat` の Full-mode 限定は panic→`Err` 変換で保持。
  `OverlapCorr` の呼出時 `max_lag` は al-math より柔軟性を落とす
  （`new` のみ容量上限を強制）が、確保責務を workspace に集約し
  RT-suitable を維持する。#150 で Deferred とされた f64 SIMD 化は引き続き
  別 Issue（本比較で効果を再計測する）。

## Deferred Design Concerns

- `gcc_phat` の Full-mode 限定を型レベル（plan 種別）で表現するか
  | 利用側から mode 問合せが要ることが判明した時点 | 現状は `Err("mode")`。
- corr カーネルの SIMD 化 | #150 と同じく将来 Issue | scalar 参照を oracle
  にした等価性検証前提。
- 複数 plan/workspace の再利用プール | AudioLens 側責務（AL#101/#102 で
  再配線）| shape キー付きプールは利用側編成であり本クレートに置かない。

## Verification Notes

- [x] `corr` モジュールが `ruviform-numeric` に配置され、crate README・
      crates/README.md・baseline A.2 表の責務記述が更新されている。
- [x] NCC/GCC-PHAT/生相互相関/overlap 相関が panic-free `Result` で
      RT-suitable と明記され、`rt_` テストが alloc なしを検証する。
- [x] scalar 参照・oracle・境界・退化 shape・ルーティング pin テストが
      `mise run test` で緑になる。
- [x] `mise run fmt` `check` `lint` `test` が pass する。
- [x] al-math 同形状入力での correctness・busy time・alloc 比較が
      `docs/impl/exp/issue-152-al-math-comparison-20261031.md` に記録される。

## Comparison Results

実測正本は `docs/impl/exp/issue-152-al-math-comparison-20261031.md`。
要約:

- **精度**: 実運用形状 7 件（band_gain・anchor・band/coarse_ncc・tiny）
  で NCC/GCC-PHAT/overlap とも max |diff| ≤ 1.1e-16、argmax 全一致。
  PHAT は既知埋込み lag に両実装とも真値ピーク。SIMD/scalar の丸め
  評価順差のみで、al-math 自身の dispatch↔scalar 差と同程度。
- **性能**（scalar vs pulp V3、per-call 中央値・plan+ws 再利用）:
  FFT 経路 NCC 1.29〜1.36 倍・PHAT 1.39〜1.47 倍・overlap 1.24〜1.47 倍
  （rf/al）。direct 経路 NCC は 1.51〜2.09 倍で SIMD axpy が最も効く
  箇所。#150 の FFT 単体差より小さく、相関呼出しは非 FFT 処理
  （prefix 二乗和・正規化・白化）が支配的なため。
- **メモリ/alloc**: 処理呼出しは両実装とも 0 alloc。ws は全形状で
  ruviform が小さい（52.9MB→17.6MB の direct 形状など。
  `template_squares`/`signal_squares` 削除 + `phat_energy` の
  Full 限定化の効果）。
- **ルーティング再校正**: C_eff=8.0 境界形状で t_direct≈t_fft
  （35.6 vs 36.9µs）— al-math 校正値 `C=8` が scalar でも交差位置と
  一致したため継承を維持。`NccDirect` 判定・overlap 係数ともに
  変更なし。
