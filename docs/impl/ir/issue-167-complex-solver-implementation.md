---
id: "IMPL-034R167CmplxSlvImp1XyZ9"
type: "impl"
status: "completed"
title: "Issue #167 complex_solve (複素線形ソルバ) implementation"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/167"
    - "https://github.com/scarecrow173/audio-lens/issues/102"
  changes:
    modified:
      - "crates/ruviform-numeric/Cargo.toml"
      - "crates/ruviform-numeric/README.md"
      - "crates/ruviform-numeric/src/lib.rs"
      - "crates/ruviform-numeric/src/linalg.rs"
      - "docs/impl/ir/README.md"
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #167 複素線形ソルバ API（complex_solve）— 実装と検証記録

## Summary

audio-lens #102 の R1 契約提案に基づき、`al-analysis` の
`expensive::channel::csolve`（複素 Gaussian 消去 + 部分 pivot +
条件数 proxy）を `ruviform-numeric::linalg::complex_solve` として
移植した。AL 側のローカル二重実装を将来的に除去できるよう、pivot
選択・消去順序・失敗判定を移植元と同一に保ち、同一入力に対して同一の
解・条件数 proxy・失敗（`None`）を返す（bit-exact 置換可能）。

## 実装内容

- `linalg::complex_solve(mat, rhs) -> Option<(Vec<(f64, f64)>, f64)>`:
  正方複素連立一次方程式 `A·w = b` を解く。複素値は `(re, im)` タプル
  （`csolve` と同じ表現・`fft::cmul1` が内部で使う表現でもある）。
- アルゴリズムは `csolve` の逐語移植:
  - 拡大係数行列 `[A | b]` を構築し、列ごとに
    `max_by(|i, j| cabs(aug[i][col]).total_cmp(&cabs(aug[rj][col])))`
    で部分 pivot 選択（同値最大は `max_by` の仕様どおり最後の index
    — 移植元と同じ tie-break）。
  - pivot 絶対値 `< 1e-30` を特異・数値的縮退として `None`。
  - 消去は `aug[row][c] -= factor·aug[col][c]`（`c in col..=dim`、
    pivot 行正規化なし）→ 後退代入で `aug[row][row]` 除算。
  - 複素演算は `cabs = sqrt(re²+im²)`、`cdiv = x·conj(y)/|y|²`、
    `cmul = (xr·yr − xi·yi, xr·yi + xi·yr)` の非融合 `a*b±c*d` 形で
    評価順を固定（`fft::cmul1`・crate の bit-exact 契約と同じ流儀）。
  - 条件数 proxy = 消過 pivot 絶対値の最大/最小比（`piv_min <= 0` なら
    `INFINITY`）。スペクトル条件数 `condition_number` とは別物の診断値。
- 失敗判定は `csolve` と同一のみ: `n == 0`・非正方/ragged・
  `rhs` 長不一致・pivot `< 1e-30`。**非有限入力は拒否しない**
  （NaN/Inf は IEEE どおり解へ伝播し `Some` を返す）— `csolve` が
  有限性検査を持たないため、同一の失敗判定を維持する意図的な契約。
  `least_squares` 等の `None` 拒否とはこの点で異なり、
  `linalg` モジュール doc・`lib.rs` 精度契約に例外として明記した。
- 依存追加なし（std のみ・`num-complex` 不採用 — workspace の Cargo
  マニフェストに同依存は存在せず、`csolve` 自体も `(f64, f64)` タプル
  表現のため移植がそのまま成立する）。RT 契約は確保を伴うため
  **non-RT only**。
- `clippy::many_single_char_names`（workspace pedantic）対応で変数名は
  意味名に置換（`a`→`mat`、`b`→`rhs`、`m`→`aug` 等）。算術式・評価
  順序は移植元と同一のため bit-exact 性に影響しない。

## 検証（全て実実行・pass）

| コマンド | 結果 |
| --- | --- |
| `mise run fmt` | pass（差分なし） |
| `mise run check` | pass |
| `mise run lint` | pass（`clippy --all-targets -D warnings`） |
| `mise run test` | pass（全ワークスペース 86 スイート・0 失敗） |
| `mise run build` | pass |

追加テスト（`linalg::tests` + doctest、計 7 件）:

- `complex_solve` doctest（対角系・既知解・cond=2.0）
- `complex_solve_recovers_known_solution_with_small_residual` —
  良条件 3x3 系で既知 w から独立に `b = A·w` を算定し、解一致 +
  残差 `||b − A·x|| < 1e-12`（csolve と同じ数学的結果を返す検証）
- `complex_solve_partial_pivot_avoids_tiny_column_entry` — pivot swap
  で 1e-40 の先頭候補を回避し求解 / 全候補 `< 1e-30` なら `None`
- `complex_solve_uses_complex_division_in_elimination` — `(1+i)·w = 2`
  → `w = 1−i`（複素除算・後退代入経路）
- `complex_solve_condition_proxy_is_pivot_magnitude_ratio` —
  対角系で proxy = pivot 絶対値比（1e10/1e-20 = 1e30）
- `complex_solve_singular_and_invalid_shapes_are_none` — ランク落ち・
  `n==0`・非正方・ragged・rhs 長不一致が `None`
- `complex_solve_propagates_non_finite_like_csolve` — NaN 入力が
  `None` ではなく非有限解を含む `Some` を返す契約の固定

最終検証前に `git fetch origin main && git merge origin/main` を実施
（Already up to date）。`cargo generate-lockfile` は無関係な 30 件の
バージョン上書きを生じたため `git checkout -- Cargo.lock` で復元
（依存追加なしのため lockfile 再生成トリガ非該当）。

## 受け入れ条件との対応

| 要件 | 対応 |
| --- | --- |
| AL `csolve` 相当を `linalg` の複素版として提供 | `complex_solve` を `linalg` に追加。`csolve(&a, &b)` と同じ形状の `&[Vec<(f64,f64)>]` 入力で AL 側置換を直結できる |
| 同一入力で同一の解・失敗判定 | pivot 探索式・消去/代入順序・閾値・非有限伝播を逐語移植し、コメントで対応関係を明記 |
| 単体テスト | 上表 7 件（既知解・特異/劣条件・非正方・非有限） |
| `fmt`/`check`/`lint`/`test`/`build` 全 pass | 上表どおり全て実実行・pass |

## 未解決・既知制限

1. **bit-exact 性の検証は構造的一致による** — AL 側との直接比較実行は
   別リポジトリのため未実施。同一式・同一順序の逐語移植と
   `fft::cmul1` 系の bit-exact 契約に拠る。AL 側移行時に実値での
   差分確認を推奨。
2. **条件数 proxy は概算値** — pivot 比は真のスペクトル条件数ではない
   （`csolve` 同様、呼出側が `cond > 1e3` で ill-conditioned 判定する
   診断用）。必要なら `condition_number`（AᴴA 版）が別途必要。
3. **複素最小二乗（非正方）の直接 API は未提供** — Issue の用途は AL
   が外部で正規方程式を構成する正方ソルバであり、本 Issue では
   `csolve` 等価物の提供に留めた（YAGNI）。非正方の complex
   `LeastSquares` 相当が必要になった場合は別 Issue。
4. **split re/im planar 形式の API は未提供** — FFT 系の複素規約とは
   別に、`csolve` との表現一致（`(re, im)` タプル）を優先した。
   planar 変換は利用側の責務。
