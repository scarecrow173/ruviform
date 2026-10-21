---
id: "SPEC-034JPubrypO0IylcWkuqlz"
type: "spec"
status: "approved"
title: "fearless_dispatched 軽量カーネル劣後の原因分離（低負荷再計測）"
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
    - "../adr/dsp-simd-backend-adoption.md"
  derived-by:
    - "../designs/dispatched-overhead-method.md"
    - "../plans/implement-issue-56-dispatch-analysis.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# fearless_dispatched 軽量カーネル劣後の原因分離（低負荷再計測）

## What Should Be Built

`fearless_dispatched`（per-call `dispatch!(Level::new(), …)`）が軽量 elementwise カーネルで強制 avx2 列に劣後した原因を、「per-call dispatch 固定コスト」と「計測ノイズ」に分離し、定量的な結論を実験ログとして残す計測・分析タスク。成果物は `docs/impl/exp/` 配下の計測レポート（+ 生データ jsonl）であり、dispatch 層設計 Issue（#53: prepare 固定・トークン繰り上げ）の判断材料とする。

原因候補の切り分け方法:

- **per-call 固定コスト仮説**: `Level::new()`（`LazyLock` キャッシュ済み検出の atomic 読出し）+ `dispatch!` の `__dispatch_target` 正規化 + arm 選択呼出しは呼出しあたり定数コスト。小サイズ帯ではカーネル本体時間が小さいため固定コストが支配し、比率が最も悪化するはず → N に対する差分フィットと専用プローブ計測で直接定量する
- **ノイズ仮説**: 前回計測は並行エージェント多数稼働下（~350 プロセス・コミット逼迫で分割実行）で run 間分散 p50 1.8% / p90 25.7% / p99 237.7%。低負荷環境での再計測で劣後が消える・不安定になるならノイズ起因

## Why Now

採否 ADR（`docs/adr/dsp-simd-backend-adoption.md`）決定 2 の条件 (b) は「dispatch は per-call ではなく prepare/ブロック境界でトークンを保持・渡す」ことを `fearless_dispatched` の劣後を根拠に要求しているが、その劣後が dispatch コストかノイズかは未解決（同 ADR 検証表の未チェック項目・Issue #56 として起票済み）。#53 の dispatch 層設計が「prepare 固定が必須か」「トークン繰り上げで十分か」の判断を要するため、先に原因を定量する。

## Users and Value

- #53 の設計者: dispatch 層を prepare 固定まで持ち上げる根拠（または不要性）を実測で得る
- 採否 ADR の検証完了: 決定 2 の未チェック項目を閉じる

## Scope

### In Scope

- 低負荷環境（他エージェント・重いプロセス不在）での再計測
- `fearless_dispatched` vs `fearless_avx2`（+ 比較基準として `scalar`）の elementwise 軽量カーネル（`add`・`scale`・`clamp_unit`・`fade`）を含む小サイズ帯の計測
- per-call `dispatch!` 素コストの直接計測（`Level::new()` 単独・`dispatch!` no-op・強制トークン生成の専用プローブベンチ）
- N に対する差分の解析（固定コスト支配なら N↑ で比率回復・定数差分として残る）
- run 間分散の再評価（複数 run・中央値採用）
- 実験ログ（`docs/impl/exp/`）へのレポート + 生データ記録

### Out of Scope

- dispatch 層の本番実装（#53 スコープ）
- `fearless_simd` 本体の修正・フォーク
- AVX-512/NEON 実機計測（#55・環境待ち）
- 本番 `[dependencies]` への昇格（#52/#53 スコープ）
- ADR の再採否判断自体（本 Issue は判断材料の提供まで）

## Requirements

- 再計測は低負荷環境で実施し、計測時の環境負荷（プロセス数・CPU 負荷）をレポートに記録する
- per-call 固定コストとノイズの分離には少なくとも2系統の独立した証拠を使う: (a) 専用プローブによる素コスト直接計測、(b) N 依存の差分解析（小サイズ帯含む）
- 複数 run を実施し、run 間分散を前回値と比較する
- 結論は「per-call コスト支配」「ノイズ支配」「混在」のいずれかに分類し、#53 への設計示唆を明記する

## Acceptance Criteria

- [ ] 低負荷環境での再計測データ（生データ jsonl + 環境メタデータ）が `docs/impl/exp/` に記録されている
- [ ] `dispatch!` per-call 素コストがプローブ計測で定量されている
- [ ] 軽量カーネルの劣後比率が N 依存モデルまたはラン間比較で「固定コスト vs ノイズ」に分類されている
- [ ] 採否 ADR の検証項目「`fearless_dispatched` の軽量カーネル劣後が per-call dispatch コストかノイズかの分離」に回答する記述がある
- [ ] #53 への設計示唆（prepare 固定の要否・トークン繰り上げ方針）が記録されている

## Open Questions

- なし（計測で解決する）
