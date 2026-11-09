---
id: "IMPL-034UlznQ9G787VMPYNa3S3"
type: "impl"
status: "completed"
title: "Issue #294 daily candidate-tuple trigger（decide.mjs tuple mode + daily-ci decide-consumer-verify job）"
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
      - type: "workflow"
        path: ".github/workflows/daily-ci.yml"
      - type: "workflow"
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
    - "docs/tasks/issue-294-daily-tuple-trigger.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-294-daily-tuple-trigger.md"
    - "docs/plans/issue-294-daily-tuple-trigger.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-294-daily-tuple-trigger.md"
    - "docs/test-specs/issue-294-daily-tuple-trigger.md"
    - "docs/impl/ir/issue-294-consumer-verify-tuple-evidence.md"
    - "docs/designs/issue-367-daily-scheduler.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-294-daily-tuple-trigger.md"
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

# Issue #294 daily candidate-tuple trigger（decide.mjs tuple mode + daily-ci decide-consumer-verify job）

## Summary

`ci/daily/decide.mjs` に `--mode tuple`（`DAILY_MODE=tuple`）を追加し、daily scheduler の検証単位を単一 SHA から候補 tuple `(ruviform main SHA, audio-lens main SHA, synvelis main SHA, consumer-verify-tuple/v1)` へ一般化した。`.github/workflows/daily-ci.yml` に `decide-consumer-verify` job を追加し、条件成立時に `consumer-verify.yml` へ `ruviform_rev` + `<consumer>_ref`/`<consumer>_expected_sha` 付きで dispatch する。Actions run API は dispatch inputs を公開しないため、各 run が検証した tuple は `consumer-evidence-<consumer>-attempt-<run_attempt>` artifact の証拠 JSON から復元する fail-closed 経路を実装した。単一 SHA mode（既定 `single`）の挙動は不変。

## Context

- 実施タスク: `docs/tasks/issue-294-daily-tuple-trigger.md`（`docs/plans/issue-294-daily-tuple-trigger.md` 配下）
- 上流: `docs/specs/issue-294-daily-tuple-trigger.md`・`docs/designs/issue-294-daily-tuple-trigger.md`・`docs/test-specs/issue-294-daily-tuple-trigger.md`
- 先行実装: `docs/impl/ir/issue-294-consumer-verify-tuple-evidence.md`（証拠契約の生産側）・`docs/designs/issue-367-daily-scheduler.md`（scheduler 本体）
- review-protocol: dispatch 制約で独立 subagent を起動できないため、reviewer/validator/judge の役割を分離した通過として design 段で adversarial review を実施し blocker なしで approved 化（PR 本文に記録）

## Implementation

### Changes

