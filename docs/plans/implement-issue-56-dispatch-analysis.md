---
id: "PLAN-034JPwgFVQGc33sjgR8htz"
type: "plan"
status: "approved"
title: "Issue #56 dispatch 固定コスト分離の実施計画"
created: "2026-10-21"
test-spec-skip: "計測・分析タスクで外部から観測可能な新規振る舞いを持たない。検証は Acceptance Criteria（定量値の存在・複数 run 分散統計・2 系統証拠の一致）で担保する"
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
  implements:
    - "../specs/dispatched-overhead-analysis.md"
  implemented-by:
    - "../tasks/add-dispatch-probe-bench.md"
    - "../tasks/run-dispatch-remeasure.md"
    - "../tasks/write-dispatch-analysis-report.md"

  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/dispatched-overhead-analysis.md"
    - "../designs/dispatched-overhead-method.md"
  derived-by:
    - "../tasks/add-dispatch-probe-bench.md"
    - "../tasks/run-dispatch-remeasure.md"
    - "../tasks/write-dispatch-analysis-report.md"

  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #56 dispatch 固定コスト分離の実施計画 Implementation Plan

## Goal

`fearless_dispatched` の軽量カーネル劣後を per-call 固定コスト vs ノイズに分離し、#53 の dispatch 層設計へ定量材料を渡す。

## Scope

単一ストリーム: プローブベンチ追加 → 低負荷再計測 → 分析・レポート記録。dispatch 層実装・採否変更は含まない（#53）。

## Upstream Documents

- [../specs/dispatched-overhead-analysis.md](../specs/dispatched-overhead-analysis.md)
- [../adr/dsp-simd-backend-adoption.md](../adr/dsp-simd-backend-adoption.md)（決定 2・検証未完了項目の解消先）

## Design Inputs

- [../designs/dispatched-overhead-method.md](../designs/dispatched-overhead-method.md)（系統 A プローブ + 系統 B N 依存差分の2系統証拠）

## File Map

- `crates/ruviform-math/benches/simd_poc.rs` — `dispatch_probe` グループ追加（`level_new`/`dispatch_noop`/`token_avx2` + 小サイズ `add` 16/32）
- `docs/impl/exp/dispatched-overhead-<date>.md` — 計測レポート（環境メタデータ・分散・判定・#53 示唆）
- `docs/impl/exp/data/dispatched-overhead-<date>-NN.jsonl` — 生データ
- `docs/adr/dsp-simd-backend-adoption.md` — 検証未チェック項目の完了反映

## Implementation Sequence

- [x] 低負荷環境の確認（CPU 負荷・プロセス状況を記録）— 実測 1% 負荷で確認済み
- [x] `dispatch_probe` ベンチ追加（公開 API のみ・unsafe なし）
- [x] elementwise 軽量カーネル（add/scale/clamp_unit/fade）の filtered 再計測 × 複数 run + プローブ測定
- [x] 生データ収集・分散統計・Δ(N) 解析
- [x] レポート記述（分類結論 + #53 示唆）・ADR 検証項目の完了反映

## Dependencies and Risks

- 環境依存: 計測中に他負荷が混入した場合は run を棄却して再計測する（レポートに環境負荷を記録）
- criterion フィルタによる部分実行でも `estimates.json` 産物は全量実行と同一形式
- `dispatch_noop` が最適化で消去された場合は Δ(N) フィットのみで推定（設計書 Error Handling 節）

## Task Breakdown

- T1: プローブベンチ追加（`dispatch_probe` グループ）
- T2: 再計測・生データ収集・分散統計
- T3: 分析・レポート記述・ADR 検証項目反映・文書整合

## Verification

- [x] `cargo bench -p ruviform-math -- dispatch_probe` が実行され値を返す
- [x] elementwise 軽量カーネルが複数 run で再計測され分散統計がある
- [x] 結論分類（固定コスト/ノイズ/混在）が2系統の証拠で支持される
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`bench-smoke` パス
- [x] doc 監査で本チェーン起因 findings 0

## Test Spec Evidence

- 省略: 計測・分析タスクであり挙動変更を持たない。検証性は Acceptance Criteria（定量値の存在・分散統計・2系統証拠の一致）で担保する

## Open Questions / Gaps

- なし
