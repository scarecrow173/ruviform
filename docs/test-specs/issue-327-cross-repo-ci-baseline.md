---
id: "TSPEC-034RBMKdNX5HX6vExF2WlS"
type: "test-spec"
status: "approved"
title: "Issue #327 CI計測Baseline 検証仕様"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "docs/specs/issue-327-cross-repo-ci-baseline.md"
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
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-327-cross-repo-ci-baseline.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
  verified-by: []
  references:
    - "docs/adr/cross-repo-ci-revision.md"
  defers: []
  deferred-by: []
---

# Issue #327 CI計測Baseline 検証仕様

## Purpose

Issue #327 の AC1〜AC6 が満たされたことを、実測証拠つきで判定する検証仕様。本 issue は計測・文書化タスクであり、「検証」はコードの実行テストではなく (a) 計測スクリプトの再現可能性、(b) Baseline 文書が比較条件・指標分離・必須検証集合を含むこと、(c) fail-closed 原則（NotRun/Unsupported を Pass にしない）が守られることの3点に集約される。後続 Issue がこの仕様を再利用して Before/After の妥当性を監査する。

## Feature

CI Baseline 計測: `gh api` による 3 repo の Run/Job/Step timing 収集、lane 分類、待機/実行分離、critical path・総 Runner 時間・cache Hit/Miss の算出、必須検証集合の再導出と Baseline 記録。

## Rules

- **R1（再現性）**: `node ci/metrics/collect-ci-metrics.mjs` が非ゼロ終了せず、`--out` へ `runs.jsonl`（raw）と `report.md`（派生表）を生成する。report の数値は JSONL から再導出可能。
- **R2（指標分離）**: report が workflow_wall_time / job_wait / job_exec / exec_span / critical_path / total_runner_time を**別欄**で示す。待機時間を実行時間に合算しない。
- **R3（代表 run 記録）**: 3 repo それぞれで代表 run が run id・event・SHA・conclusion・比較条件（toolchain/OS/profile/features/Cargo.lock/cache 状態）付きで記録される。
- **R4（統計の正直さ）**: n≧10 の集合のみ p50/p95 を報告。n<10 は観測値一覧+n+範囲をそのまま報告する。
- **R5（fail-closed）**: 測定不能項目（log 未取得の cache 判定・needs 解決不能の critical path・権限外の contexts）は `unknown`/`NotRun`/`Unsupported` と理由付きで記録し、Pass/ゼロとみなさない。
- **R6（必須検証 Baseline）**: required contexts と workflow job 集合が API/YAML から再導出された形で記録され、Before/After 差分監査に使える。消えた context・not-run 化 lane を差分として検出できる。
- **R7（分類の閉じ性）**: lane 分類に落ちない step/job は `other` として report で名列挙される（分類漏れを沈黙させない）。
- **R8（他 repo 不変）**: audio-lens/synvelis への write API 呼出・本 repo の既存 workflow/検証ファイルの変更がない。

## Examples

- Given: ruviform run 41780346388 の jobs API 値。When: スクリプト実行。Then: `workflow_wall_time_s`=318±(API 秒粒度)、`ui-verify (windows-latest)` の job_exec_s ≈313、12 job の `job_wait_s` が個別に記録される（R2/R3）。
- Given: audio-lens（n=2）。When: report 生成。Then: p50/p95 欄が出ず、2 観測値と n=2 がそのまま記録される（R4）。
- Given: `--logs` なし実行。Then: cache Hit/Miss 欄が `unknown` で、hit と誤記されない（R5）。
- Given: 分類表に無い新規 job `foo-verify`。Then: lane=`other` で report の other 一覧に job 名が出る（R7）。

## Guarantees

- Baseline の各数値は JSONL raw に対応行があり、`gh api` で第三者が再検証できる。
- Before/After テンプレートが design doc に存在し、後続 Issue が同一指標・同一条件欄で記入できる。
- 検証漏れ監査が required contexts・job 集合の差分として機械比較可能。

## Non-goals

- CI workflow 自体の正当性・速度の評価（計測のみ）。
- API が返さない指標（runner 内部 queue 詳細・billing multiplier）の検証。
- 連続監視の正しさ（単発収集のみ）。

## Risk

- これらの保証が外れると、後続最適化が「速いが検証が減った」変更を非回帰と誤認する（Fail-Open）。R5/R6 が最重要。
