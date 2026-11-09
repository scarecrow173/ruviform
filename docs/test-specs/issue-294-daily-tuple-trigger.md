---
id: "TSPEC-034Ule6zLxK0at8L5q0W7q"
type: "test-spec"
status: "proposed"
title: "Issue #294 daily candidate-tuple trigger test spec — tuple 判定・証拠回復・dispatch 契約"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "docs/specs/issue-294-daily-tuple-trigger.md"
    - "docs/designs/issue-294-daily-tuple-trigger.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ci/daily/decide.mjs"
      - type: "file"
        path: "ci/daily/tests/decide.test.mjs"
      - type: "file"
        path: "ci/daily/tests/daily-ci-yml-contract.test.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-daily-tuple-trigger.md"
    - "docs/designs/issue-294-daily-tuple-trigger.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-294-daily-tuple-trigger.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-294-daily-tuple-trigger.md"
    - "docs/designs/issue-294-daily-tuple-trigger.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 daily candidate-tuple trigger test spec — tuple 判定・証拠回復・dispatch 契約

## Purpose

`ci/daily/decide.mjs` の tuple モードと `daily-ci.yml` の `decide-consumer-verify` job が「(ruviform SHA, audio-lens SHA, synvelis SHA, consumer-verify-tuple/v1) の 4 要素を単位に consumer-verify.yml の検証済み判定・in-flight 抑制・bounded retry を fail-closed で行う」契約を守ることを保証する。spec/design の受入条件が変わり tuple の構成要素・証拠形式・判定優先順位が再設計された場合に本 test spec を見直す。consumer-verify.yml 側の既存契約（lane 語彙・unsupported 正直行・証拠 schema）は別 test spec（issue-294-consumer-verify-tuple-evidence）の管轄であり、本書は「daily 側が依存する証拠形状の契約固定」と「daily 側判定の行列」を対象とする。

## Feature

Daily scheduler tuple 判定 — candidate tuple 解決・consumer-verify run 履歴からの証拠 artifact 回復・tuple 一致判定・bounded retry・dispatch payload・pre-dispatch 再解決・confirmation poll。

## Rules

- Rule: tuple の verified 判定は三重条件 — (a) run が API 上 completed+success、(b) 新しさの一意性（ambiguity contender がいない）、(c) 両 consumer の証拠 `consumer.status === "pass"`。run conclusion success だけ・片方 pass だけ・証拠欠落では verified にしない。
- Rule: 証拠回復不能（artifact 不在・期限切れ・zip/JSON 破損・内部不整合・runId 不一致・contract 不一致）は unverifiable であり、verified の材料にせず skip-verified を主張しない。unverifiable の auto dispatch は budget に保守計上する。
- Rule: in-flight run（証拠未到達で tuple を証明できないものを含む）は常に dispatch 抑制側。別 tuple の in-flight があっても抑制する。
- Rule: 判定優先順位は 1) in-flight 抑制 → 2) unknown/最新性不明の covering で hold → 3) verified skip → 4) budget 判定 → 5) dispatch の順で固定。covering が無く unverifiable のみなら dispatch/budget-exhausted に進む（unverifiable は dispatch を妨げない）。
- Rule: unverifiable run が最新 covering attempt より新しい（または順序不明）場合、skip-verified は hold-unknown に倒す — 隠れた新規 attempt を見逃して偽 verified を記録しない。
- Rule: dispatch payload は `ref: main` と inputs `{ruviform_rev: <RF SHA>, audiolens_ref: main, synvelis_ref: main, audiolens_expected_sha: <AL SHA>, synvelis_expected_sha: <SV SHA>}` を厳密に持つ。dispatch 直前に 3 SHA を再解決し、変化していれば再評価、上限超過で `candidate-moved`。
- Rule: API 失敗・malformed response・consumer SHA 解決失敗・evidence 走査上限超過は error/hold で閉じ、推測 tuple で dispatch しない。
- Rule: 単一 SHA モード（`decide`/`execute`・`--workflow ci.yml` 経路）は tuple 追加後も既存 contract test 全件を変更なしで通す — 既存呼び出しを壊さない。
- Rule: daily-ci.yml の `decide-consumer-verify` job は `actions: write`・self-test（`test-ci-daily`）・attempt 固有 report artifact・`DAILY_MODE: tuple`・`DAILY_TARGET_WORKFLOW: consumer-verify.yml` を持ち、既存 `decide` job（`DAILY_TARGET_WORKFLOW: ci.yml`）と独立して並走する。
- Rule: daily 側の証拠回復が依存する consumer-verify 側契約 — artifact 名 `consumer-evidence-<consumer>-attempt-<run_attempt>`・evidence JSON の `kind: consumer-verify-evidence`・`tuple.{ruviformRev,consumerResolvedSha,contract}`・`consumer.{name,status}`・`run.runId` — は workflow 構造契約 test で固定し、consumer-verify.yml の変更が tuple 回復を壊す場合に検出する。

