---
id: "SPEC-034Ul9sCnKK1pa64jCWkdO"
type: "spec"
status: "proposed"
title: "Issue #294 daily candidate-tuple trigger — consumer-verify tuple 判定の daily-ci 組込み（tuple-aware decide・証拠 artifact 回復・bounded retry）"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-367-daily-scheduler.md"
  changes:
    added:
      - type: "directory"
        path: "ci/daily"
    modified:
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
      - type: "file"
        path: "ci/daily/decide.mjs"
      - type: "file"
        path: "ci/daily/tests/decide.test.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-367-daily-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/test-specs/issue-294-consumer-verify-tuple-evidence.md"
    - "docs/test-specs/issue-367-daily-scheduler.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-367-daily-scheduler.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
  defers: []
  deferred-by: []
---

# Issue #294 daily candidate-tuple trigger — consumer-verify tuple 判定の daily-ci 組込み（tuple-aware decide・証拠 artifact 回復・bounded retry）

## What Should Be Built

Issue #294 の残 AC「daily 候補 tuple=(Ruviform main SHA, AudioLens main SHA, Synvelis main SHA, 契約版) を既存 GitHub Run・構造化証拠で再取得できる / 同 tuple 成功→skip・いずれか SHA 更新→実行・同 tuple 実行中→重複なし・失敗→上限付き retry」のうち、**daily trigger 側の候補タプル判定と dispatch** を実装する。

1. **tuple-aware 判定器**: `ci/daily/decide.mjs` に tuple モードを追加する。候補 tuple を `(ruviform main SHA, audio-lens main SHA, synvelis main SHA, "consumer-verify-tuple/v1")` として解決し、`consumer-verify.yml` の run 履歴をタプル単位で評価する。既存の単一 SHA 判定（`ci.yml` 用 `decide`/`execute`）は変更せず併存させる。
2. **タプルの回復**: list API は run の `inputs` を返さない（実測済み）ため、各 run の検証 tuple は `consumer-evidence-{audiolens,synvelis}-attempt-<run_attempt>` artifact の JSON（`tuple.ruviformRev` / `tuple.consumerResolvedSha` / `tuple.contract` / `consumer.status`）から回復する。artifact 不在・期限切れ・破損は「未検証」として扱い、pass にしない（Issue #294 本文の契約どおり）。
3. **daily-ci.yml job 追加**: 既存 `decide` job と並ぶ `decide-consumer-verify` job が、条件成立時に `consumer-verify.yml` へ `ref: main` + `ruviform_rev` + `*_ref=main` + `*_expected_sha` 付きで dispatch し、有界 poll で新規 run を確認する。

## Why Now

- `consumer-verify.yml` は tuple 証拠契約（解決 SHA 捕捉・期待 SHA fail-closed 照合・構造化 JSON artifact）を main に持つが、それを**起動する側**がない。現状は手動 dispatch のみで、「同 tuple 成功→skip・SHA 更新→実行・in-flight→抑制・失敗→bounded retry」の日次判定が未実装。
- #367 scheduler（`decide.mjs` + `daily-ci.yml`）は単一 SHA 前提で確定済み。本 slice はその判定器を tuple に一般化し、consumer-verify を 2 本目の dispatch 対象として接続する。
- Issue #294 は umbrella のため本 slice 完了後も Open 継続が前提（PR 本文は `Refs #294`）。

## Users and Value

- **Ruviform 運用者**: 3 repo の main HEAD tuple が未検証・失敗・中断のときだけ consumer-verify を自動起動し、検証済み tuple では runner を消費しない。判定根拠（回復 tuple・証拠 artifact・attempt 表）は Job Summary と report artifact で監査できる。
- **Consumer Owner（AL/SV）**: consumer main が動いた日だけ自動で候補 tuple 検証が走り、Consumer 側の追従 PR 候補と独立 CI の証拠を混同しない。
- **#368 tracker（将来）**: budget-exhausted / hold-unknown の warning + report が失敗 Issue 集約の入力になる。

## Scope

### In Scope

