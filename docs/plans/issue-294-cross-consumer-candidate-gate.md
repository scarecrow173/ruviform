---
id: "PLAN-034S9vX2cF6hK9mP3rT7wZ"
type: "plan"
status: "proposed"
title: "Issue #294 Cross-Consumer候補tuple Gate 実装計画 — evidence script・workflow 拡張・契約 test"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  changes:
    added:
      - type: "directory"
        path: "ci/consumer-verify"
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  implemented-by:
    - "docs/tasks/issue-294-t1-consumer-verify-evidence-script.md"
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  depends-on:
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  derived-by:
    - "docs/tasks/issue-294-t1-consumer-verify-evidence-script.md"
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
    - "docs/test-specs/issue-294-consumer-verify-tuple-evidence.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-consumer-verify-tuple-evidence.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 Cross-Consumer候補tuple Gate 実装計画

## Goal

`consumer-verify.yml` に候補 tuple の証拠契約（解決 SHA 捕捉・期待 SHA 照合・構造化 JSON artifact）と、両 Consumer 自身の mise 入口による UI headless lane・追加 task 接続点を実装する。判定ロジックは `ci/consumer-verify/evidence.mjs` に単一権威として集約し、fixture test と workflow 構造契約 test で固定する。

## Scope

In: `ci/consumer-verify/`（script+tests）、`.github/workflows/consumer-verify.yml`、`mise.toml` task、`docs/development-environment.md`、chain docs。
Out: `ci/daily`・`ci-issue-tracker*`・`ci.yml`・Consumer repo。

## Upstream Documents

- `docs/specs/issue-294-cross-consumer-candidate-gate.md`
- `docs/designs/issue-294-cross-consumer-candidate-gate.md`
- 既存 gate 意味論: `docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md`

## Design Inputs

- evidence.mjs の 3 コマンド（classify-ref / lane / finalize）と status 導出優先順
- verify job への `actions/checkout`@`$RUVIFORM_REV` 追加（script 供給 + 候補存在の早期 fail）
- fetch＋checkout FETCH_HEAD による ref/SHA 統一 checkout + `git rev-parse HEAD` 捕捉
- lane 集合: AL=check-ruviform-lock/check-ci/smoke/test-ci/sync-ui-deps/check-ruviform-rev --require-vendor/setup-ui/check-ui/test-ui、SV=check-ruviform-rev/check/test/test-rt/check-headless-deps/sync-ui-deps/setup-ui/check-ui/test-ui/build-ui/check-ruviform-rev --require-vendor
- `unsupported` 固定行（実窓 host・macOS/実 OS lane）
- extra_tasks 入力 → 個別 lane 記録
- upload-artifact で JSON 証拠、summary job に tuple 表

## File Map

| file | 変更 | 内容 |
| --- | --- | --- |
| `ci/consumer-verify/evidence.mjs` | 新規 | classify-ref/lane/finalize、status 導出、証拠 JSON+Markdown 生成 |
| `ci/consumer-verify/tests/evidence.test.mjs` | 新規 | 判定・schema・summary fixture test |
| `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` | 新規 | workflow 構造契約（textual、ci-yml-contract.test.mjs 型） |
| `.github/workflows/consumer-verify.yml` | 変更 | inputs/concurrency/preflight 自己検証/checkout+fetch 経路/lane 拡張/artifact/summary tuple 表 |
| `mise.toml` | 変更 | `test-ci-consumer-verify` 追加 |
| `docs/development-environment.md` | 変更 | task 対応表追記 |

## Implementation Sequence

1. T1: `ci/consumer-verify/` script + tests + mise task（先に RED→GREEN で判定器を固める）
2. T2: workflow 拡張（inputs→preflight→verify 両 job→summary）+ yml contract test + `docs/development-environment.md`
3. 検証: `mise run test-ci-consumer-verify`、`actionlint`、`prek run --all-files`
4. IR doc・index 再生成・main merge・最終検証

## Dependencies and Risks

- Consumer mise task 名に依存（現行 checkout で実測済み一覧。将来の改名は lane fail/not_run として可視化）。
- `git fetch origin <sha>` は GitHub の unadvertised-object 受理で広く機能（GC 済み・未 push SHA は clone fail=正直な証拠）。
- workflow_dispatch は default branch の定義を使うため、workflow 実走検証は merge 後（NotRun として証拠表に記録）。
- npm ci は consumer repo の package-lock 依存（runner 上で registry 到達が前提。失敗は lane fail として記録）。

## Verification Matrix

| 検証 | 入口 | 対象 |
| --- | --- | --- |
| fixture test | `mise run test-ci-consumer-verify` | status 導出・schema・summary・yml 契約 |
| 構文 | `mise exec -- actionlint .github/workflows/consumer-verify.yml` | workflow 構文・式 |
| hygiene | `mise exec -- prek run --all-files` | yaml/whitespace/secret |
| 実 runner | merge 後 workflow_dispatch | 本 PR では NotRun |

## Rollback

変更は workflow + 新規 ci/ スクリプトのみ。revert で従来 lane 構造へ即復帰（既存 lane 意味論は変えない）。Consumer repo・scheduler・tracker への影響なし。

## Task Breakdown

- `docs/tasks/issue-294-t1-consumer-verify-evidence-script.md`
- `docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md`
