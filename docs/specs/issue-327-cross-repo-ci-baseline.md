---
id: "SPEC-034RBJKNinKQbKocpnqqM9"
type: "spec"
status: "approved"
title: "Issue #327 3リポジトリ横断CI計測Baseline・非回帰基準 仕様"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "docs/discovery/issue-327-cross-repo-ci-baseline.md"
  changes:
    added:
      - type: "directory"
        path: "ci/metrics"
      - type: "file"
        path: "docs/discovery/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/specs/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/designs/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/plans/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/test-specs/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/tasks/issue-327-t1-baseline-measurement.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/cross-repo-ci-revision.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-327-cross-repo-ci-baseline.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-327-cross-repo-ci-baseline.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/ruviform/issues/328"
    - "https://github.com/scarecrow173/ruviform/issues/329"
    - "https://github.com/scarecrow173/ruviform/issues/330"
    - "https://github.com/scarecrow173/ruviform/issues/331"
    - "https://github.com/scarecrow173/ruviform/issues/332"
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "https://github.com/scarecrow173/audio-lens/issues/148"
    - "https://github.com/scarecrow173/audio-lens/issues/149"
    - "https://github.com/scarecrow173/synvelis/issues/147"
    - "https://github.com/scarecrow173/synvelis/issues/148"
  defers: []
  deferred-by: []
---

# Issue #327 3リポジトリ横断CI計測Baseline・非回帰基準 仕様

## What Should Be Built

Ruviform / AudioLens / Synvelis の現行 CI の所要時間と検証内容を共通指標で測定・記録する **CI 計測 Baseline**。後続の CI 最適化 Issue（#328 Concurrency・#329 sccache・#330 npm 集約・#331 Cargo 重複・#332 Playwright・#333 変更影響 CI・AudioLens #148/#149・Synvelis #147/#148）が、変更前後の効果と検証品質の非回帰を同一手順で評価できるようにする。

1. **計測スクリプト** `ci/metrics/collect-ci-metrics.mjs`: `gh api` で対象 repo の Workflow Run・Job・Step の timing を取得し、lane 分類・待機時間・実行時間・総 Runner 時間・critical path・(log 取得時) cache Hit/Miss を算出して JSONL raw + Markdown 表を出力する。
2. **計測契約**（design doc）: 指標定義・lane 分類・Cold/Warm 判定・比較条件・Before/After テンプレートの正本。
3. **Baseline 記録**（experiment log + raw JSONL）: 3 repo の直近代表 Run の実測値・計測条件・必須検証集合の棚卸し。

新たな監視サービス・証拠 DB は作らない。GitHub Actions API・Job Summary・既存 Evidence のみを使う。

## Why Now

CI 最適化 DAG の根幹 Issue（P0・依存なし）であり、後続 9 Issue 全てが本 Baseline の指標と Before/After テンプレートで効果を報告する前提になっている。Baseline がないと「速くなったが検証が減った」型の退化（Fail-Open）を検出できない。

## Users and Value

- 後続 CI 最適化 Issue の担当者: 同一スクリプト・同一テンプレートで Before/After を測り、効果（wall time・critical path・総 Runner 時間）と検証漏れ（必須検証集合の差分）を報告できる。
- Ruviform/Consumer のレビュア: 必須検証一覧の Baseline に対する差分で、カバレッジの暗黙削減を監査できる。
- Issue #327 自体: 「計測の正本」として 3 repo の現状観測を確定値に更新する。

## Scope

### In Scope

- 3 repo（`scarecrow173/ruviform`・`scarecrow173/audio-lens`・`scarecrow173/synvelis`）の PR・main push・workflow_dispatch Run を `gh api` で計測する手順と、その実施（直近の代表 Run）。
- Run/Job/Step の開始・終了・待機時間の分類取得（success・failure・cancelled・再試行 `run_attempt` を区別）。
- lane 分類（Rust compile/check/clippy/test、mise、npm、Playwright、依存取得、GUI・RT・MSRV・Cross-Consumer 等）・critical path・総 Runner 時間（job 実行時間合計）の算出。
- Cargo cache・mise cache・（存在すれば）sccache・Playwright cache の Hit/Miss と転送コストを **job log 解析で測定可能な範囲** で記録。Cold/Warm の判定手順を定義。
- 比較条件（repo/SHA/toolchain/OS/profile/features/Cargo.lock/GUI fixture）と、job 待機時間と実行時間の分離ルール。
- 必須検証一覧（OS/feature/Golden/RT/GUI/MSRV・required checks）の Baseline 化と、変更前後の検証漏れ・Fail-Open 監査手順。
- 測定手順・結果保存先の docs 記録と、既存 Issue から参照可能な Before/After テンプレート。
- 本 issue の doc chain（discovery → spec → design → plan → test-spec → task → impl record / experiment log）。

