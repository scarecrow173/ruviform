---
id: "TASK-034R4Jx09ei8vexAKp8oe7"
type: "task"
status: "in-progress"
title: "T5 consumer-verify anonymous-HTTPS access mode（public repo 対応）+ 棚卸し refresh"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T5 consumer-verify anonymous-HTTPS access mode（public repo 対応）+ 棚卸し refresh

## Work

設計前提「3 repo とも private」が変化した（2026-11-03 実測: ruviform / audio-lens / synvelis すべて public、匿名 `git ls-remote` HTTPS 成功）。`CROSS_REPO_DEPLOY_KEY` 未設定では全 lane `not run` で証拠ゼロのまま滞留するため、public repo 期間は匿名 HTTPS で lane を実行できる access mode fallback を `consumer-verify.yml` に追加する。credential が設定されていれば従来どおり Deploy Key 経路を優先する（private 復帰に備えて経路は残す）。

- `preflight` が `credential_configured` に加えて per-consumer の匿名 probe（`git ls-remote https://…{consumer,ruviform}.git HEAD`）を出力する。
- 各 verify job が `mode`（`deploy_key` / `anonymous` / `not_run`）を解決し、SSH 鍵 install・`url.insteadOf` 書換・cleanup は `deploy_key` のみ、clone/probe は mode に応じて SSH/HTTPS を切替える。
- 証拠表に access mode を記録する（どの経路で検証したか追跡可能にする）。
- 棚卸し refresh: audio-lens `rev` pin が `e9e6abe`（ruviform HEAD）へ前進・16 dep、synvelis が 8 member / 13 ruviform git dep / lock `258df5a` / mise `rust=1.99.0` 追加済み、両 repo public 化を discovery dated snapshot に追記する。
- trigger/cadence（手動 dispatch の運用契機）と rollback 経路を design/runbook に明記する。

## Classification

- Type: implementation
- Size: M

## Upstream Documents

- `docs/plans/issue-219-consumer-cross-repo-ci-gate.md`
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`
- `docs/specs/issue-219-consumer-compat-contract.md`

## Dependencies

なし（credential 一次設定待ちの open blocker を迂回する補完経路）

## Affected Files

- `.github/workflows/consumer-verify.yml`
- `docs/discovery/issue-219-consumer-cross-repo-ci.md`（dated snapshot 追記）
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`（access mode・trigger/cadence 追記）
- `docs/impl/ir/issue-219-consumer-verify-anonymous-https-access-mode-refresh.md`

## Done When

- access mode の 3 値（deploy_key / anonymous / not_run）が gate で正しく分岐し、credential 未設定 + public repo で lane が匿名 HTTPS 実行される
- `deploy_key` 経路の既存構造（known_hosts pin・url.insteadOf・cleanup）を維持
- 証拠表に access mode が記録される
- 棚卸し dated snapshot と trigger/cadence・rollback の記録が docs に反映
- `actionlint` pass

## Verification

- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml`
- [ ] ローカル scratch 検証: 両 consumer clone → manifest 正規化 → `cargo fetch` 単一 SHA guard → check/test lane
- [ ] `workflow_dispatch`（本ブランチ ref）で anonymous mode の実走証拠を確認
