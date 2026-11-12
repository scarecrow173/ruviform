---
id: "PLAN-5ts8wdqB6w1EhNnXtogfbs"
type: "plan"
status: "approved"
title: "Issue #582 実施計画 — ci 3 lane 失敗の修正と workflow_dispatch 検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/582"
    - "docs/specs/issue-582-ci-lane-failures.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-582-ci-lane-failures-plan.md"
      - type: "file"
        path: "docs/tasks/issue-582-ci-lane-failures.md"
      - type: "file"
        path: "docs/test-specs/issue-582-ci-lane-failures.md"
      - type: "file"
        path: "docs/impl/ir/issue-582-ci-lane-failures.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/specs/issue-582-ci-lane-failures.md"
  implemented-by:
    - "docs/tasks/issue-582-ci-lane-failures.md"
  depends-on:
    - "docs/specs/issue-582-ci-lane-failures.md"
    - "docs/designs/issue-582-ci-lane-failures-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-582-ci-lane-failures.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-582-ci-lane-failures-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-582-ci-lane-failures.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #582 実施計画 — ci 3 lane 失敗の修正と workflow_dispatch 検証

## Goal

run 43270011255 の 3 系統失敗(gpui-verify 冷キャッシュ metadata / offscreen-verify ENOATTR / icc-verify moxcms drift)を修正し、ブランチの `ci` workflow run で対象 4 job の success を確認する。

## Scope

`ci.yml` の ui-gpui-verify への fetch step 追加、`Cargo.lock` の libc pin 復元、`icc.rs` の境界 check + dst-side test、doc-chain 6 件と管理 index 再生成。`consumer-verify.yml`・ci-issue-tracker・他 lane・公開 API は変更しない。

## Upstream Documents

- spec: `docs/specs/issue-582-ci-lane-failures.md`
- design: `docs/designs/issue-582-ci-lane-failures-design.md`
- test-spec: `docs/test-specs/issue-582-ci-lane-failures.md`

## Steps

1. doc-chain(spec/design/plan/task/test-spec + IR)を作成する。
2. `cargo update -p libc --precise 0.2.189` で `Cargo.lock` を pin 復元し、`git log` で 0.2.191 混入経緯(`4bcf1402`)を確定する。
3. moxcms 0.9.1→0.9.2 の差分を registry source で確認し、intent fallback(`get_device_to_pcs`/`get_pcs_to_device`)と encoder 不変(`write_lut_entry`)を特定する。
4. `icc.rs` に `IccIntent::lacks_a_to_b_tag`/`lacks_b_to_a_tag` と `IccTransform::new` の境界 check、dst-side test を実装する。
5. `ci.yml` ui-gpui-verify に `mise exec -- cargo fetch --locked` step を追加する。
6. ローカル検証: `mise run test-video-proc-icc`、`mise run lint-video-proc-icc`、`mise run check-ui-driver-boundary`、`mise exec -- cargo fetch --locked`。
7. `git fetch origin main && git merge origin/main` → 触った docs dir の管理 index を `doc-maintenance` の index apply で再生成。
8. Conventional Commits で commit → `git push -u origin bugfix/582-ci-lanes-failures` → `gh pr create`(`Refs #582`)。
9. `gh workflow run ci.yml --ref bugfix/582-ci-lanes-failures` → `gh run watch` で ui-gpui-verify ×2 / ui-gpui-offscreen-verify / icc-verify の success と新規失敗なしを確認。結果 URL を PR 本文に追記。

## File map

- 変更: `.github/workflows/ci.yml`、`Cargo.lock`、`crates/ruviform-video-proc/src/icc.rs`
- 新規: `docs/{specs,designs,plans,tasks,test-specs}/issue-582-ci-lane-failures*.md`、`docs/impl/ir/issue-582-ci-lane-failures.md`
- 再生成: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md`

## Verification matrix

| 検証 | コマンド | 合格条件 |
| --- | --- | --- |
| icc feature test | `mise run test-video-proc-icc` | exit 0、missing intent tag reject を含む全 test pass |
| icc lint | `mise run lint-video-proc-icc` | exit 0(`-D warnings`) |
| boundary guard | `mise run check-ui-driver-boundary` | verdict: pass |
| dep fetch | `mise exec -- cargo fetch --locked` | exit 0 |
| 実 CI | `gh workflow run ci.yml --ref bugfix/582-ci-lanes-failures` → `gh run watch` | ui-gpui-verify×2 / offscreen / icc-verify = success、新規失敗なし |
| index 整合 | `doc_maintenance.js index apply` | exit 0 |

## Risks

- offscreen lane の Linux compile はローカル(Windows)で再現不能 → CI run を証拠とする。
- `cargo fetch --locked` step が yaml 構文・job 構造を壊す可能性 → actionlint/prek hook と CI run で確認。
- 他 Worker の並行変更による index 競合 → merge 後に再生成して同一コミットに含める。
