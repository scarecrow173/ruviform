---
id: "IMPL-034R169SpecStatsPrimXy"
type: "impl"
status: "completed"
title: "Issue #169 basic spectral statistics primitives"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/169"
    - "https://github.com/scarecrow173/audio-lens/issues/102"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-numeric/src/spec.rs"
      - type: "file"
        path: "docs/impl/ir/issue-169-spectral-statistics-primitives.md"
    modified:
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

# Issue #169 basic spectral statistics primitives

## Summary

audio-lens #102（R3 契約提案）が要求する基礎スペクトル統計を
`ruviform_numeric::spec` に追加する。AL 側 `al-analysis::cheap` の
`measure_spectral` / `measure_transient` に inline で書かれていた
スペクトル形状統計（重心・帯域幅・ロールオフ・平坦度・エントロピー・
フラックス）と汎用統計（OLS 傾き・中央値・argmax 選択）を、AL と
同じ逐次評価順・縮退規則を持つ用途非依存 `f64` プリミティブとして
提供し、AL が将来 bit-exact に置換できる形にする。

## Context

- Source: ruviform issue #169 / audio-lens issue #102（R3 契約提案）。
- AL 側参照実装（読み取りのみ、変更禁止）:
  - `al-analysis/src/cheap.rs` `measure_spectral` — per-frame
    スペクトル形状軌跡（`spec.centroid_hz`・`spec.bandwidth_hz`・
    `spec.rolloff_hz`・`spec.flatness`・`spec.entropy`・`spec.flux`）、
    `spec.slope` の log-log OLS 傾き、narrowband peak/notch の
    近傍中央値と tonal peak の argmax。
  - `al-analysis/src/cheap.rs` `measure_transient` — onset 判定の
    局所中央値（`nb[nb.len()/2]` パターン）。
  - `al-analysis/src/cheap.rs` `ipd_block_lag` — NCC 曲線の argmax
    （`.enumerate().max_by(total_cmp)` パターン）。
- 配置: `crates/ruviform-numeric`（Primitives 最下位・std のみ）。
  2 信号間スペクトル統計を持つ `xspec`（Issue #153）と同じ層に、
  単信号スペクトル形状統計モジュール `spec` を新設した。

## Implementation

### Changes

- `crates/ruviform-numeric/src/spec.rs`（新規）
  - モジュール doc: AL verbatim の縮退規則・NaN 応答の非対称・
    argmax tie 規則・評価順契約を明文化。
  - `centroid(mag, bin_hz) -> f64` — 振幅重み付き重心
    `Σ(k·bin_hz)·mag[k] / Σmag[k]`。`total > 0.0` でなければ `0.0`。
  - `bandwidth(mag, bin_hz) -> f64` — 重心まわり振幅重み付き標準偏差
    `sqrt(Σ mag[k]·(f(k)−c)² / Σmag[k])`。`total <= 0.0` なら `0.0`
    （否定形判定のため NaN total は IEEE 伝播）。
  - `rolloff(mag, bin_hz, fraction) -> f64` — 累積振幅が
    `fraction·total` に初めて達する bin の周波数。未到達・NaN total
    では bin 0（`0.0`）。fraction は引数化（AL は 0.85 固定呼出）。
  - `flatness(mag) -> f64` — `exp(Σln(mag+1e-12)/n) / (Σmag/n)`。
    `total > 0.0` でなければ `0.0`。
  - `entropy(mag) -> f64` — `−Σ_{mag>0} p·ln(p) / ln(n)`。
    `total > 0.0` でなければ `0.0`。`n=1` かつ `total > 0.0` は
    `ln(1)=0` 除算で `NaN`（AL verbatim）。
  - `flux(curr, prev) -> Result<f64>` — `Σ max(0, curr−prev)`。
    長さ不一致は `Err("length")`（AL の zip 黙認を本クレート規約の
    検証に置換）。先頭フレーム `0.0` ポリシーは呼出側。
  - `ols_slope(xs, ys) -> Result<f64>` — `(n·Σxy − Σx·Σy)/(n·Σx² −
    (Σx)²)`。分母 `0.0` なら `0.0`（NaN は判定を通過して伝播）。
    長さ不一致は `Err("length")`。`1..(half*9/10).max(2)` の bin 域
    選択と `ln` 変換は呼出側ポリシーとして残す。
  - `median(x) -> Option<f64>` — `total_cmp` 整列済みコピーの
    `sorted[n/2]`（上位中央値）。空は `None`。確保するため non-RT。
  - `argmax(x) -> Option<(usize, f64)>` — `Iterator::max_by` +
    `total_cmp`。同率最大は最後（最大 index）、NaN は最大扱い。
    空は `None`。対象範囲の絞り込み（DC 除外・上限帯域）は呼出側。
  - 契約テスト 10 件追加（`spec::tests`）。AL inline 式を verbatim
    で写した独立 oracle との `to_bits` 比較を含む。
