---
id: "DESIGN-034JPvz9ieX8KoPbwDcvaX"
type: "design"
status: "approved"
title: "dispatch 固定コスト分離の計測設計"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/dispatched-overhead-analysis.md"
    - "../adr/dsp-simd-backend-adoption.md"
  derived-by:
    - "../plans/implement-issue-56-dispatch-analysis.md"

  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# dispatch 固定コスト分離の計測設計

## Overview

Issue #56 の計測方法論。`fearless_dispatched`（`dispatch!(Level::new(), …)` per-call 経路）の軽量カーネル劣後を「per-call 固定コスト」と「計測ノイズ」に分離するため、**直接プローブ計測**と **N 依存差分解析**の2系統の独立した証拠を取得する。

## Architecture

### 観測対象の分解

`fearless_dispatched` の1呼出しは `Level::new()`（x86+std では `LazyLock` キャッシュ済み検出値の atomic 読出し）+ `Level::__dispatch_target`（Option/match による正規化）+ arm 呼出しに分解される。いずれも呼出しあたり定数コストであり、カーネル本体時間 `t_k(N)` とは加算関係にある:

```text
t_dispatched(N) = t_kernel(N) + C_dispatch
t_forced(N)     = t_kernel(N) + C_forced   （≈0, `Level::new().as_avx2()` token 経由でも同程度）

→ Δ(N) = t_dispatched(N) − t_forced(N) = C（N に依らない定数）
```

### 系統 A: プローブ直接計測

criterion ベンチに `dispatch_probe` グループを追加し、以下を直接測定する:

| case | 内容 | 意味 |
|---|---|---|
| `level_new` | `black_box(Level::new())` | キャッシュ済み検出の読出しコスト |
| `dispatch_noop` | `dispatch!(Level::new(), simd => black_box(simd))` | per-call dispatch 全経路（検出読出し + 正規化 + arm） |
| `token_avx2` | `black_box(Level::new().as_avx2())` | 強制トークン生成の基準コスト（比較用） |

`dispatch_noop` の値が per-call 固定コスト C の直接推定となる。

### 系統 B: N 依存差分解析

elementwise 軽量カーネル（`add`・`scale`・`clamp_unit`・`fade`）で `fearless_dispatched` と `fearless_avx2` を既存 LENS（64/256/1024/4096/100/1000/1023）で再計測し、Δ(N) = dispatched − avx2 の median 差分を求める。

- Δ(N) がほぼ定数で系統 A の C と一致 → **per-call 固定コスト支配**（比率劣後は小 N で構造的に発生、N↑ で回復）
- Δ(N) が非定数・符号不安定 → **ノイズ支配**
- 定数成分 + 散乱が混在 → **混在**（定数成分は C、残差は環境変動）

加えて小サイズ帯（16/32）をプローブベンチ側の `add` カーネルで補足計測し、固定コスト支配域での比率悪化を確認する。

### ノイズ評価

同一条件で複数 run（≥3）を実施し、median_ns の run 間 |Δ%| 分布（p50/p90）を前回値（p50 1.8% / p90 25.7%、並行負荷あり）と比較する。計測時の環境負荷（プロセス数・CPU 負荷）を記録する。

## Data and State

- 生データ: `docs/impl/exp/data/dispatched-overhead-<date>-NN.jsonl`（1 行 = 1 計測点、PoC 形式踏襲）
- レポート: `docs/impl/exp/dispatched-overhead-<date>.md`（環境メタデータ・分散・判定表・結論・#53 示唆）

## Interfaces and Contracts

- ベンチ追加は `crates/ruviform-math/benches/simd_poc.rs` の `dispatch_probe` グループのみ（既存 matrix ・等価性テストは不変）
- プローブは `fearless_simd::{Level, dispatch}` の公開 API のみ使用（`unsafe` 不使用・production API と同一経路）

## Error Handling

- プローブ測定で `dispatch_noop ≈ 0`（最適化で消去）の場合: `black_box` 境界と戻り値消費を確認して再測定、消去不可能なら Δ(N) フィットのみで推定
- 低負荷環境が確保できない場合: 計測を中断し環境条件を記録して延期（高負荷データを正規結果として採用しない）

## Verification Notes

- [ ] `dispatch_probe` ベンチがビルド・実行される
- [ ] 複数 run の分散統計が計算されている
- [ ] 判定（固定コスト/ノイズ/混在）が2系統の証拠で支持されている
