---
id: "TSPEC-034S9zD5gJ8lN2qU6xA9eH"
type: "test-spec"
status: "implemented"
title: "consumer-verify tuple証拠契約（解決SHA捕捉・期待照合・status導出・証拠JSON・workflow構造）"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "directory"
        path: "ci/consumer-verify"
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# consumer-verify tuple証拠契約（解決SHA捕捉・期待照合・status導出・証拠JSON・workflow構造）

## Purpose

`consumer-verify.yml` が「候補 tuple を実測で証拠化し、unknown/not_run/unsupported/fail を pass に偽装しない」ことを、ローカル fixture test で保証する。実 runner 検証は別途 dispatch で実施（`gh workflow run --ref <branch>` でブランチ上の変更版も dispatch 可能 — 基準 head `30205f2` で実測済み・run 42302538464）。本 test-spec は判定器と workflow 構造の契約を固定する（先行 test-spec `consumer-verify-workflow-gate-...` の gate 意味論を変更しない）。

## Feature

Cross-Repo 候補 tuple の Consumer 互換検証と構造化証拠。

## Rules

- `classify-ref`: 40-hex → `sha`、branch/tag 安全形 → `ref`、それ以外（`..`・先頭/末尾 `/`・`.lock` 終端・空白・空）→ `invalid`。
- lane 記録: `evidence.mjs lane` が `{name,command,feature,os,outcome,reason}` を JSONL に追記する。outcome 語彙は `pass`/`fail`/`not_run`/`unsupported`/`skipped` のみ。
- status 導出優先順: `GATE=blocked` → `fail`(preflight_failed)、`GATE=not_run` → `not_run`、期待 SHA 提供時に `resolvedSha != expectedSha`（未捕捉含む）→ `fail`(reason=`tuple_mismatch`)、lane `fail` 含有 → `fail`、≥1 `pass` かつ `fail` なし → `pass`、`unsupported` のみ → `unsupported`、その他 → `not_run`。
- `expectedSha` 未提供の `shaMatch` は `null`（照合なしを成功と記録しない）。提供時は `true`/`false`。
- 証拠 JSON: `schemaVersion`・`kind`・`contract`・`run`・`candidate.ruviformRev`・`consumer{requestedRef,resolvedSha,expectedSha,shaMatch}`・`accessMode`・`lanes[]`・`status`・`statusReason`・`coverage`・`generatedAt` を必須キーとし、秘密系 env を転写しない。
- workflow 構造: `on:` は `workflow_dispatch` のみ。`_expected_sha`・`_extra_tasks` 入力・`concurrency`・`git rev-parse HEAD` 解決 SHA 捕捉・`evidence.mjs`・`if: always()` の `upload-artifact`（attempt 固有名）が両 verify job に存在する。実窓/macOS lane は `unsupported` 理由行として記録される。

## Examples

- Given lanes=[{pass},{pass}]、When finalize、Then status=`pass` かつ JSON に lanes 2 件
- Given lanes=[{pass},{fail}]、Then status=`fail`
- Given expectedSha=<a>・resolvedSha=<b>（不一致）、Then status=`fail` reason=`tuple_mismatch`、JSON に両 SHA
- Given lanes=[{unsupported}]、Then status=`unsupported`（pass でも fail でもない）
- Given lanes=[] または全 not_run/skipped、Then status=`not_run`
- Given expectedSha 空、Then `shaMatch=null`
- Given `audiolens_ref=deadbeef…(40hex)`、When classify-ref、Then `sha`（checkout が FETCH_HEAD 経路を選ぶ根拠）

## Guarantees

- 判定器は `mise run test-ci-consumer-verify`（`node --test ci/consumer-verify/`）でオフライン再現可能。
- workflow の `on:` 直子 trigger が `workflow_dispatch` 以外を含むと contract test が fail。
- `not_run`/`unsupported`/`skipped`/`unknown` いずれも `pass` に計上しないことを status 導出の全分岐で固定。

## Non-goals

- 実 runner 上の lane 成否・npm registry 到達・git fetch の実挙動（merge 後 dispatch で観測）。
- scheduler 側の tuple 判定（#367）・tracker 連携（#368）。

## Risk

- 判定器の誤導出を preflight の自己検証（test-ci-consumer-verify）が run ごとに拒否する構成を外すと、壊れた証拠器が誤 status を記録し得る。