- `crates/ruviform-numeric/src/lib.rs`
  - `pub mod spec` 追加・クレート doc の Issue 参照更新。
  - `rt_spectral_paths_do_not_allocate` に `spec` の RT-suitable 8 関数
    を `assert_no_alloc` 配下で追加（`median` は確保するため除外）。
- `crates/ruviform-numeric/README.md`
  - `spec` モジュールの責務記述と Issue #169 を追記。

### Normalization 契約（AL 実装との突き合わせ）

| 要素 | `spec` API | AL `cheap.rs` 逐次式 | 差分 |
| --- | --- | --- | --- |
| 重心 | `total > 0.0` → `Σ(k·bin_hz)·v / total`、他 `0.0` | `measure_spectral` per-frame loop と同一 | なし（verbatim） |
| 帯域幅 | `total <= 0.0` → `0.0`、else `sqrt(Σv·(f−c)²/total)` | 同一（2 パス目で total を再計算） | なし（verbatim） |
| ロールオフ | `acc += v; acc >= fraction·total` の最初の bin | `0.85` を `fraction` 引数化（`total·0.85` と同値） | 引数化のみ |
| 平坦度 | `exp(Σln(v+1e-12)/n) / (total/n)`、`total>0.0` ゲート | 同一 | なし（verbatim） |
| エントロピー | `−Σ_{v>0} p·ln(p) / ln(n)`、`total>0.0` ゲート | 同一 | なし（verbatim） |
| フラックス | `Σ max(0, curr−prev)`、長さ不一致 `Err` | `zip` 等長前提 | 検証を追加 |
| OLS 傾き | `(n·Σxy−Σx·Σy)/(n·Σx²−(Σx)²)`、分母 0 → `0.0` | `spec.slope` の fold と同一 | bin 域選択は呼出側 |
| 中央値 | `total_cmp` 整列コピーの `sorted[n/2]`、空は `None` | `nb.sort_by(total_cmp); nb[n/2]`（非空前提） | 空契約を追加 |
| argmax | `max_by(total_cmp)` → `(index, value)`、同率は最後 | 同一（AL は `unwrap_or` で None を 0 写像） | None 契約を追加 |

### 端点・ゼロ分散・空入力・NaN の扱い

- 空入力: 全スペクトル統計は `total = 0.0` 経由で `0.0`。`flux` /
  `ols_slope` は空同士で `0.0`。`median` / `argmax` は `None`。
- 全零（無音）: `total = 0.0` → `0.0`（rolloff は bin 0 経由で `0.0`）。
- NaN total: `centroid` / `flatness` / `entropy` は `total > 0.0` 肯定形
  ゲートで `0.0` に写像。`bandwidth` のみ `total <= 0.0` 否定形を
  通過して `NaN` を返す（AL の非対称を維持）。`rolloff` は
  `acc >= target` 不成立で bin 0。`ols_slope` は NaN 分母が `== 0.0`
  判定を抜けて除算へ進み `NaN` を伝播。
