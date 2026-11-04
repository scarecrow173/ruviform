---
id: "SPEC-nZdlFyflADPeFBthK8sEBB"
type: "spec"
status: "approved"
title: "Issue #333 CI-OPT-06 変更影響ベース検証 DAG・Fail-Closed Required Gate"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/326"
  changes:
    added:
      - type: "directory"
        path: "ci/impact"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/plans/issue-333-impact-based-ci.md"
  depends-on:
    - "docs/discovery/issue-333-impact-classifier-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/adr/cross-repo-ci-revision.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-333-impact-classifier-inventory.md"
  derived-by:
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/plans/issue-333-impact-based-ci.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-333-impact-based-ci.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
metadata:
  spec:
    acceptance-source: "issue-body"
---

# Issue #333 CI-OPT-06 変更影響ベース検証 DAG・Fail-Closed Required Gate

## Problem

`ci.yml` は `pull_request`/`push: main`/`workflow_dispatch` の全 trigger で
6 job・12 required contexts を毎回実行している。変更と無関係な検証の省略で
PR 待ち時間と runner 消費を削減したい。しかし required check 契約の下では、
job 単体を条件付き skip すると GitHub の skipped job が required check として
成功扱いされる穴（required 保護の形骸化）か、逆に required context が欠落して
merge 不能になる。したがって**検証範囲の決定と required gate の安全な移行は
不可分の実装単位**であり、分類器だけ・gate だけを先に導入することはできない。

## Diagnosis（変更前に診断する）

- 12 required contexts は job `name` 単位で ruleset `main-protect`（id
  26529830・enforcement active）に登録されている。GitHub の required check
  評価は check run の conclusion を見るため、job-level `if:` で skip された
  job の check は「成功」として merge を許可する — つまり単純な条件 skip は
  required 契約を壊さずに「検証せずに通る」状態を作る。これは検証の欠落であり
  保護ではない。
- lane 選別に必要な情報は 3 層ある: (a) ファイル→変更クラスの写像、
  (b) 変更 crate/ui パッケージ→依存逆方向伝播、(c) crate→lane の責務写像。
  このうち (b)(c) を静的リストで持つと workspace 進化で陳腐化し、
  「本当は影響があるのに skip する」偽陰性を静かに生む。fail-closed の立場では
  分類規則自体をリポジトリから機械導出する必要がある。
- 導入は段階でしか安全にできない: Shadow Mode（分類のみ出力・Full CI 全実行）
  で Full CI 結果との整合・偽陰性なしを確認してから集約 gate の fail-closed
  検証、Ruleset 移行、選択実行有効化の順。本 PR は Shadow Mode +
  集約 gate の設計/実装/検証までをスコープとし、lane の `if:` 有効化は
  偽陰性なし確認後の後続 PR とする。

## Requirements