- `ci/daily/decide.mjs` — tuple mode を追加。`resolveTupleCandidate`（Ruviform は既存 `git/ref` API、Consumer main は匿名 `git ls-remote https://github.com/<owner>/<repo>.git refs/heads/main` を `parseLsRemote` で厳格 1 行解釈 — 失敗/0 件/複数/非 SHA は `ApiError`→`error`）、`scanTupleHistory`（`consumer-verify.yml` 履歴を scope→in-flight 分離→終端 run を `DAILY_MAX_EVIDENCE_RUNS`（既定 60）で有界化→`getRunEvidence` で artifact 証拠を復元→`classifyRunEvidence` で covering/non-target/unverifiable に分類）、`decideTuple`（`decide()` と同じ語彙の決定行列 — verified=success+全 consumer `status==="pass"`+tuple 完全一致、unverifiable は retry 計上に保守的に算入、in-flight は `suppress-in-flight`）、`executeTuple`（決定→dispatch 前の tuple 全要素再解決（`sameTuple`・`maxRechecks` 有界）→`ruviform_rev`+`*_ref`/`*_expected_sha` inputs で dispatch→新規 run 出現を有界 poll→着地 `head_sha` 照合）。ZIP は `jsonFromArtifactZip` が EOCD→central directory→local header を stdlib で走査し stored/deflate を `node:zlib` で展開（data descriptor 不可・サイズは central directory 由来）。artifact zip DL は 302 `Location` を手動追従し token を署名 URL へ転送しない。report は `kind: "daily-ci-tuple-decision"` で tuple/evidence/covering/unverifiable/dispatch/warnings を含み、`markdownSummary` が tuple 節を描画。fixture client は `repoRefReads`/`repoRefErrors`/`runArtifacts`（実 ZIP bytes 生成の `buildStoredZip` + `__missing__`/`__expired__`/`__malformed__`/`__badzip__` マーカー）をサポート。CLI は `--mode`/`--consumer-repo`（初回指定で env 既定を置換）/`--tuple-contract`/`--max-evidence-runs`。
- `ci/daily/tests/decide.test.mjs` — tuple テスト群を追加（decideTuple 行列・classifyRunEvidence 分類・ZIP stored/deflate/選択/異常系・parseLsRemote・parseConsumerRepos・executeTuple orchestration: skip-verified/dispatch-confirmed 全 inputs/in-flight 抑制/missing・expired・malformed・badzip 証拠/manual vs auto retry/budget-exhausted/tuple 移動再評価/candidate-moved/解決失敗 error/history-truncated/evidence-truncated/landed-mismatch/unconfirmed/dry-run/summary 描画）。fixture 駆動・実 API 呼出しなし。
- `ci/daily/tests/daily-ci-yml-contract.test.mjs` — 新設。両 job の存在・tuple job の self-test 先行・env pin（`DAILY_MODE=tuple`/`DAILY_TARGET_WORKFLOW=consumer-verify.yml`/`DAILY_CONSUMER_REPOS`/`DAILY_TUPLE_CONTRACT`/`DAILY_MAX_*`/`DRY_RUN`）・`--mode tuple` 明示・report artifact 名非衝突・最小権限・`CONSUMER_REPO` 値との repo map 一致を固定。
- `.github/workflows/daily-ci.yml` — `decide-consumer-verify` job を追加（`contents: read`+`actions: write`、timeout 20 分、self-test→tuple 判定→`daily-ci-tuple-report-attempt-<n>` artifact）。`dry_run` 入力説明を両 workflow 対象に更新。
- `.github/workflows/consumer-verify.yml` — 両 verify job の Record evidence step env に `RUVIFORM_REV: ${{ inputs.ruviform_rev || github.sha }}` を追加。consumer task が `$GITHUB_ENV` で tuple 識別値を偽装し得る経路に対し、daily tuple 復元が信頼する `tuple.ruviformRev` を既存の pin 機構（EXPECTED_SHA/REQUESTED_REF/CONSUMER_NAME/CONSUMER_REPO/RESOLVED_SHA）と同列で固定。lane 構成・`unsupported`/`not_run` 行の意味は不変。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` — result step pin 検査に `RUVIFORM_REV`/`RESOLVED_SHA` を追加、artifact 名検査を `consumer-evidence-<consumer>-attempt-${{ github.run_attempt }}` の consumer 完全一致に強化。
- `docs/development-environment.md` — daily-ci.yml 節に tuple job の判定・復元・fail-closed 契約を追記。

### Decisions During Implementation

- **tuple の出所は証拠 artifact のみ**: run API は dispatch inputs を露出しないため、`consumer-evidence-<consumer>-attempt-<n>` artifact JSON の `tuple.*`/`consumer.*`/`run.runId` を唯一の復元源とした。証拠なし・期限切れ・破損・内部不整合は全て `unverifiable`（verified にも無視にもせず、auto actor なら retry budget に保守的に計上）。
- **non-target vs unverifiable の区別**: 証拠が完全に読めて tuple が candidate と異なる run は `non-target`（証明済みの別 tuple — 計上しない）。証拠が欠ける/壊れる run は `unverifiable`（candidate tuple の可能性を排除できない — 保守側に計上）。
- **`classifyRunEvidence` の consumer 一覧は `candidate.consumers` が正本**: config consumer（`{name, repo}`）は sha を持たないため、一致判定は解決済み sha を持つ candidate 側で行う（初版は config 側を参照し全件 non-target になる bug — fixture テストで検出・修正）。
- **zip は最小自前 reader**: 依存追加を避けるため EOCD/central directory/local header を stdlib で走査。data descriptor・zip64・method 0/8 以外は `ApiError`→unverifiable。実運用の upload-artifact v4 zip（deflate）をカバーするため fixture 側に deflate zip 生成テストを追加。
- **`--consumer-repo` flag は env 既定を置換**: append すると env 既定と明示 flag が暗黙 union されるため、初回 flag 出現で env 由来リストをクリア。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

<!-- なし -->

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| daily 判定 contract | `mise run test-ci-daily` | **111 pass / 0 fail**（node --test ci/daily/ — decide 95 + ci-yml-contract + daily-ci-yml-contract） |
| consumer-verify contract | `mise run test-ci-consumer-verify` | **64 pass / 0 fail** |
| workflow 構文 | `mise exec -- actionlint .github/workflows/daily-ci.yml .github/workflows/consumer-verify.yml` | exit 0 |
| fmt | `mise run fmt-check` | exit 0 |
| check | `mise run check`（cargo check --workspace --all-targets） | exit 0 |
| hygiene | `mise exec -- prek run --all-files` | 全 hook pass |
| consumer repo 匿名解決 | `git ls-remote https://github.com/scarecrow173/{audio-lens,synvelis}.git refs/heads/main` | 両 repo とも 40-hex SHA を返却（匿名 HTTPS 到達可能を実測） |

## Risks

- **実 dispatch は未実施**: fixture ベースの実装であり、実 GitHub API に対する tuple 判定・dispatch は merge 後の daily/手動 `workflow_dispatch`（`dry_run` 推奨）で初めて実測される。artifact zip の実フォーマット（upload-artifact v4）が parser 前提（EOCD+central directory+method 0/8）を外れる場合は unverifiable 側に倒れ fail-closed。
- **証拠 artifact の保持期限**: GitHub の artifact retention を超えた run は証拠復元不可 → unverifiable として retry 計上。長期未検証 tuple で auto budget が枯渇し得る（保守的挙動・意図どおり）。
- **`unsupported`/`not_run` consumer を含む covering run は verified にならない**: GUI-VISUAL/OS・PLUGIN-NATIVE-DOCK・macOS の固定 `unsupported` 行を持つ現行 consumer-verify では、全 consumer `status==="pass"` が成立するかは consumer 側 lane 実装に依存する — 本 slice は判定器のみで、consumer 側の lane 追加は対象外（upstream gate が owner）。

## Follow-ups

- merge 後に `daily-ci` workflow を手動 `workflow_dispatch`（`dry_run: true` 推奨）で tuple 判定を実測し、`daily-ci-tuple-report-attempt-<n>` artifact で outcome を確認する。
- Issue #294 は umbrella — 本 slice 完了後も Open 継続（PR は `Refs #294`）。
