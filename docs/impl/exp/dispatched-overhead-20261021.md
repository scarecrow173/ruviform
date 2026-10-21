# fearless_dispatched 軽量カーネル劣後の原因分離 — 低負荷再計測レポート（2026-10-21）

Issue [#56](https://github.com/scarecrow173/ruviform/issues/56) / task T2-T3（`docs/tasks/run-dispatch-remeasure.md`・`docs/tasks/write-dispatch-analysis-report.md`）の計測実績。採否 ADR（`docs/adr/dsp-simd-backend-adoption.md`）決定 2・検証表の未解決項目「劣後が per-call dispatch コストかノイズかの分離」への回答であり、dispatch 層設計 Issue（#53）の判断材料。
生データ: [`data/dispatched-overhead-20261021-01.jsonl`](data/dispatched-overhead-20261021-01.jsonl)〜[`03.jsonl`](data/dispatched-overhead-20261021-03.jsonl)（1 行 = 1 計測点、261 行/run × 3 run）。

## 環境メタデータ

- OS: Windows 11 x64 / CPU: Intel Core i9-9900K @ 3.60GHz / 16 コア（HT あり）
- rustc 1.99.0 / criterion 0.8.2
- 負荷状況（前回との差異が本 Issue の核心）: 計測中の OS CPU 負荷 9–20%・プロセス数 ~390（常駐プロセスは多いが実行中のエージェント/重量ジョブは本セッションのみ）。前回計測（PoC レポート `dsp-simd-poc-20261021.md`）は ~350 プロセスで**並行エージェント多数が実働**・コミット上限逼迫（分割実行を強制）する高負荷条件だった
- 計測列: 全9列（`scalar`, `fearless_fallback`, `fearless_sse4_2`, `fearless_avx2`, `fearless_dispatched`, `pulp_scalar`, `pulp_v3`, `pulp_dispatched`, `multiversion`）を維持 — 前回と同一方法論で直接比較可能
- 計測ケース: 261/run（elementwise 4 カーネル × 9 列 × 7 長 + `dispatch_probe` 9 case）。時間設定は PoC 既定（warm-up 300ms / measurement 1s / sample 20）
- `dispatch_probe` は本 Issue で追加した専用グループ（`crates/ruviform-math/benches/simd_poc.rs`）: `level_new`・`token_avx2`・`dispatch_noop` の部品プローブ + `add` の小サイズ帯（N=16/32、3 列）

## 実施記録

- 各 run = `dispatch_probe` + `add`/`scale`/`clamp_unit`/`fade` のカーネル別フィルタ実行（criterion フィルタによる部分実行、estimates 産物は全量実行と同一形式）
- 3 run 完走・クラッシュなし。所要 ~6.5 分/run（対象ケース 261 のみ）
- 生データは criterion `estimates.json` を `tmp/bench56/collect.py` で jsonl 化（group/column/input_len パス分解）

## 系統 A: per-call 素コストの直接計測（dispatch_probe）

| case | run1 | run2 | run3 | 意味 |
|---|---|---|---|---|
| `level_new` | 0.433ns | 0.434ns | 0.435ns | `Level::new()` — `LazyLock` キャッシュ済み検出値の読出し。実質無料 |
| `token_avx2` | 0.433ns | 0.433ns | 0.430ns | `Level::new().as_avx2()` — 強制トークン生成。同上 |
| `dispatch_noop` | **1.291ns** | **1.292ns** | **1.296ns** | `dispatch!(Level::new(), simd => …)` no-op — per-call dispatch 全経路 |

**`dispatch!` の per-call 固定コスト ≈ 1.29ns**（3 run で 0.005ns 以内に安定・CI95 ±0.01ns）。`Level::new()` 単独は測定分解能以下で、`__dispatch_target` 正規化 + arm 選択呼出しが実質的なコスト本体。

## 系統 B: Δ(N) = dispatched − forced_avx2 の N 依存解析

`add` の小サイズ帯（3-run median、単位 ns）:

| N | avx2 | dispatched | Δ | avx2/disp |
|---|------|-----------|---|-----------|
| 16 | 6.14 | 12.85 | **+6.71** | 0.48 |
| 32 | 7.03 | 13.34 | **+6.31** | 0.53 |
| 64 | 8.67 | 15.39 | **+6.72** | 0.56 |
| 100 | 12.75 | 19.86 | **+7.11** | 0.64 |
| 256 | 17.97 | 21.71 | +3.74 | 0.83 |
| 1000 | 57.32 | 59.14 | +1.91 | 0.97 |
| 1024 | 61.98 | 67.93 | +4.83 | 0.91 |
| 4096 | 451.46 | 476.71 | +25.25 | 0.95 |

`add` の Δ は小 N で **6–7ns と安定**（N=16 で 3 run それぞれ 6.72/6.71/6.83ns — ほぼ一致）。これはプローブ値 1.29ns より大きいが、dispatched 経路は `#[target_feature]` シャム経由の非インライン呼出し + 引数受け渡しを含むため、実カーネル経路の固定コストは数 ns 規模と整合する。

他カーネルの小 N（N=64）Δ: `scale` +2.2ns（0.75x）・`clamp_unit` +1.1ns（0.90x）・`fade` +2.0ns（0.86x）— いずれも固定コスト支配域。

**大サイズ帯（N≥1023）の Δ は符号不定の大散乱**: 例 `add/1023` = +3.3 / −20.1 / +34.7ns（run 1/2/3）、`scale/4096` = +17.7 / +77.4 / −48.8ns。メモリ帯域律速域では ±10–30% の環境ジッターが絶対差に拡大写像され、run 間で符号が反転する — **ノイズ支配**。

## 分散（run 間再現性）

- median_ns の run 間 |Δ%|（3 run・min-max 幅/中央値、261 セル）: p50 = 4.0%・p90 = 30.7%・p99 = 91.9%・max = 132.7%
- 前回（高負荷）: p50 1.8% / p90 25.7% / p99 237.7% — p99 の極端偏位は改善したが、分散は依然有意。**この環境の「低負荷」でも背景 CPU ~10-20% + Windows スケジューリングで大サイズ帯はなお数 %〜数十 % 揺れる**ことを確認した
- したがって分散緩和に頼らず「小 N で安定した Δ（固定コスト）」と「大 N で符号不定な Δ（ノイズ）」の構造的分離で結論を導く

## 判定

**混在 — per-call 固定コストは実在し、小サイズ帯の劣後を説明する。大サイズ帯（@1024）で観測された劣後はノイズだった。**

根拠（2 系統の一致）:

1. プローブ直接計測: `dispatch_noop` = 1.29ns が 3 run で 0.4% 以内に安定 — 固定コストは実在し測定可能
2. N 依存解析: 小 N（16–256）で Δ が正・安定（add で ~6–7ns、run 間 ±0.1–1.5ns）→ 固定コスト支配。N≥1023 で Δ が符号反転する大散乱 → 前回の @1024 での add 0.62x・scale 0.66x は当時の高負荷環境由来のノイズであり、低負荷では同地点 0.91–0.97x に回復

前回レポートの「dispatched 小サイズ回帰（0.58–0.84x）」は本実測でも再現（今回の小 N: add 0.48–0.83x）し**実在**だが、「@1024 で強制 avx2 に劣後」は**再現せず**（0.91–0.97x）— 後者はノイズと確定。

## Issue #53 への設計示唆

- **prepare 固定・トークン繰り上げは正当**: per-call ~1.3–7ns の固定コストを構造的に除去できる。オーディオブロック典型サイズ（64–512 フレーム）× 軽量カーネルでは固定コストが総時間の 10–80% を占め得るため、除去価値が大きい
- **カーネル/サイズ別ルーティングも併用が正当**: 小 N の軽量カーネルは forced avx2 自体が scalar とほぼ同値（add/64: avx2 8.67ns vs scalar 8.95ns）のため、トークン化しても小 N の elementwise はスカラー経路へのルーティングが合理的（決定 2 条件 (a) と整合）
- **dispatched 経路は大サイズで劣化しない**: @1024+ での劣後観測はノイズ確定のため、#53 で「大サイズでも dispatched は安全」とは言えない…ではなく「per-call コスト以外の dispatched 固有の構造的欠陥は未観測」。prepare 固定の根拠はあくまで小サイズ帯の固定コスト除去に限定される

## 採否 ADR 検証項目への回答

> `fearless_dispatched` の軽量カーネル劣後が per-call dispatch コストかノイズかの分離（低負荷環境での再計測）

**回答: per-call dispatch コスト（小サイズ帯の実在する固定コスト ≈1.3ns・カーネル経路では数 ns）が劣後の原因。大サイズ帯で観測されていた劣後は高負荷環境のノイズだった。** 決定 2 条件 (b) の prepare 固定方針は、この実測固定コストによって引き続き正当化される。
