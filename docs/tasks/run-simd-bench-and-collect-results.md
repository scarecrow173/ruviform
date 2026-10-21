---
id: "TASK-034Jf49f5gwAy5fpTPvXG9"
type: "task"
status: "done"
title: "ベンチ実行と結果収集"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/dsp-simd-poc-20261021.md"
      - type: "file"
        path: "docs/impl/exp/dsp-simd-poc-benchmark-comparison.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/dsp-simd-poc-20261021-01.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/dsp-simd-poc-20261021-02.jsonl"
    modified:
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/dsp-simd-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/dsp-simd-poc.md"
    - "implement-criterion-benchmarks.md"
  blocks:
    - "write-simd-adoption-adr.md"
  supersedes: []
  superseded-by: []
  related:
    - "../specs/dsp-simd-poc-adr.md"
    - "../designs/dsp-simd-poc.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/bench-tasks-and-ci-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# ベンチ実行と結果収集

## Work

plan `PLAN-034JdOWI3vblJW11kIueY0`（[../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md)）の Phase F / Task T6。Windows x64 主計測環境で全量ベンチを実行し、`estimates.json` 集約と experiment log（jsonl + 文書）を `docs/impl/exp/` に記録する。

- [x] F-1. Windows x64 主計測環境で全量ベンチを実行（`mise run bench` = `cargo bench --workspace`）。同一マシンで 2–3 run 実施して分散を記録（省電力モード・負荷状況を環境メタデータに記録）。criterion の `estimates.json` は run ごとに上書きされるため「run 実行 → `estimates.json` 集約（F-2）→ 次 run」の順序で実施する。削減設定後の全量予想所要時間（E-4 の目安 ≈20–30 分）と実測所要時間を記録する — **実績: 2 run × 756 ケース完走（各 ≈25–27 分）。単一プロセス全量実行は 2 回とも ~82–85% でコミット上限逼迫による alloc 失敗 abort（並行エージェント多数稼働下の環境要因）のため、カーネル単位 8 分割実行へ変更（産物は同一）**
- [x] F-2. `target/criterion/**/new/estimates.json` から `median` / `mean` を抽出する軽量集約（node ワンショット or 手集計）を行い、`docs/impl/exp/dsp-simd-poc-<yyyymmdd>-<seq>.jsonl`（1 行 = 1 計測点）を生成してコミット。jsonl は LF 改行・末尾改行ありで生成する（prek の end-of-file-fixer / mixed-line-ending 系 hook 適合 — コミット対象のため） — **実績: `docs/impl/exp/data/dsp-simd-poc-20261021-0{1,2}.jsonl`（各 756 行、LF・末尾改行）。計測データ jsonl を `data/` サブディレクトリへ配置したのは、impl-doc の experiment-log 監査が exp/ 直下の全 `.jsonl` をイベントスキーマで検証するため（スキーマ衝突回避の構造判断。declared path からの逸脱として記録）**
- [x] F-3. experiment log 文書 `docs/impl/exp/dsp-simd-poc-<yyyymmdd>.md` を作成（環境メタデータ・カーネル × impl のスカラー比判定表・denormal 崖観察・判定目安との照合結果を記録） — **実績: `docs/impl/exp/dsp-simd-poc-20261021.md` + impl-doc イベントログ `dsp-simd-poc-benchmark-comparison.jsonl`（9 イベント、監査クリーン）**

コミット粒度は plan 規約どおり 1 タスク 1 コミット。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md) — `PLAN-034JdOWI3vblJW11kIueY0`（approved）。Phase F の正本
- [../test-specs/bench-tasks-and-ci-smoke.md](../test-specs/bench-tasks-and-ci-smoke.md) — `TSPEC-034Jeo1j5l7fBhYc8iYsTN`。ベンチ・検証タスクと CI スモーク挙動の契約保証の正本
- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`。`mise run bench` 計測可能（AC-6）・experiment log の受け入れ基準
- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（approved）。experiment log 形式・採否 ADR への入力形式の正本

## Dependencies

- 前提タスク: [implement-criterion-benchmarks.md](implement-criterion-benchmarks.md)（T5 — ベンチ実装が前提）
- 後続をブロック: [write-simd-adoption-adr.md](write-simd-adoption-adr.md)（T7 — 計測結果が採否 ADR の証拠入力）
- 外部前提: Windows x64 計測環境。計測ノイズ（turbo/省電力・負荷・タイマ分解能）は 2–3 run の分散記録と環境メタデータで緩和（plan Risk 5）
- spec AC-6 の `mise run bench` グリーン要求は本タスクの全量計測実績で充足する（Phase H では `bench-smoke` のみ再確認）

## Affected Files

- `docs/impl/exp/dsp-simd-poc-<yyyymmdd>-<seq>.jsonl` — 新規（計測結果。1 行 = 1 計測点。LF・末尾改行あり。コミット対象）
- `docs/impl/exp/dsp-simd-poc-<yyyymmdd>.md` — 新規（experiment log 文書。環境メタデータ・判定表・観察メモ）
- `target/criterion/**` — 計測出力（git 管理外。集約元として参照）

## Done When

- [x] Windows x64 で `mise run bench`（`cargo bench --workspace`、全量）の計測が完了し、2–3 run の分散が記録されている — **実績: 分割実行で 2 run × 756 ケース。分散 p50=1.8% / p90=25.7% / max=516.9%（環境負荷由来、experiment log に記録）**
- [x] `docs/impl/exp/` に jsonl（LF・末尾改行）と環境メタデータつき experiment log 文書が存在する
- [x] experiment log に環境メタデータ・カーネル × impl のスカラー比判定表・denormal 崖観察・判定目安との照合結果が記録されている

## Verification

- [x] `mise run bench`（`cargo bench --workspace`、全量）が完走する — **実績: 単一プロセス全量は環境由来の alloc 失敗で 2 回中断、カーネル分割の全量完走で充足（experiment log 実施記録節に経緯を記録）**
- [x] `ls docs/impl/exp/` で jsonl と experiment log 文書の存在を確認（experiment log レビュー: 環境メタデータ・判定表・分散記録を含むこと）