## Examples

- Example: 同一 tuple で両 consumer pass の成功 run が最新 → `skip-verified`、dispatch なし。
- Example: AudioLens main が 1 commit 進み candidate tuple が変化 → 履歴に一致 run なし → `dispatch`（budget 内）。
- Example: 同一 tuple の in-flight run が存在 → `suppress-in-flight`（証拠未到達でも抑制）。
- Example: 同一 tuple の最新 covering run が failure、auto dispatch 1 回済み（上限 2）→ `dispatch`（bounded retry）。2 回 auto 済み → `budget-exhausted`。
- Example: 成功 run の artifact が期限切れ/欠落 → unverifiable → 当該 run を verified にせず、auto actor なら budget 計上、手動なら非計上で dispatch 判定へ。
- Example: consumer status が `not_run`/`unsupported`/`fail` の success run → 部分検証であり tuple verified にしない → retry 判定へ。
- Example: dispatch 直前の再解決で Synvelis SHA が変化 → candidate 更新して再評価、`maxRechecks` 超過で `candidate-moved`。
- Example: artifact zip が deflate method・stored method の両方で JSON を回復できる（`jsonFromArtifactZip`）。
- Example: API が malformed JSON や 404 を返す → `ApiError` → `error` outcome、dispatch しない。

## Guarantees

- `decideTuple` の全 decision 枝（skip-verified / suppress-in-flight / hold-unknown / budget-exhausted / dispatch）が fixture 行列で再現可能。
- `executeTuple` が candidate 解決 → 履歴走査 → 証拠回復 → 判定 → dispatch → confirmation poll を orchestration し、report/summary に tuple・covering・unverifiable・dispatch を記録する。
- `daily-ci.yml` の job 構造が契約 test で固定され、既存 `decide` job の退行を検出する。
- consumer-verify.yml の証拠契約（artifact 名・JSON field）が契約 test で固定され、daily 側回復の前提が崩れたら検出する。

## Non-goals

- consumer-verify.yml の lane 判定・証拠生成ロジック自体の正しさ（別 test spec 管轄）。
- 実 GitHub API・実 dispatch・実 artifact download の E2E（fixture のみ。実走は merge 後の日次/手動 run で観測）。
- secret/deploy key・required check・ruleset の設定。
- consumer 独立 CI・証拠 DB などの新規状態基盤。
- 単一 SHA モードの判定ロジック変更（回帰しないことのみ保証）。

## Risk

- tuple 判定が「run conclusion success = verified」と誤実装されると、片 consumer 未検証や証拠欠落の tuple を verified と誤記し daily trigger が止まる — Issue #294 の中核契約（fail-closed）を破壊する。
- in-flight 抑制や unverifiable 計上を省略すると重複 dispatch・retry 暴走・偽 skip が起きる。
- dispatch payload の input 名・値が consumer-verify.yml の契約とずれると expected-SHA guard が fail-closed で落ちるか、誤 tuple を検証する。
- 証拠回復が artifact 命名・schema の暗黙依存を持つと、consumer-verify 側の正当な変更で daily が静かに unverifiable 化し続ける — 契約 test で検出可能にする。
- daily-ci.yml の job 追加が既存 `decide` job や trigger/権限を退行させると ci.yml 側の daily 検証が止まる — 契約 test で固定。