### Out of Scope

- CI の変更・最適化そのもの（workflow・mise・依存の変更は後続 Issue の範囲。本 issue は計測と文書化のみ）。
- AudioLens・Synvelis リポジトリ内のファイル変更（read-only 観測のみ）。
- 継続的な監視基盤・証拠 DB・ダッシュボードの構築。
- required checks / Ruleset の変更（#326/#333 の管轄）。
- p50 20% 短縮目標の達成（目標値の定義は既存 Issue にあり、本 issue は基準値の確定のみ）。

## Requirements

- **証拠分離**: Pass/Fail/NotRun/Unsupported/NotApplicable を区別し、NotRun/Unsupported を Pass に読み替えない（ADR `cross-repo-ci-revision` の fail-closed 原則に従う）。
- **待機時間の分離**: job 待機時間（queue）と job 実行時間を別指標として報告し、混ぜない。
- **再現性**: 計測はスクリプトまたは文書化手順として再実行可能とし、raw データ（JSONL）を保存して集計を監査可能にする。
- **二重管理の回避**: 必須検証一覧は required checks と workflow job 集合から機械的に再導出できる形で記録し、手書きコピーのみの正本を作らない。
- **測定の正直さ**: 観測値と統計値を区別する。n が十分なら p50/p95、少ない場合は観測値と n・ばらつきをそのまま報告する（n=1/2 で統計値を装わない）。
- **他 repo 不変**: AudioLens/Synvelis は read-only 観測。本 repo 内での変更も計測用の新規ファイルと docs に限定し、既存 workflow・検証を変更しない。

## Acceptance Criteria

- [x] AC1: 3 repo それぞれの代表 Run（run id・event・SHA・conclusion）・計測条件（toolchain/OS/profile/features/Cargo.lock）・現行チェック集合（job 一覧と required checks）が docs に記録されている。
- [x] AC2: Workflow wall time / job 実行時間 / job 待機時間 / step 時間 / critical path / 総 Runner 時間が分離され、同一テンプレートで 3 repo 比較可能である。
- [x] AC3: Cold/Warm の区別方法と cache Hit/Miss・転送コストの計測手順が確立され、測定可能な範囲で実測値が記録されている。測定不能な項目は NotRun/Unsupported と理由付きで明記されている。
- [x] AC4: 後続 Issue がそのまま使える Before/After 測定テンプレート（指標・条件記入欄・判定欄）が docs に存在する。
- [x] AC5: 必須検証一覧 Baseline が `gh api`（required checks）と workflow job 集合から再導出可能な形で記録され、検証漏れ・Fail-Open を差分検出できる。
- [x] AC6: 計測手順と結果の保存先が docs に記載され、スクリプト実行により raw JSONL が再生成可能である。

## Deferred Design Concerns

- CI 結果の継続収集・傾向監視 — Issue は新規監視基盤を禁止。必要になれば別 Issue で再検討。
- self-hosted runner や請求時間（billing）ベースの計測 — 現行は GitHub-hosted のみ。分母が変わる変更が入った場合に再定義。
- Playwright cache 等、現行 workflow に存在しない cache 種別の分類 — 該当 lane が追加された時点で lane 分類表を拡張する。

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/327 （CI-OPT-00・P0・依存なし。Acceptance が正本）
- Discovery: `docs/discovery/issue-327-cross-repo-ci-baseline.md` — 3 repo の workflow/job/required checks/Run 蓄積の実測棚卸し。
- 上位原則: `docs/adr/cross-repo-ci-revision.md`（not-run を pass としない fail-closed・証拠ゼロを緑にしない）。
- 先行様式: `docs/discovery/issue-194-m0-baseline-audit.md`・`docs/discovery/issue-219-consumer-cross-repo-ci.md`。
