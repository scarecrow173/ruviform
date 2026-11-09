---
id: "PLAN-034Ulf4tsZJz5CatpaE7hG"
type: "plan"
status: "completed"
title: "Issue #294 daily candidate-tuple trigger 実装計画 — decide.mjs tuple モード・daily-ci job・契約 test"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-daily-tuple-trigger.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-294-daily-tuple-trigger.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-294-daily-tuple-trigger.md"
    - "docs/specs/issue-294-daily-tuple-trigger.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-294-daily-tuple-trigger.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-daily-tuple-trigger.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 daily candidate-tuple trigger 実装計画 — decide.mjs tuple モード・daily-ci job・契約 test Implementation Plan

## Goal

`ci/daily/decide.mjs` に tuple モードを追加し、`daily-ci.yml` に `decide-consumer-verify` job を新設して、consumer-verify.yml の `(ruviform SHA, audio-lens SHA, synvelis SHA, consumer-verify-tuple/v1)` tuple を日次で検証済み判定・in-flight 抑制・bounded retry し、未検証時に expected-SHA 入力付きで dispatch する。

## Scope

単一実装 stream。`decide.mjs` tuple モード + `daily-ci.yml` job + 契約/判定 test + consumer-verify 証拠契約 test の追加 + development-environment.md 同期で完結する。consumer-verify.yml 本体・consumer repo・secret/権限は変更しない。

## Upstream Documents

- `docs/specs/issue-294-daily-tuple-trigger.md`（本 slice spec・approved 相当 — review-protocol 通過）
- `docs/designs/issue-294-daily-tuple-trigger.md`（approved）
- `docs/test-specs/issue-294-daily-tuple-trigger.md`（検証契約）

## Design Inputs

- `docs/designs/issue-294-daily-tuple-trigger.md`

## File Map

- `ci/daily/decide.mjs` — tuple モード追加: `resolveConfig` 拡張（`--mode`/`--consumer-repo`/`--tuple-contract`/`--max-evidence-runs`）、`getRepoRefSha`/`listRunArtifacts`/`downloadArtifactZip`/`getRunEvidence` client メソッド、`jsonFromArtifactZip`（stdlib zlib ZIP 抽出）、`decideTuple` 判定、`executeTuple` orchestration、tuple dispatch payload、report/summary tuple 節。既存単一 SHA 経路は非破壊。
- `ci/daily/tests/decide.test.mjs` — tuple 判定行列・executeTuple orchestration・zip 抽出・fixture 拡張 test 追加。既存 test は不変。
- `ci/daily/tests/daily-ci-yml-contract.test.mjs`（新規）— daily-ci.yml job 構造契約。
- `.github/workflows/daily-ci.yml` — `decide-consumer-verify` job 追加。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` — tuple 回復依存契約（artifact 名・evidence field・`if: always()`）の固定 test 追加。
- `docs/development-environment.md` — daily-ci.yml job 構成の同期。

## Implementation Sequence

- [ ] T1: RED — `ci/daily/tests/decide.test.mjs` に tuple 判定・orchestration・zip 抽出の失敗 test を追加（`decideTuple`/`executeTuple`/`jsonFromArtifactZip`/fixture 拡張が未実装で fail することを確認）。
- [ ] T2: GREEN — `ci/daily/decide.mjs` に tuple モード実装（config・client メソッド・zip 抽出・`decideTuple`・`executeTuple`・report/summary・`main()` 分岐）。`mise run test-ci-daily` で全 test 通過。
- [ ] T3: `ci/daily/tests/daily-ci-yml-contract.test.mjs` 新規 + `.github/workflows/daily-ci.yml` に `decide-consumer-verify` job 追加。契約 test 通過。
- [ ] T4: `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` に証拠契約固定 test 追加。`mise run test-ci-consumer-verify` 通過。
- [ ] T5: `docs/development-environment.md` 同期 + impl record（docs/impl/）作成。
- [ ] T6: `git fetch origin main && git merge origin/main` → `mise run fmt-check` / `mise run check` / `mise run test-ci-daily` / `mise run test-ci-consumer-verify` / `mise exec -- actionlint` / `mise exec -- prek run --all-files` 最終検証。

## Dependencies and Risks

- 依存: `consumer-verify.yml` の tuple 証拠契約（main 済み・変更なし前提）。daily 側は `consumer-evidence-{name}-attempt-{n}` artifact の JSON schema に依存 — schema 変更時は契約 test で検出。
- リスク: artifact 期限切れ（90 日）で verified tuple が unverifiable 化 → 再 dispatch（意図的 fail-closed）。evidence 走査上限超過 → `evidence-truncated` hold。in-flight 全面抑制による最大 1 日遅延。`git ls-remote` anonymous HTTPS 依存（private 化時は error outcome で可視化）。
- 単一 SHA モード非回帰が最重要制約 — `decide`/`execute`/既存 client メソッド・既存 config 既定値を変更しない。

## Task Breakdown

- T1+T2: decide.mjs tuple モード（本体実装・test 同梱）
- T3: daily-ci.yml job + 契約 test
- T4: consumer-verify 証拠契約 test
- T5+T6: docs 同期・impl record・最終検証

## Verification

- [ ] `mise run test-ci-daily` — tuple 判定行列・executeTuple orchestration・zip 抽出・既存単一 SHA 非回帰
- [ ] `mise run test-ci-consumer-verify` — 証拠契約固定 test を含む既存 suite
- [ ] `mise exec -- actionlint .github/workflows/daily-ci.yml`（+ 全 workflow）
- [ ] `mise run fmt-check` / `mise run check`
- [ ] `mise exec -- prek run --all-files`
- [ ] 実 dispatch は行わない（fixture のみ。実走は merge 後 — PR で NotRun 明記）

## Open Questions / Gaps

- なし（spec/design で確定済み。consumer repo private 化・secret 経路は対象外として明記）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch のため subagent 不使用）
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol で代替済み — design approved）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: (1) `decideTuple` の verified 三重条件と unverifiable 保守計上、(2) `getRunEvidence` の fail-closed 境界（throw しない unverifiable 化）、(3) dispatch payload の input 名と consumer-verify.yml 契約の一致、(4) 単一 SHA モード非回帰、(5) `daily-ci.yml` job の権限・self-test・attempt 固有 artifact。