- `ci/daily/decide.mjs`: tuple モード（候補 tuple 解決・証拠 artifact 回復・`decideTuple` 判定・`executeTuple` orchestration・dispatch 入力・confirm poll・report/summary）。単一 SHA 経路は非破壊。
- `ci/daily/tests/decide.test.mjs`: tuple 判定行列・executeTuple orchestration・zip 抽出・fixture 拡張の contract test。
- `ci/daily/tests/` の daily-ci.yml 構造契約 test（tuple job の存在・権限・dispatch 入力・trigger 不変）。
- `.github/workflows/daily-ci.yml`: `decide-consumer-verify` job 追加（既存 `decide` job・trigger・concurrency は不変）。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`: daily tuple 回復が依存する証拠 artifact 契約（artifact 命名・evidence JSON tuple/status フィールド・`if: always()` 出力）の固定テスト追加。
- `.github/workflows/consumer-verify.yml`: **原則変更なし**（既存入力・証拠契約で充足。変更が必要になった場合のみ最小限）。
- 対象 chain docs（spec/design/plan/task/test-spec/impl-record）。

### Out of Scope

- secret / deploy key 設定・required check 化・ruleset 変更（人間/権限作業）。
- consumer リポジトリ（audio-lens / synvelis）内のコード・workflow 変更。
- `crates/` の製品コード。
- 既存 lane の `unsupported`/`not_run` 正直表示の変更（upstream gate が owner）。
- consumer-verify 失敗の自動 Issue 起票（#368 tracker の責務）。
- consumer-verify.yml 自体の lane 追加・変更。
- private consumer repo への credential 経路（anonymous HTTPS のみ。private 化時は別途権限作業）。

## Requirements

- **R1 候補 tuple 解決**: `ruviform main` SHA を既存 `getRefSha` で、`audio-lens main` / `synvelis main` SHA を consumer repo の `refs/heads/main` 解決で取得し、`(rf, al, sv, "consumer-verify-tuple/v1")` を候補とする。consumer SHA 解決は匿名 HTTPS（`git ls-remote` または未認証 API — consumer-verify の anonymous probe と同じ境界）で行い、失敗時は推測せず `error` outcome とする。
- **R2 タプル単位の履歴評価**: `consumer-verify.yml` の run を `head_branch === main` かつ Full CI event allowlist で絞り、各 terminal run の証拠 artifact から検証 tuple を回復する。回復 tuple が候補と一致する run のみを「covering attempt」とし、既存 `decide` の優先順位（in-flight 抑制 > unknown hold > verified skip > bounded retry）をタプルに一般化する。回復不能 run は「unverifiable」として、(a) skip-verified の判定材料にせず、(b) auto actor のものは retry budget に保守的に計上し、(c) 最新 covering attempt より新しい unverifiable run がある場合は skip-verified を `hold-unknown` に倒す（隠れた新規 attempt を見逃さない）。
- **R3 verified の定義**: covering run が `completed`+`success` かつ**両 consumer の evidence `status === "pass"`** のときのみ `skip-verified`。consumer status が `not_run`/`unsupported`/`fail` の run は tuple を検証済みにしない（未検証として retry 候補）。run conclusion のみで verified にしない。
- **R4 bounded retry**: covering run の auto dispatch（actor provenance は既存 `isAutoDispatch` と同一規則）+ unverifiable auto dispatch の合計が `maxAutoDispatches`（既定 2）以上なら `budget-exhausted` で dispatch しない。手動 actor の dispatch は budget を消費しない。
- **R5 in-flight 抑制**: in-scope run に in-flight status が 1 件でもあれば `suppress-in-flight`（tuple 未回復の in-flight run も抑制側 — 別 tuple の可能性を排除できないため保守的）。
- **R6 dispatch 契約**: `decision === dispatch` のとき `POST /actions/workflows/consumer-verify.yml/dispatches` に `{ref: "main", inputs: {ruviform_rev: <rf SHA>, audiolens_ref: "main", synvelis_ref: "main", audiolens_expected_sha: <al SHA>, synvelis_expected_sha: <sv SHA>}}` を送る。dispatch 直前に 3 SHA 全てを再解決し、いずれかが動いていれば再判定（`maxRechecks` 上限、超過は `candidate-moved`）。確認 poll は既存と同型で、新規 `workflow_dispatch` run の出現と着地 `head_sha` を報告する。
- **R7 fail-closed**: API 失敗・malformed response・evidence JSON 破損・tuple 内部不整合（両 consumer evidence の ruviformRev/contract 不一致、`run.runId` 不一致）は verified に寄与しない。証拠 artifact の走査が上限を超える場合は不完全履歴として hold（`evidence-truncated` 系 outcome）し、dispatch/skip を確定しない。exit code 規約は既存と同じ（決定完遂 outcome は 0、`error` は 1）。
- **R8 daily-ci.yml**: `decide-consumer-verify` job を追加し、既存 `decide` job と同じ最小権限（`contents: read` + `actions: write`）・self-test 先行・report artifact（attempt 固有名）を持つ。workflow の `on:` trigger・concurrency・既存 job は変更しない。
- **R9 非破壊**: 単一 SHA モード（`ci.yml` 向け `decide`/`execute`/`resolveConfig` 既定）は既存テストを全て通し続ける。tuple モードは明示 opt-in（`--mode tuple` / `DAILY_MODE=tuple` + consumer repo 設定）。
- **R10 検証**: `mise run test-ci-daily`・`mise run test-ci-consumer-verify`・`mise run fmt-check`・`mise run check`・`actionlint`・`prek run --all-files` が pass。実 dispatch は行わない（fixture のみ）。

## Acceptance Criteria

- [ ] (構造) `decide.mjs` が tuple モードを持ち、候補 tuple 解決・証拠 artifact 回復・`decideTuple` 判定・`executeTuple` orchestration・tuple 用 dispatch 入力を実装し、単一 SHA 経路の既存 contract test が無変更で pass する。
- [ ] (構造) tuple 判定行列（covering success+両 pass→skip-verified / in-flight→suppress / failure→bounded dispatch / auto dispatch 上限→budget-exhausted / unverifiable auto 計上 / unverifiable newer→hold-unknown / artifact 不在・期限切れ→未検証扱い / consumer status 非 pass→非 verified）が `mise run test-ci-daily` で pass。
- [ ] (構造) `daily-ci.yml` が `decide-consumer-verify` job を持ち、`actions: write` 権限・self-test・report artifact・`on:` が schedule+workflow_dispatch のみであることを contract test が保証。
- [ ] (構造) `consumer-verify-yml-contract.test.mjs` が daily tuple 回復の依存契約（artifact 命名規則・evidence JSON の tuple/status フィールド・全終端経路 artifact 出力）を固定する。
- [ ] (構造) `actionlint`（daily-ci.yml 含む全 workflow）と `prek run --all-files` が pass。
- [ ] (実走・後続) merge 後の daily schedule または手動 `workflow_dispatch` で、候補 tuple の判定 report（回復 tuple・covering attempts・unverifiable・dispatch 入力）が artifact/Job Summary から監査でき、条件成立時に `consumer-verify.yml` run が `ruviform_rev`+`*_expected_sha` 入力付きで起動することを `gh api` で確認する（本 PR では NotRun — workflow_dispatch 専用のため PR 時点で実 runner 検証不可）。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 |
| --- | --- | --- |
| consumer-verify 失敗の自動 Issue 起票・tracker 監視対象拡大 | #368 tracker の責務境界 | Issue #368 |
| private consumer repo の credential 経路（deploy key を daily job へ拡張） | secret/権限は人間作業・anonymous HTTPS で現行 public repo は充足 | Issue #294 本体 |
| GUI-VISUAL/OS・PLUGIN-NATIVE-DOCK・macOS lane の `unsupported` 行変更 | upstream gate が owner | Issue #294 本体 |
| tuple 判定の consumer repo 側独立 Daily 証拠の再利用 | 「完全に同一候補 revision・条件」の証拠再利用判定は別設計 | Issue #294 本体 |
