---
id: "TASK-aX702E4kUPjRWUHnHKK82B"
type: "task"
status: "in-progress"
title: "Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-314-golden-evidence.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/normalize.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/report.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-314-golden-evidence.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-314-golden-evidence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-314-golden-evidence.md"
  verified-by:
    - "docs/test-specs/issue-314-golden-evidence.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane

## Work

Golden Utility の環境正規化・fixture registry 結合・Scenario
runner 統合・証拠 lane を `ruviform-ui-test` に additive に実装
する。

- `golden/normalize.rs` 新設: `GoldenKeyEnv`、`key_env`
  （`EnvironmentMeta` → env class 正規化）、`NormalizeError`、
  `key_for_fixture`（`StoryFixture` 結合の `GoldenKey` 構築）、
  `registry_mismatches`/`RegistryMismatch`（`StoryManifest`
  整合検査）。
- `golden.rs`: `pub mod normalize;` + re-export。
- `golden/report.rs`: `GoldenEvidence.lane`（`"pixel-golden"`）
  + `EVIDENCE_LANE` 定数。
- `scenario/visual.rs`: `record_golden_outcome`
  （`ScenarioOutcome` 版の golden fold）+ 観測 JSON/merge の
  共有 helper 抽出 + module doc 更新。
- `tests/golden.rs`/`tests/scenario_contract.rs`: 正規化・
  registry・`run_scenario` 経由 golden step・evidence lane の
  test 追加。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-314-golden-evidence.md`（実装計画）
- `docs/designs/issue-314-golden-evidence.md`（設計 — 正規化規則、
  registry 2 段結合、lane 語彙、#262/#313 リンク構造）
- `docs/specs/issue-314-golden-evidence.md`（spec — R1〜R8/AC-1〜8）
- `docs/test-specs/issue-314-golden-evidence.md`（test 仕様）

## Dependencies

- slice 1（`docs/specs/issue-314-golden-core.md`、PR #481）—
  `BaselineStore`/`RgbaImage`/`GoldenKey`/`EnvironmentMeta`/
  `GoldenEvidence`/`append_evidence`。
- slice 2（`docs/specs/issue-314-golden-scenario.md`、PR #483）—
  `VisualGuiTest`/`golden_check`/`record_golden_step`。
- slice 3（`docs/specs/issue-314-golden-capture.md`、PR #487）—
  GPUI 実窓 capture backend（consumer 側、参照のみ）。
- #464 runner slice（`docs/specs/issue-464-scenario-runner.md`、
  PR #511）— `run_scenario`/`ScenarioSpec`/`ScenarioOutcome`/
  `settle_until`。
- #390（`docs/specs/issue-390-driver-contract-lab.md`）—
  `StoryManifest`/`stories.json` registry。
- #426（`docs/specs/issue-426-gpui-lab-binding.md`）—
  `lab_story::StoryFixture` code registry。

## Affected Files

- `crates/ruviform-ui-test/src/golden/normalize.rs`（新規）
- `crates/ruviform-ui-test/src/golden.rs`（mod 宣言 + re-export）
- `crates/ruviform-ui-test/src/golden/report.rs`（`lane` field）
- `crates/ruviform-ui-test/src/scenario/visual.rs`
  （`record_golden_outcome` + 共有 helper + doc）
- `crates/ruviform-ui-test/tests/golden.rs`（test 追加）
- `crates/ruviform-ui-test/tests/scenario_contract.rs`（test 追加）
- `docs/{specs,designs,plans,tasks,test-specs}/issue-314-golden-evidence.md`
  （新規）+ `docs/*/README.md`（index 自動生成）

## Done When

- [x] `key_env` が `EnvironmentMeta` から env class を正規化し、
      不正値を `NormalizeError` で拒否する。
- [x] `key_for_fixture` が fixture の story_id/theme を持つ
      `GoldenKey` を返し、`registry_mismatches` が manifest
      整合を検査する。
- [x] `record_golden_outcome` が `ScenarioOutcome` に golden
      evidence を畳み込み、`run_scenario` 経由で Fake adapter の
      `Unsupported` が正直に記録される。
- [x] `GoldenEvidence` JSONL が `"lane":"pixel-golden"` を含む。
- [x] 既存 golden/scenario/manifest 検査が非回帰。
- [x] mise canonical 検証一式が pass する。

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test`
- [x] `mise run test-ui-golden` / `test-ui-scenario-contract` /
      `test-ui-scenario-contract-fake` /
      `test-ui-scenario-contract-gpui` / `test-ui-driver-contract`
- [x] `mise run check-ui-driver-boundary` /
      `test-ui-test-msrv`
