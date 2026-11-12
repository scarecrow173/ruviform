---
id: "IMPL-ngz9RSZGnlsfJnmktd0EGE"
type: "impl"
status: "completed"
title: "Issue #333 CI-OPT-06 変更影響ベース検証 DAG・fail-closed 集約 gate（Shadow Mode）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/333"
  changes:
    added:
      - type: "file"
        path: "ci/impact/map.mjs"
      - type: "file"
        path: "ci/impact/lib.mjs"
      - type: "file"
        path: "ci/impact/classify.mjs"
      - type: "file"
        path: "ci/impact/gate.mjs"
      - type: "file"
        path: "ci/impact/tests/classify.test.mjs"
      - type: "file"
        path: "ci/impact/tests/gate.test.mjs"
      - type: "file"
        path: "docs/discovery/issue-333-impact-classifier-inventory.md"
      - type: "file"
        path: "docs/specs/issue-333-impact-based-ci.md"
      - type: "file"
        path: "docs/designs/issue-333-impact-based-ci.md"
      - type: "file"
        path: "docs/plans/issue-333-impact-based-ci.md"
      - type: "file"
        path: "docs/tasks/issue-333-t1-impact-classifier.md"
      - type: "file"
        path: "docs/tasks/issue-333-t2-ci-gate-shadow-wiring.md"
      - type: "file"
        path: "docs/test-specs/issue-333-impact-based-ci.md"
      - type: "file"
        path: "docs/impl/ir/issue-333-ci-opt-06-impact-gate.md"
      - type: "file"
        path: "docs/impl/exp/issue-333-impact-based-ci.md"
      - type: "file"
        path: "docs/impl/exp/data/issue-333-impact-decision-run-41835836190.json"
      - type: "file"
        path: "docs/impl/exp/data/issue-333-gate-report-run-41835836190.json"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements:
    - "docs/tasks/issue-333-t1-impact-classifier.md"
    - "docs/tasks/issue-333-t2-ci-gate-shadow-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-333-impact-based-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/test-specs/issue-333-impact-based-ci.md"
    - "docs/discovery/issue-333-impact-classifier-inventory.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #333 CI-OPT-06 変更影響ベース検証 DAG・fail-closed 集約 gate（Shadow Mode）

## Summary

変更ファイルから必要検証 lane 集合を決める汎用 Impact Classifier（`ci/impact/`、stdlib-only Node `.mjs`）と、全 lane の `needs` 結果を期待集合と照合する fail-closed 集約 `ci-gate` を `.github/workflows/ci.yml` へ **Shadow Mode** で追加した。分類は `cargo metadata` の workspace 逆依存伝播・`rt_` 保持 crate scan・mise task `-p` 引数・`ui/*` DAG・full-fallback 規則で機械導出し、曖昧/分類不能/共通設定変更は Full CI。lane job の `if:` は一切変更せず既存 12 required contexts を維持したまま、`impact`（分類+decision artifact）と `ci-gate`（`if: always()` で needs 全結果を判定・gate-report artifact）が実運転で観測される。PR #345 run 41835836190 で両 job 緑・12 contexts 全 pass・decision `full` × gate `pass` を確認。

## Context

task `TASK-69XeSXbqn9bazeVEgD0LzH` / `TASK-auXqAVyXzCi31qldUWzY6t`（plan `PLAN-zwAUuudwYKnZuLsOviWhpU`、spec `SPEC-nZdlFyflADPeFBthK8sEBB`、design `DESIGN-oq5cpmBZrL3fUcNnuUzPxg`、test-spec `TSPEC-zbT71fmv3NCZBzB3xof25n`、discovery `DISC-3qnA5E8SxxIxxEfA9LSI1k`）として実施。Issue #333 CI-OPT-06、#326 で active 化済みの `main-protect` ruleset（12 required contexts）を壊さない Shadow 導入。分類契約は Consumer（AudioLens #149・Synvelis #148）再利用前提の reviewable 文書として design が正本。

## Implementation

### Changes

