---
id: "TASK-65Lh1QvTgy1yhrYEwfozUi"
type: "task"
status: "done"
title: "Issue #294 consumer oracle lanes — consumer-verify 両 verify job への rf628 oracle lane 配線"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-294-consumer-oracle-lanes.md"
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-294-consumer-oracle-lanes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-294-consumer-oracle-lanes.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-294-virtual-first-evidence-schema.md"
    - "docs/tasks/issue-294-t3-gui-contract-lane-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-294-consumer-oracle-lanes.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-consumer-oracle-lanes.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 consumer oracle lanes — consumer-verify 両 verify job への rf628 oracle lane 配線

## Work

1. `.github/workflows/consumer-verify.yml` の両 verify job に `oracle_probe`（canonical file 存在 → `oracle`/`oracle_rt` output）と run step（`git lfs pull` + `mise run test <pkg> --test <file>`）を追加。AL は `al-analysis`/`rf628_oracle` 1 file、SV は `synvelis-recording`/`rf628_oracle` + `rf628_oracle_rt_alloc` 2 file（file 粒度）。
2. 両 job の Record step に `rec()` spec 拡張（field 6=`oracle`・field 7=`fixture_version`）+ `FIXTURE_VER` 抽出（`$CONSUMER_DIR/Cargo.lock` の `ruviform-audio-test` version + `@<RUVIFORM_REV>`）+ oracle lane 行（3 分岐: probe=1→run outcome / probe=0→owner 付き not_run / else→probe outcome）を追加。
3. workflow ヘッダ comment に oracle lane の説明を追記。
4. `consumer-verify-yml-contract.test.mjs` の `CASE_SPEC_RE`・`rshaRows`・新規 oracle pin を更新。
5. `docs/development-environment.md` :131 の lane 列挙へ oracle lane を追記（他節不変）。
6. doc chain（本 slug）と `evidence/rf294/` 検証記録を仕上げる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-294-consumer-oracle-lanes.md`
- `docs/specs/issue-294-consumer-oracle-lanes.md`
- `docs/designs/issue-294-consumer-oracle-lanes.md`
- `docs/test-specs/issue-294-consumer-oracle-lanes.md`

## Dependencies

- 着地済み schema v2（`ci/consumer-verify/evidence.mjs` — optional `oracle`/`fixtureVersion`/`resolvedRuviformSha` 受理済み）
- consumer oracle test merge 済み（AL `scarecrow173/audio-lens#127`・SV `scarecrow173/synvelis#115` — canonical file 実在確認済み）
- gui_candidate probe→run→record 先例（`.github/workflows/consumer-verify.yml`）

## Affected Files

- `.github/workflows/consumer-verify.yml`
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`
- `docs/development-environment.md`（consumer-verify 節の lane 列挙のみ）
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir}/issue-294-consumer-oracle-lanes.md`（docs index は Coordinator が統合時に再生成）
- `evidence/rf294/`（検証記録）

## Done When

- [ ] 両 verify job が oracle lane を持ち、file 粒度 lane 行（AL 1・SV 2、各 3 分岐）に `--oracle`/`--resolved-ruviform-sha`/`--fixture-version` を記録
- [ ] target 不在は owner 付き `not_run`、probe/上流未完了はその outcome を正直に写す（偽 pass なし）
- [ ] `mise run test-ci-consumer-verify`・`mise exec -- actionlint .github/workflows/consumer-verify.yml`・`mise exec -- prek run --all-files` が pass、補助 `mise exec -- node --test ci/`
- [ ] `docs/development-environment.md` lane 列挙へ追記・doc chain + `evidence/rf294/` 整合
- [ ] Issue レベル AC の対象外項目（per-case emitter・実 OS lane・GUI fixture・contract v2・post-merge dispatch）を deferred/gate として記録

## Verification

- [ ] `mise run test-ci-consumer-verify`
- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml`
- [ ] `mise exec -- prek run --all-files`
- [ ] 補助（canonical=mise）: `mise exec -- node --test ci/` で `evidence.mjs` export consumer の横断退行確認
- [ ] workflow 実走は NotRun（post-merge dispatch は Coordinator 所有）
