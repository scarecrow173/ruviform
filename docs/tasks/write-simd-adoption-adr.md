---
id: "TASK-034Jf4g6600XCHwXaEZG26"
type: "task"
status: "todo"
title: "採否 ADR"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "docs/adr/dsp-simd-backend-adoption.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/adr/README.md"
  implements:
    - "../plans/dsp-simd-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/dsp-simd-poc.md"
    - "add-simd-equivalence-matrix-tests.md"
    - "run-simd-bench-and-collect-results.md"
  blocks:
    - "record-impl-and-sync-docs.md"
  supersedes: []
  superseded-by: []
  related:
    - "../specs/dsp-simd-poc-adr.md"
    - "../designs/dsp-simd-poc.md"
    - "../adr/dsp-simd-poc.md"
    - "../discovery/dsp-simd-poc-issue-19.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 採否 ADR

## Work

plan `PLAN-034JdOWI3vblJW11kIueY0`（[../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md)）の Phase G / Task T7。ベンチ・等価性・補助データの証拠から採否 ADR を作成し、SIMD 二層構造採否・バックエンド選定・MSRV・等価基準・denormal / AVX-512 を結論する。

- [ ] G-1. 採否基準の補助データを収集: コンパイルコスト（T1 A-1 の baseline に対する `mise run check` 所要時間の dev-dep 前後比較 + `mise exec -- cargo check --timings --workspace --all-targets` の候補クレート分抽出）、unsafe 範囲（`mise run lint` の `unsafe_code` lint 通過を正の検証とし、`assume_supported` / `new_unchecked` / `std::arch` を使わずに済んだことを確認。`grep -rn "unsafe"` はコメント・doc 言及を拾う偽陽性があるため採用しない）、MSRV 適合（候補各クレートの `rust-version` 列挙 + pulp `x86-v4` の推移的下限 ≈1.89 の確認 + `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` 実績）、保守状況（discovery 記録値の更新確認）
- [ ] G-2. `docs/adr/` に採否 ADR を作成（adr-doc skill で scaffold。ファイル名既定: `dsp-simd-backend-adoption.md`）。設計ベースラインの採否基準に沿い、SIMD 二層構造の採否・バックエンド選定・等価基準確定値（実測 max ulp に余裕を持たせた値）・AVX-512 の扱い・denormal 方針・MSRV 判断を記録（不採用・条件付き採用も正当な結論。先に決めた結論へ数値を寄せない）
- [ ] G-3. ADR の front matter relations を設定（`source` = Issue #19、`derives-from` = 本 plan / design / spec / approach ADR）し、`node .agents/skills/doc-status/scripts/audit_docs.js --type adr --json` がクリーンであることを確認

コミット粒度は plan 規約どおり 1 タスク 1 コミット。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md) — `PLAN-034JdOWI3vblJW11kIueY0`（approved）。Phase G の正本
- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`。採否 ADR 存在の受け入れ基準
- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（approved）。採否基準・ADR への入力形式の正本
- [../adr/dsp-simd-poc.md](../adr/dsp-simd-poc.md) — `ADR-034JcihlNe8WCZVpp9RBCq`（proposed）。評価アプローチの決定（本 ADR の `derives-from` 対象）
- [../discovery/dsp-simd-poc-issue-19.md](../discovery/dsp-simd-poc-issue-19.md) — 候補比較・MSRV 調査値の canonical 記録（保守状況確認の基準値）

## Dependencies

- 前提タスク: [add-simd-equivalence-matrix-tests.md](add-simd-equivalence-matrix-tests.md)（T4 — 等価性結果・観測 max ulp）、[run-simd-bench-and-collect-results.md](run-simd-bench-and-collect-results.md)（T6 — 計測結果・experiment log）。補助データは T1 の baseline 計測値も参照
- 後続をブロック: [record-impl-and-sync-docs.md](record-impl-and-sync-docs.md)（T8 — impl record・文書整合の前提）
- 構造的制約の記録対象: `multiversion` の個別 clone 強制不可（plan Risk 9）、AVX-512 実測環境不在の場合は「実測不在」判断材料を明示（Risk 10）

## Affected Files

- `docs/adr/dsp-simd-backend-adoption.md` — 新規（採否 ADR。ファイル名は adr-doc skill での生成時に確定可、計画名を既定とする）
- `docs/adr/README.md` — 生成索引の再生成（ADR 追加に伴う索引更新）

## Done When

- [ ] 採否 ADR が `docs/adr/` に存在し、二層構造採否・バックエンド選定・MSRV 判断・等価基準確定値・denormal / AVX-512 方針の 6 項目を記録している
- [ ] ADR の front matter relations が設定されている（`source` = Issue #19、`derives-from` = plan / design / spec / approach ADR）
- [ ] 証拠（ベンチ結果・等価性結果・補助データ）への参照が ADR 本文に含まれている

## Verification

- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type adr --json` がクリーン
- [ ] ADR 内容レビュー: 6 項目の結論と証拠参照が揃っていること