1. **Impact Classifier**: 変更ファイル（PR diff）から必要な検証 lane 集合を
   決定する汎用分類器を実装する。crate 依存逆方向伝播（`cargo metadata`
   --no-deps 由来）と ui/* パッケージ依存 DAG（tsconfig・相対 import・
   mise task sources 由来）を機械導出し、過度な専用フレームワークを作らない
   （stdlib-only Node `.mjs`・`ci/metrics` 先例）。
2. **分類対象クラス**: Rust crate/public API、Cargo.lock、toolchain/mise、
   RT、ICC、plugin L1、webview、ui/{core,components,viz,binding,guitest,
   showcase,browser-test}、benchmark、Actions/CI スクリプト、docs。
   曖昧・分類不能・共通依存/設定変更は安全側の **Full CI fallback**。
3. **適用 trigger**: 選択実行は `pull_request` のみ。`push: main` と
   `workflow_dispatch` は常に Full CI（分類結果は証拠として記録してよいが
   lane 期待集合は全 lane）。
4. **監査可能な出力**: 分類結果は期待 lane/OS/feature/対象 revision
   （base/head SHA）/対象外理由を含み、Pass / Fail / NotRun / Unsupported /
   NotApplicable を区別する。`NotApplicable` は影響なしが証明できた場合のみ
   許し、`NotRun`・`Unsupported` を Pass に読み替えない
   （ADR `cross-repo-ci-revision` の fail-closed 原則）。
5. **集約 required gate**: 安定した名前の集約 gate job を設計・実装し、
   期待される lane 結果がすべて成功または根拠付き NotApplicable であることを
   検査する。job skip を合格判定に使わない。実行された lane の失敗は
   分類対象外でも必ず Fail に伝播する（実行結果は常に信用する）。
6. **段階導入**: 初期は Shadow Mode（分類結果のみ出力・Full CI 全実行）。
   Shadow で偽陰性（不要と分類した lane の失敗）が検出された場合、
   その旨を表面化する。選択実行の有効化と Ruleset 移行は本 PR では行わず、
   後続手順としてレビュー可能な形で設計に残す。
7. **移行設計**: 旧 required contexts との互換、PR base 変更時の再検証、
   Cancel/Failure/Skipped/NotRun/Unsupported の扱い、手動 Full CI、
   緊急 Rollback を設計に含める。Ruleset 新旧移行で無保護期間・誤
   デッドロックを作らない手順を定義する。
8. **Consumer 独立性**: AudioLens(#149)/Synvelis(#148) は分類契約と共通部品を
   再利用する。製品固有の影響ルール・SolidJS runtime・製品 Oracle を
   Ruviform に持ち込まない。`consumer-verify.yml`/#294 の両 Consumer 検証は
   本仕組みと独立に維持する。

## Acceptance Criteria

- [ ] AC1: 変更クラス→期待 lane（OS/feature）と依存伝播規則が design 文書としてレビュー可能に文書化されている。
- [ ] AC2: Shadow Mode で classifier が分類結果（期待 lane・理由・revision）を出力し、Full CI が全実行される。classifier が不要と判定した lane の失敗は偽陰性シグナルとして gate 出力に表面化する。
- [ ] AC3: 不明な変更・CI 設定・共有 toolchain・Cargo.lock 変更は Full CI に fallback する（fixture テストで検証）。
- [ ] AC4: 集約 gate が期待集合/実結果/対象外理由を PR の job summary + artifact で監査可能に出力する。
- [ ] AC5: gate が Failure/Cancelled/Skipped(期待 lane の)/分類不能を成功として扱わない（fixture テストで検証）。
- [ ] AC6: main push・workflow_dispatch が Full CI のまま維持され、既存 12 required contexts が本 PR でも変更されない。
- [ ] AC7: Ruleset 新旧移行・選択実行有効化・緊急 Rollback の手順が無保護期間・デッドロックなしで design 文書に定義されている。
- [ ] AC8: `consumer-verify.yml`・`consumer-impact-check.yml` が無変更で、両 Consumer の候補 SHA 検証・独立 CI が欠落しない。
- [ ] AC9: Before/After 記録方針（critical path・総 runner 時間・失敗検出能力・Rollback）が baseline(#327) の計測契約に沿って design/impl 記録に残る。Shadow 期は After=同一（削減なし）が正しい値。

## Non-goals

- lane job の `if:` 条件付き skip の有効化（Shadow 検証完了後の後続 PR）。
- Ruleset `main-protect` の required contexts 変更（#326 で追跡される別途の
  運用作業。本 PR は手順を設計に残すのみ）。
- matrix leg（OS）単位の選択実行 — 選択単位は job 粒度とする。
- AudioLens/Synvelis 側の製品固有分類ルール実装（各 Consumer 側 Issue の責務）。
- `consumer-verify.yml` の trigger 変更・required 化。
- CI 所要時間自体の削減効果実測（Shadow 期は削減なしが正しい状態）。

## Consumer 影響分類の前提

本変更は `.github/workflows/ci.yml` に job を 2 つ追加し mise.toml に task を
追加するが、crate 公開 API・feature・dependency には一切触れない。
Consumer への影響は PR 本文の定型分類表で `no impact`（CI 基盤内部変更・
Consumer 検証 workflow 無変更）を根拠付きで記録する。

## 関連文書

- 前提調査: [docs/discovery/issue-333-impact-classifier-inventory.md](../discovery/issue-333-impact-classifier-inventory.md)