- `flux` の NaN 差分は `f64::max(0.0)`（maxNum）で `0.0` に写像。
- `entropy` は `n = 1` かつ `total > 0.0` で `ln(1) = 0` 除算 `NaN`
  （AL verbatim）。
- argmax 同率: `Iterator::max_by` は最後の最大要素を返すため最大
  index が選ばれる（AL の tonal peak / AM bin 選択と同一）。

### Decisions During Implementation

調査で確定した事実と、それに基づく設計判断:

- AL の rolloff コメントは "85% energy rolloff" だが累積するのは
  マグニチュード行（`mag[..half]`）であり、エネルギー（二乗）では
  ない。実装の契約を維持するため振幅累積として移植し、`fraction`
  を引数化した（`0.85` をハードコードすると AL 固有情報が汎用
  プリミティブへ漏れる）。
- `bandwidth` は AL が 2 パス目で `total` を再計算する形を verbatim
  で維持した。`centroid()` 内部関数を再利用すると NaN total で
  `c = 0.0` に写像した後も結局 `NaN` を返すため結果は同じだが、
  移植境界を単純に保つため逐次式のままとした。
- `median` は AL が Vec コピーを整列するパターンと同じく内部確保とし
  non-RT とした。scratch 注入形（`_into`）は現在の消費者（AL の
  offline 解析）が要求しないため YAGNI で見送った。
- フレーム走査・`prev` 保持・`mean_row` 蓄積・`spec_rec` の
  robust_summary 記録変換は AL 側の責務（解析意味論）であり、
  本 Issue の移行対象は測定値を返す純粋統計カーネルのみ。

## Related Experiments

### Adopted

（なし — AL 実装の verbatim 移植が目的であり Experiment Log は作らない）

### Rejected

（なし）

## Validation

- `mise run fmt` — pass（ソース整形済み）。
- `mise run check` — pass。
- `mise run lint` — pass（`cargo clippy --workspace --all-targets -- -D warnings`）。
- `mise run test` — `ruviform-numeric` 全テスト pass（`spec::tests` 10 件
  含む）。ワークスペース全体では `ruviform-plugin-host` の
  `l1_proxy_e2e::rejected_block_replies_resolve_pass_through` が
  1 回だけ timeout 失敗したが、単独再実行では pass（IPC タイミング
  依存の既知 flaky で本変更と無関係）。
- `mise run build` — pass。
- `mise run fmt-check` — pass。
- 契約テスト: `spectral_stats_match_al_oracle_bit_exact` が AL inline
  式の独立 oracle と全統計値で `to_bits` 一致（退化・単要素・空・
  擬似スペクトルの 6 系列）。argmax tie・NaN 非対称・`n=1` entropy
  NaN・median 上位中央値の個別契約テストも pass。

## Risks

- AL verbatim 移植のため AL 式の非対称（`bandwidth` の NaN 伝播・`entropy` の `n=1` で `ln(1)=0` 除算 NaN・argmax 同率は最後）を意図的に維持 — consumer 側で「修正」すると bit-exact 互換が崩れる
- `flux`/`ols_slope` は AL の `zip` 黙認から `Err("length")` 検証へ意図的に変更 — AL 側で等長前提が破れた入力を渡した場合の挙動は本クレート規約側が優先される
- `median` は内部確保の non-RT — 将来 RT 経路で必要になれば scratch 注入形（`_into`）の追加が必要（現時点は消費者が offline 解析のみのため YAGNI で未実装）
- `spec.slope` の bin 域選択（`1..(half*9/10).max(2)`）・`prev` 保持・`mean_row` 蓄積・`spec_rec` 変換は呼出側ポリシーとして残っており、誤用は consumer 側の責務

## Follow-ups

- AL 側（audio-lens #102 R3 契約）での bit-exact 置換は consumer 側タスク — `spec` API が移行先
- `median` の scratch 注入形（`_into`）は需要が立証された時点で再評価