- `ci/impact/map.mjs`: 分類規則データ。file class glob（crate ソース/`Cargo.lock`/toolchain/mise/RT/ICC/plugin L1/webview/`ui/*` 7 種/benchmark/Actions・CI スクリプト/docs）・lane 定義（OS・feature・required context 名）・full-trigger 規則（ci-config・shared-config・lockfile・unclassified 混在）を保持する reviewable data module。
- `ci/impact/lib.mjs`: `cargo metadata --no-deps` から workspace member/dep graph を組立て、変更 crate → 逆依存伝播 → lane-owner 解決（rt_ 保持判定・`bench-smoke` の `-p` 引数 parse・ICC/webview/L1 feature owner・ui/* DAG）を行う共有ロジック。
- `ci/impact/classify.mjs`: CLI。`--event/--base/--head/--files/--all/--out` を受け、git diff（two-dot tree diff、merge-base 不要）または明示 file list から decision JSON（schemaVersion・event・base/head revision・lane 別 expected/OS/feature/contexts/reasons・file 別 class）を出力。`CI_IMPACT_DISABLE=1` で expected=ALL に退化する kill switch。GITHUB_OUTPUT へ decision/expected/lane_* を emit。
- `ci/impact/gate.mjs`: CLI。`NEEDS_JSON`（`toJson(needs)`）× decision JSON を照合し gate-report.json + exit code を返す fail-closed 判定。expected lane の missing/failure/cancelled/skipped・分類失敗・期待集合出力欠落は全て Fail。not-expected lane の実行 failure は偽陰性シグナルとして Fail（shadow 検出器）。not-expected 未実行のみ NotApplicable。skipped を Pass にしない。
- `ci/impact/tests/`: `node --test` contract test 30 件（分類規則 fixture・full fallback・gate 判定表の expected×actual 全組合せ）。
- `mise.toml`: `[tasks.test-ci-impact]` 新設（`node --test ci/impact/`）。impact job が decision 出力前の自己検証として呼ぶ。
- `.github/workflows/ci.yml`:
  - `pull_request` types に `ready_for_review`・`edited` を追加（base retarget で再分類）。
  - `impact` job 追加: checkout + base SHA fetch + mise（rust・node）→ `mise run test-ci-impact` → classify（`workflow_dispatch` は `--all`）→ decision を job outputs + `impact-decision` artifact へ出力。
  - `ci-gate` job 追加: `needs: [impact, verify, rt-verify, icc-verify, ui-verify, bench, l1-real-verify]` + `if: always()`、mise（node）→ `gate.mjs` → `ci-gate-report` artifact（`if: always()` upload）。
  - 既存 6 lane job は無変更（Shadow Mode）。
- `AGENTS.md`: `test-ci-impact` 行追加、ci.yml 説明文に `impact`/`ci-gate`・`edited` type・`CI_IMPACT_DISABLE`・Shadow 段階性を追記、`bench-smoke` 行の stale 記述（viz 抜け）を実態に同期。
- `docs/`: discovery/spec/design/plan/test-spec/tasks/ir/exp チェーン + 生成 index 再構築。

### Decisions During Implementation

- diff は `git fetch --depth=1 origin $BASE` 後の two-dot tree diff — merge-base 不要で shallow clone と両立。
- `impact` job は `MISE_ENABLE_TOOLS: rust,node` で pin toolchain のみ install — cargo metadata と node の両方が必要で、他 lane の rust 群と pin を共有。
- `ci-gate` は matrix leg でなく job 粒度の `needs` result を判定 — per-OS 内訳は各 leg 自身の check run が持ち、権限追加（checks API）を避ける。
- 分類不能・mixed-impact は lane 推論を試みず Full に倒す — 部分 lane 集合の誤判定は偽陰性源。
- `edited` type の採用 — title 編集でも再実行されるが concurrency group が前 run を cancel するため実害なし、base retarget の stale 期待集合を防ぐ方が重要。
- gate の artifact upload は `if: always()` + `if-no-files-found: warn` — gate 自身が fail しても report が残る経路を確保（report 未生成なら warn のみで二重 fail 化しない）。

## Related Experiments

### Adopted

- 本 PR（#345・run 41835836190）での Shadow 観測: `impact-classify` 18s pass・decision=`full`（ci-config+shared-config が fallback 規則どおり発火）・`ci-gate` 10s pass・6 lane 全 Pass・既存 12 contexts 全 pass。raw: `docs/impl/exp/data/issue-333-*-run-41835836190.json`。
- `node --test` contract test で gate 判定表（expected×actual 全組合せ・skipped≠Pass・NotApplicable の証明条件）を fixture 検証 — design 判定表と実装の一致を機械確認。

### Rejected

- `impact` の lane outputs を lane job `if:` へ今回接続すること: 偽陰性なし確認前の選択実行は issue の段階設計違反（required check 形骸化リスク）。Shadow 維持を採用。
- `actions/cache`・`paths-filter` 等の既成 Action による差分収集: 分類規則（逆依存伝播・feature 判定）が本 repo 固有で、外部 Action に渡すと contract が見えなくなる。stdlib-only 自前実装を採用。
- `docs-only` を「全 lane skip」にすること: docs 変更でも prek の言語非依存 hook が効く `verify` は必要（test-spec Rule 化済み）。

## Validation

実測環境（ローカル）: Windows 11 x64 / Git Bash、mise 管理 toolchain（node 26.12.1）。

| 項目 | 実行 | 結果 |
| --- | --- | --- |
| `mise run test-ci-impact` | `node --test ci/impact/` | Pass — 30/30（classify 19 + gate 11） |
| `mise exec -- actionlint .github/workflows/ci.yml` | — | Pass（exit 0） |
| `prek run`（変更ファイル全 hook） | commit 時フック | Pass |
| classifier E2E（本 PR 差分） | `decision=full`・6 lane expected | Pass — ci-config/shared-config fallback 発火 |
| doc_status lint（issue-333 docs） | lint --json | Pass — 新規 finding 0（index・reciprocal・navigation 解消済み） |

CI（PR #345、2026-11-05）:

- run 41835836190（pull_request・初回）: 15 check 全 pass — 12 required contexts + `impact-classify`（18s）+ `ci-gate`（10s）+ consumer-impact `classification`。
- impact-decision artifact: `decision=full`・6 lane expected・base `f880c79` / head `8306080`。
- gate-report artifact: `verdict=pass`・`classificationOk=true`・6 lane 全 `expected=true/result=success/verdict=Pass`。
- Shadow 比較: expected 集合 == 実行集合（full のため偽陰性検出器は本 run では未発動 — 設計どおり）。

## Risks

- 分類器の誤判定(expected が本当に必要な lane を欠く)は偽陰性源。分類不能・共通設定変更・lockfile は Full CI へ倒す fallback 規則で緩和しているが、分類規則の盲点は残りうる — shadow 期間の継続観測(Follow-ups)が前提。
- `ci-gate` は現状 required context ではなく観測用(Shadow Mode)。lane job の `if:` は未接続のため、gate が Fail しても merge 判定は従来の 12 required contexts が担う — 誤検出は CI を止めないが、逆に gate の検出も merge を防がない。
- fail-closed 設計上、分類失敗・期待集合出力欠落は gate Fail となる。gate を required context へ昇格する Stage C/D 以降は、classifier/toolchain の障害が全 merge を止めうる — 昇格前に kill switch(`CI_IMPACT_DISABLE=1`)と復旧手順の確認が必要。
- `edited` trigger の採用により title 編集でも再実行される。concurrency group が前 run を cancel するため実害なしと判断したが、連続編集で run が churn しうる。

## Follow-ups

- 偽陰性なし確認の継続観測（docs-only/単一 crate 変更 PR で expected 部分集合の実測蓄積）。
- ruleset `main-protect` の required context 移行（12 → `ci-gate` 単一。design runbook の Stage C/D、無保護期間なし手順）。
- 選択実行の有効化（lane `if:` への `impact` outputs 接続。上記完了後のみ）。
