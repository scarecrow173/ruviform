---
id: "TASK-034UlgdtbhGVBT1saWLNNP"
type: "task"
status: "done"
title: "Issue #294 daily tuple trigger 実装 — decide.mjs tuple モード・daily-ci job・契約 test"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "file"
        path: "ci/daily/tests/daily-ci-yml-contract.test.mjs"
    modified:
      - type: "file"
        path: "ci/daily/decide.mjs"
      - type: "file"
        path: "ci/daily/tests/decide.test.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-294-daily-tuple-trigger.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-294-daily-tuple-trigger.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-daily-tuple-trigger.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 daily tuple trigger 実装 — decide.mjs tuple モード・daily-ci job・契約 test

## Work

`ci/daily/decide.mjs` に tuple モードを追加する — `--mode tuple` / `DAILY_MODE=tuple` で `executeTuple` に分岐し、candidate tuple（ruviform main SHA + consumer main SHA 群 + `consumer-verify-tuple/v1`）を解決、consumer-verify.yml の run 履歴から evidence artifact（`consumer-evidence-{name}-attempt-{n}`）を回復して tuple 一致を判定し、`decideTuple` の優先順位（in-flight 抑制 → hold → verified skip → budget → dispatch）で `consumer-verify.yml` を `ruviform_rev` + `*_ref` + `*_expected_sha` 入力付きで dispatch する。`.github/workflows/daily-ci.yml` に `decide-consumer-verify` job を追加し、`test-ci-daily`・report artifact・`actions: write` 権限で配線する。単一 SHA モードは非破壊。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-294-daily-tuple-trigger.md`
- `docs/designs/issue-294-daily-tuple-trigger.md`
- `docs/specs/issue-294-daily-tuple-trigger.md`
- `docs/test-specs/issue-294-daily-tuple-trigger.md`

## Dependencies

- `consumer-verify.yml` の tuple 証拠契約（main 済み・変更なし前提）
- `ci/daily/decide.mjs` 既存の `decide`/`execute`/`createGitHubClient`/`createFixtureClient`（単一 SHA モード — 非破壊で拡張）

## Affected Files

- `ci/daily/decide.mjs`
- `ci/daily/tests/decide.test.mjs`
- `ci/daily/tests/daily-ci-yml-contract.test.mjs`（新規）
- `.github/workflows/daily-ci.yml`
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`
- `docs/development-environment.md`

## Done When

- [ ] `decideTuple`/`executeTuple`/`jsonFromArtifactZip`/fixture 拡張が実装され `mise run test-ci-daily` が全件通過（既存単一 SHA test を含む）
- [ ] `daily-ci.yml` に `decide-consumer-verify` job が追加され契約 test が通過
- [ ] consumer-verify 証拠契約（artifact 名・JSON field・`if: always()`）の固定 test が追加され `mise run test-ci-consumer-verify` が通過
- [ ] `docs/development-environment.md` の daily-ci.yml 記述が新 job を反映
- [ ] impl record（docs/impl/）が作成される

## Verification

- [ ] `mise run test-ci-daily`
- [ ] `mise run test-ci-consumer-verify`
- [ ] `mise exec -- actionlint .github/workflows/daily-ci.yml`
- [ ] `mise run fmt-check` / `mise run check`
- [ ] `mise exec -- prek run --all-files`
