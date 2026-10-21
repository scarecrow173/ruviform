---
id: "TASK-034JPwZ8htMzYNPNgOceEI"
type: "task"
status: "done"
title: "分析・レポート記述・文書整合"
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
  implements:
    - "../plans/implement-issue-56-dispatch-analysis.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-56-dispatch-analysis.md"
    - "run-dispatch-remeasure.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-56-dispatch-analysis.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 分析・レポート記述・文書整合

## Work

1. Δ(N) = dispatched − avx2 の解析（定数性・系統 A プローブ値との一致確認）
2. run 間分散を前回値（p50 1.8% / p90 25.7%）と比較
3. 結論を「per-call 固定コスト支配 / ノイズ支配 / 混在」に分類
4. #53 への設計示唆（prepare 固定の要否・トークン繰り上げ）を記述
5. `docs/impl/exp/dispatched-overhead-<date>.md` レポート作成
6. `docs/adr/dsp-simd-backend-adoption.md` の検証未チェック項目を実測結果で完了に更新
7. 全量検証 + doc 監査

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- 先行タスク: [run-dispatch-remeasure.md](run-dispatch-remeasure.md)

## Affected Files

- `docs/impl/exp/dispatched-overhead-<date>.md`（新規）
- `docs/adr/dsp-simd-backend-adoption.md`（検証項目）
- `docs/tasks/README.md`（ステータス同期）

## Done When

- [x] 分類結論が2系統証拠で支持されている
- [x] #53 示唆が記録されている
- [x] ADR 検証項目が更新されている
- [x] レポートが `docs/impl/exp/` に存在する

## Verification

- [x] `mise run fmt-check`/`check`/`lint`/`test`/`bench-smoke` パス
- [x] doc 監査で本チェーン起因 findings 0

## Completion Notes

- 分類結論: **混在（per-call 固定コスト実在 + 大サイズ帯はノイズ）**。2 系統証拠の一致: (a) `dispatch_noop` 1.29ns 安定・(b) 小 N で Δ=+6-7ns（add、run 間 ±0.1-1.5ns）安定・大 N で符号反転する ±20-70ns 散乱。
- 前回の @1024 劣後（add 0.62x・scale 0.66x）は低負荷で 0.91-0.97x に回復 — ノイズ確定。小サイズ回帰（0.48-0.90x）は実在。
- レポート: `docs/impl/exp/dispatched-overhead-20261021.md`。#53 示唆（prepare 固定の正当性・小 N 軽量カーネルのスカラールーティング・dispatched 大サイズ劣化なし）を記録。
- `docs/adr/dsp-simd-backend-adoption.md` 検証項目を実測回答で完了化 + MSRV 項目を #52 実績で更新。
