---
id: "DESIGN-034UlaEev2MQ6gsnrNqias"
type: "design"
status: "approved"
title: "Issue #294 daily candidate-tuple trigger 設計 — tuple-aware decide・証拠 artifact 回復・daily-ci job"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-daily-tuple-trigger.md"
  changes:
    added:
      - type: "directory"
        path: "ci/daily"
      - type: "file"
        path: "docs/specs/issue-294-daily-tuple-trigger.md"
    modified:
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
      - type: "file"
        path: "ci/daily/decide.mjs"
      - type: "file"
        path: "ci/daily/tests/decide.test.mjs"
      - type: "file"
        path: "ci/daily/tests/daily-ci-yml-contract.test.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-294-daily-tuple-trigger.md"
  implemented-by:
    - "docs/plans/issue-294-daily-tuple-trigger.md"
  depends-on:
    - "docs/specs/issue-294-daily-tuple-trigger.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-367-daily-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-daily-tuple-trigger.md"
  derived-by:
    - "docs/plans/issue-294-daily-tuple-trigger.md"
    - "docs/test-specs/issue-294-daily-tuple-trigger.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-daily-tuple-trigger.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
  defers: []
  deferred-by: []
---

# Issue #294 daily candidate-tuple trigger 設計 — tuple-aware decide・証拠 artifact 回復・daily-ci job

## Context

`ci/daily/decide.mjs` + `daily-ci.yml`（Issue #367）は「`ci.yml` の main HEAD に対し run 履歴だけで Full CI 検証済みかを判定し、未検証なら `expected_sha` 付きで dispatch する」単一 SHA scheduler として確定済み。`consumer-verify.yml`（Issue #294 T1/T2/T3）は「候補 `(ruviform SHA, consumer SHA, contract)` の tuple を証拠 JSON artifact に記録する」dispatch 専用 lane として main に存在するが、**それを起動・抑制・retry 判定する側がない**。

本設計は #294 の残 AC「daily 候補 tuple 判定」を実装する。制約の確認済み事実:

- **list API は run の `inputs` を返さない**（`GET .../runs` / `GET .../runs/{id}` の双方で実測確認）。tuple の consumer SHA は run record からは復元不能。
- tuple の唯一の機械可読な記録は `consumer-evidence-{audiolens,synvelis}-attempt-<run_attempt>` artifact の JSON（`tuple.{ruviformRev,consumerResolvedSha,contract}` + `consumer.{name,status}`）。artifact は既定 90 日で期限切れ → 「証拠が取得不可/期限切れなら未検証として扱う」（Issue #294 本文契約）。
- consumer repo の main HEAD は匿名 HTTPS で解決可能（`git ls-remote https://github.com/scarecrow173/{audio-lens,synvelis}.git` 実測 200。consumer-verify の anonymous probe と同一境界）。
- 既存 run 履歴は僅少（2026-11-09 時点 11 run、main branch 6 run）で、証拠 artifact 非対応の旧 run（2026-11-04 以前）と cancelled run（artifact あり）が混在する。

## Scope

### In Scope

- `ci/daily/decide.mjs`: tuple モード（`--mode tuple` / `DAILY_MODE=tuple`）— 候補 tuple 解決・証拠 artifact 回復・`decideTuple` 判定・`executeTuple` orchestration・tuple dispatch・report/summary。単一 SHA 経路は非破壊。
- `ci/daily/decide.mjs` 内の ZIP 抽出 helper（artifact zip → JSON。stdlib `node:zlib` のみ）。
- `ci/daily/tests/decide.test.mjs`: tuple 判定行列・orchestration・zip 抽出の contract test。
- `ci/daily/tests/daily-ci-yml-contract.test.mjs`（新規）: daily-ci.yml の job 構造契約。
- `.github/workflows/daily-ci.yml`: `decide-consumer-verify` job 追加。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`: tuple 回復依存契約の固定 test 追加。
- `docs/development-environment.md`: daily-ci.yml job 構成の同期。

### Out of Scope

- `consumer-verify.yml` の変更（既存入力・証拠契約で充足 — 変更不要を設計前提とし、必要が生じた場合のみ最小限）。
- secret/deploy key・required check・ruleset（人間作業）。consumer repo 側変更。`crates/`。
- 失敗 Issue 自動起票（#368）。private repo credential 経路。

## Components and Boundaries

### 1. `ci/daily/decide.mjs` — tuple モード

単一 SHA 経路（`decide`/`execute`/`resolveConfig` 既定・`createGitHubClient`/`createFixtureClient` の既存メソッド）は**一切変更しない**。tuple モードは追加の export と config で実現する。

#### config 追加（`resolveConfig`）

| env / flag | 既定 | 用途 |
| --- | --- | --- |
| `--mode` / `DAILY_MODE` | `single` | `single`=既存 / `tuple`=consumer-verify tuple 判定 |
| `--consumer-repo` / `DAILY_CONSUMER_REPOS` | `audiolens=scarecrow173/audio-lens,synvelis=scarecrow173/synvelis` | tuple 対象 consumer（`name=owner/repo` カンマ区切り、順序保持） |
| `--tuple-contract` / `DAILY_TUPLE_CONTRACT` | `consumer-verify-tuple/v1` | tuple の契約版要素 |
| `--max-evidence-runs` / `DAILY_MAX_EVIDENCE_RUNS` | `60` | 証拠 artifact を走査する terminal run の上限 |

`main()` は `cfg.mode === "tuple"` で `executeTuple` へ分岐する。`--workflow` は tuple モードで `consumer-verify.yml` を指す（env `DAILY_TARGET_WORKFLOW` 既存機構を再利用）。

#### client 追加メソッド（`createGitHubClient` / `createFixtureClient` 双方に同型で追加）

| メソッド | 実装 | 用途 |
| --- | --- | --- |
| `getRepoRefSha(repo, ref)` | `git ls-remote https://github.com/{repo}.git refs/heads/{ref}`（`node:child_process.execFile`、匿名 HTTPS — consumer-verify anonymous probe と同一境界）。出力は `<40hex>\trefs/heads/<ref>` 1 行を厳格 parse。失敗・0 件・複数件・非 40hex は `ApiError` | consumer main HEAD 解決（private repo は失敗 → `error` outcome、推測しない） |
| `listRunArtifacts(runId)` | `GET /repos/{repo}/actions/runs/{runId}/artifacts?per_page=100&page=N` ページネーション。`artifacts` 配列必須（欠落は malformed → `ApiError`） | run の artifact 一覧 |
| `downloadArtifactZip(artifactId)` | `GET /repos/{repo}/actions/artifacts/{id}/zip`。`redirect: "manual"` で 302 の `Location` を取得し、**Authorization 無し**で signed URL を fetch（token を redirect 先へ転写しない）。`arrayBuffer` → `Buffer` | evidence zip 取得 |
| `getRunEvidence(runId, { attempt, consumers })` | 上記 3 メソッドを合成: `consumer-evidence-{name}-attempt-{attempt}` を名前一致・`expired !== true` で選択 → zip → `jsonFromArtifactZip` → JSON 妥当性検査（`kind === "consumer-verify-evidence"`・`tuple`/`consumer.name`/`status` 存在・`run.runId === String(runId)`）。欠落・期限切れ・破損・不整合は `null`（throw しない — unverifiable として扱う） | run → consumer 別 evidence JSON 回復 |

`createFixtureClient` は `repoRefReads`（repo 別 SHA 列、既存 `refReads` と同じ repeat-last 意味論）と `runArtifacts`（`{"<runId>": {"<consumer>": <evidence JSON object> | "__missing__" | "__expired__" | "__malformed__"}}`）で同じメソッド面を提供し、`downloadArtifactZip` は fixture 専用の stored-method ZIP builder（`buildStoredZip`、CRC32 テーブル実装）で実 zip bytes を合成して返す — **実 unzip 経路ごと fixture で再現する**。

#### `jsonFromArtifactZip(buf)`（export・pure）

stdlib `node:zlib` のみで ZIP を走査: EOCD（`0x06054b50`）を末尾から探索 → central directory → 最初の `.json` entry（無ければ先頭 entry）→ local header → method 0（stored）はそのまま、method 8（deflate）は `inflateRawSync` → UTF-8 decode → `JSON.parse`。非 zip・entry 不在・対応外 method・JSON 破損は `ApiError`。consumer-evidence artifact は単一 JSON ファイルなので「最初の .json entry」規則で十分（複数 entry の汎用 unzip は YAGNI）。

#### tuple 判定コア `decideTuple`

```js
decideTuple({
  candidate,        // {ruviformSha, consumers: [{name, repo, sha}], contract}
  covering,         // [{run, consumerStatus: {name: status}}] — tuple 一致が証拠で立証された run、newestFirst
  inFlight,         // in-scope run のうち classifyRun === "in-flight"（tuple 未回復でも抑制側）
  unverifiable,     // [{run, isAuto}] — terminal run で証拠回復不能（artifact 不在/期限切れ/破損/不整合）
  autoActors, maxAutoDispatches,
}) → { decision, reason, latestCovering, latestVerified, autoOnTuple, unverifiableAuto }
```

判定順（既存 `decide` の優先順位を tuple に一般化）:

1. `inFlight.length > 0` → `suppress-in-flight`（in-flight run は証拠未到達で tuple を証明できない — 別 tuple の可能性を排除できないため**全て抑制側**。consumer-verify の job timeout は 120 分なので長期滞留しない）
2. `latestCovering = covering[0]`（covering 内 newestFirst — attemptTime 順序と tie 規則は既存 `newestFirst`/`attemptTime` を再利用）
3. `latestCovering` が存在し `classifyRun(latest) === "unknown"` → `hold-unknown`
4. `latestCovering` が `isSuccess(run)` かつ**全 consumer `status === "pass"`**:
   - ambiguity 検査（既存 3' と同型）: covering 内の非 verified run で attemptTime tie/unorderable/`updated_at` が最新 attemptTime より新しい contender、**または** `attemptTime > latest.attemptTime` の unverifiable run が 1 件でも → `hold-unknown`（隠れた新規 attempt を見逃して偽 verified を記録しない）
   - contention なし → `skip-verified`
5. 上記以外（covering なし・latest が非 success・success だが consumer 非 pass）→ budget 判定:
   - `autoOnTuple = covering 内 isAutoDispatch 数 + unverifiable 内 isAutoDispatch 数`（両者の合計 — unverifiable auto は tuple 帰属を証明できないが、自動発火を否定できないため保守的に計上。手動 actor は計上しない）
   - `autoOnTuple >= maxAutoDispatches` → `budget-exhausted`、else → `dispatch`

`latestVerified` = verified 条件（success + 全 consumer pass）を満たす最新 covering run（report 用）。

**verified の定義（R3）**: run conclusion `success` だけでは verified にしない — 両 consumer の evidence `status === "pass"` を必須とする。`not_run`/`unsupported`/`fail` の consumer を含む success run は「部分検証」であり tuple を verified にしない → budget 判定へ（retry 候補。恒常的 not_run は budget 枯渇 → warning で可視化）。

**unverifiable の扱い（fail-closed だが永久 hold にしない）**: artifact 不在・期限切れ・破損の terminal run は (a) covering にも non-target にも分類しない「unverifiable」。skip-verified の材料にせず（証拠なしに verified を主張しない）、auto actor なら budget に保守計上する。最新 covering attempt より新しい unverifiable が skip を `hold-unknown` に倒すのは「隠れた attempt」のみ — dispatch 判定は妨げない（再検証は安全側）。これにより証拠期限切れは「未検証→再 dispatch」に自然に写像され、かつ unverifiable が恒久的に残っても新 tuple の判定を止めない。

#### `executeTuple(cfg)` orchestration

```
1. rfSha = client.getRefSha(mainRef)                     // ruviform main（既存経路）
   consumerShas = consumers.map(c => client.getRepoRefSha(c.repo, "main"))
   candidate = {ruviformSha: rfSha, consumers: [...], contract: cfg.tupleContract}
   // いずれかの解決失敗 → error outcome（exit 1。推測で tuple を組まない）
2. listed = client.listRuns({workflow, branch: mainRef, maxPages})
   listed.truncated → history-truncated（既存と同じ fail-closed）
3. scoped = listed.runs.filter(isFullCiRun(r, mainRef))  // head_branch===main && event∈FULL_CI_EVENTS && valid head_sha
   inFlight = scoped.filter(classifyRun==="in-flight")
   terminal = scoped.filter(非 in-flight).sort(newestFirst)
   terminal.length > cfg.maxEvidenceRuns → evidence-truncated（hold-unknown 系 outcome、warning。走査しきれない履歴で skip/dispatch を確定しない）
4. 各 terminal run について client.getRunEvidence(run.id, {attempt: run.run_attempt, consumers})
   → evidence pair から tuple を回復:
     - 両 consumer evidence が存在し JSON 妥当・`run.runId` 一致・`contract` 一致・`tuple.ruviformRev` が両者で一致し 40hex
     - consumer c の evidence について `consumer.name === c` かつ `tuple.consumerResolvedSha` が 40hex
     → 3 SHA + contract が candidate と全て一致 → covering {run, consumerStatus}
     → いずれか不一致 → non-target（証明済み・無視）
     → 回復不能 → unverifiable {run, isAuto: isAutoDispatch(run)}
5. d = decideTuple({...})
6. decision != dispatch → report + summary（exit 0。budget-exhausted/hold-unknown/evidence-truncated は ::warning::）
7. dispatch → candidate 再解決ループ（既存 maxRechecks と同型）:
   while decision==="dispatch":
     re-resolve rfSha + 全 consumer SHA → candidate' が変化していれば
     candidate=candidate'、listRuns 再取得、evidence 再走査、decideTuple 再実行
     （再取得 truncated → history-truncated。maxRechecks 超過 → candidate-moved）
8. dryRun → "dry-run" outcome
9. POST dispatches {ref: mainRef, inputs: {
     ruviform_rev: candidate.ruviformSha,
     [`${c.name}_ref`]: "main" for each consumer,
     [`${c.name}_expected_sha`]: c.sha for each consumer,
   }}
   report.dispatch = {requestedAt, candidate, ref: mainRef, confirmedRun: null}
10. confirm poll（既存と同型: preDispatchIds 差分・confirmTimeout/Interval）
    - 未観測 → dispatched-unconfirmed
    - confirmed.head_sha !== candidate.ruviformSha → dispatch-landed-mismatch
      （warning — main が dispatch 間に動いた。run の evidence.rf=ruviform_rev 入力は
       正しいため tuple 証拠自体は有効だが、candidate との対応を報告で明示）
    - else → dispatch-confirmed
```

consumer 名 → workflow input 名の対応は `DAILY_CONSUMER_REPOS` の `name` をそのまま使う（`audiolens`→`audiolens_ref`/`audiolens_expected_sha`、artifact 名 `consumer-evidence-audiolens-*`、evidence `consumer.name` と一致 — consumer-verify.yml の既存命名と整合）。

#### report / summary（tuple モード）

`kind: "daily-ci-tuple-decision"`、`mode: "tuple"`、`tuple: {ruviformSha, consumers:[{name,repo,sha}], contract}`、`observedShas`（repo 別 `{repo, ref, sha}` 列）、`coveringAttemptsOnDecisionTuple`（attempt 表 + consumer status）、`unverifiableCount`、`autoOnTuple`/`maxAutoDispatches`、既存フィールド（outcome/reason/latestVerified/dispatch/warnings/historyTruncated/generatedAt）を保持。`markdownSummary` は `report.tuple` 存在時に tuple 節（候補 4 要素・covering 表・unverifiable 数）を先頭に出す — 既存の attempts 表・dispatch 節は共用。

### 2. `.github/workflows/daily-ci.yml`

既存 `decide` job は不変。並列に `decide-consumer-verify` job を追加:

```yaml
  decide-consumer-verify:
    name: decide-consumer-verify
    runs-on: ubuntu-latest
    timeout-minutes: 20           # artifact 走査 + confirm poll を見込む
    permissions:
      contents: read
      actions: write              # consumer-verify.yml runs list + artifacts + dispatches
    steps:
      - checkout / mise setup / install node   # decide job と同一
      - self-test: mise run test-ci-daily      # tuple contract test を含む同一入口
      - evaluate:
          env: GITHUB_TOKEN, GITHUB_REPOSITORY, DRY_RUN,
               DAILY_TARGET_WORKFLOW: consumer-verify.yml,
               DAILY_MAIN_REF: main,
               DAILY_MODE: tuple,
               DAILY_CONSUMER_REPOS: "audiolens=scarecrow173/audio-lens,synvelis=scarecrow173/synvelis",
               DAILY_TUPLE_CONTRACT: consumer-verify-tuple/v1,
               DAILY_MAX_AUTO_DISPATCHES: "2"
          run: mise exec -- node ci/daily/decide.mjs --mode tuple --report daily-ci-tuple-report.json
      - upload artifact daily-ci-tuple-report-attempt-<n> (if: always(), path daily-ci-tuple-report.json)
```

- `on:`（schedule+workflow_dispatch）・`permissions: {}`・`concurrency`（workflow 全体で直列化済み）は不変。2 job は独立 workflow 履歴を読むため並列実行でよい。
- `actions: write` は list runs（read）・artifacts（read）・dispatches POST（write）を含む単一 scope — 追加権限なし。
- consumer SHA 解決は `git ls-remote`（runner 標準 git）で secret 不要。deploy key は daily job に持ち込まない（consumer-verify の credential 境界を越境しない）。

### 3. Tests

- `ci/daily/tests/decide.test.mjs` に tuple セクション追加: `decideTuple` 判定行列全枝、`executeTuple` fixture orchestration（tuple 解決→証拠回復→dispatch 入力→confirm）、`jsonFromArtifactZip`（test 内 zip builder で stored/deflate 両 method）、fixture client の `repoRefReads`/`runArtifacts`/`buildStoredZip`、単一 SHA 既存テストの非回帰。
- `ci/daily/tests/daily-ci-yml-contract.test.mjs`（新規・textual）: daily-ci.yml が `decide-consumer-verify` job を持ち、`DAILY_MODE: tuple`・`DAILY_TARGET_WORKFLOW: consumer-verify.yml`・`actions: write`・self-test・attempt 固有 artifact・`on:` が schedule+workflow_dispatch のみ・既存 `decide` job の `DAILY_TARGET_WORKFLOW: ci.yml` を保持、を固定。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` に追加: tuple 回復が依存する契約 — artifact 名が `consumer-evidence-<consumer>-attempt-<n>` 形式・evidence finalize が `tuple.{ruviformRev,consumerResolvedSha,contract}` と `consumer.{name,status}` を出力・upload が `if: always()`（既存 test の一部を補強・明示化）。

## Data and Control Flow

```
schedule(UTC) / workflow_dispatch(dry_run)
  ├─ decide job（既存・ci.yml 単一 SHA）… 不変
  └─ decide-consumer-verify job
       ├─ test-ci-daily（自己検証）
       ├─ resolve candidate tuple:
       │    rf  = getRefSha(main)                    [ruviform repo, API+token]
       │    al  = getRepoRefSha(audio-lens, main)    [git ls-remote anonymous]
       │    sv  = getRepoRefSha(synvelis, main)      [git ls-remote anonymous]
       ├─ listRuns(consumer-verify.yml, branch=main)
       ├─ scope (isFullCiRun) → in-flight? → suppress
       ├─ terminal runs (≤60) → getRunEvidence → tuple 回復
       │    covering / non-target / unverifiable に分類
       ├─ decideTuple → skip-verified / suppress-in-flight / hold-unknown /
       │              budget-exhausted / dispatch / evidence-truncated
       ├─ dispatch? → 3 SHA 再解決（candidate-moved 有界）→
       │    POST dispatches {ref:main, ruviform_rev, *_ref=main, *_expected_sha}
       │    → confirm poll → dispatch-confirmed / landed-mismatch / unconfirmed
       └─ report JSON + Job Summary（::warning:: for hold/budget 系）
```

**fail-closed 不変条件**:
- `skip-verified` は「covering run が API 上 completed+success・最新性が一意・両 consumer evidence status=pass」の三重条件のみ。証拠 artifact 不在・期限切れ・破損・内部不整合はいずれの verified 主張にも寄与しない。
- in-flight run（tuple 未回復を含む）は常に抑制側 — 別 tuple の in-flight が daily dispatch を 1 日遅らせるだけで、誤 dispatch は起きない。
- unverifiable auto dispatch は budget に保守計上（過少計上=retry 暴走を防ぐ。過大計上=dispatch 減少=安全側）。
- consumer SHA 解決失敗・API 失敗・malformed response・evidence 走査超過は `error`/hold で閉じ、推測 tuple で dispatch しない。

## Non-Functional Constraints

- stdlib-only Node（`node:zlib`/`node:child_process` は stdlib）。新規 npm 依存なし。YAML パーサ依存なし（contract test は textual）。
- API call 量: evidence 走査は terminal run ≤60 × (artifacts list 1 + zip 2) ≒ 180 call 上限/日 — `actions: write` の rate 内。confirm poll は既存と同じ 120s/15s 有界。
- 秘密の非露出: token は api.github.com 宛のみ。artifact zip の redirect 先へは Authorization を転写しない。`git ls-remote` は匿名 HTTPS で secret 不使用。report/summary に token 系を転写しない。
- Consumer 独立 CI・証拠 DB・repository variable を作らない — 状態正本は GitHub run 履歴 + evidence artifact のみ。

## Risks and Trade-offs

- **artifact 期限切れ → 定期再検証**: 証拠が 90 日で切れると verified tuple も「未検証」に戻り再 dispatch される。Issue #294 の契約（「証拠が取得不可/期限切れなら未検証として扱う」）どおりの意図的挙動であり、consumer-verify 自体は冪等な検証 lane のため再実行の害は runner 時間のみ。頻度は tuple が現行 HEAD の間のみ（HEAD が動けば別 tuple）。
- **unverifiable auto の過大計上**: 別 tuple の auto dispatch（artifact 期限切れ・欠落）が budget を消費し、現行 tuple の retry を早く枯渇させうる。安全側（dispatch 減少）であり `budget-exhausted` warning + report で可視化 — 人手 dispatch（auto actor でない）は budget 非計上のため復旧経路は残る。
- **in-flight 全面抑制の粒度**: 別 tuple の in-flight run も抑制する（tuple を証明できないため）。consumer-verify は手動 dispatch が主体で頻度が低く、job timeout 120 分で滞留しない — 最大 1 日の遅延で許容。
- **head_sha と tuple.rf の非一致**: dispatch `ref: main` の run は `head_sha` = dispatch 時点の main HEAD であり、`ruviform_rev` 入力（= 判定時 main SHA）と race でずれうる。tuple 判定は evidence の `ruviformRev` を正本とするため head_sha には依存しない。`dispatch-landed-mismatch` は「main が判定〜dispatch 間に動いた」シグナルとして報告のみ（evidence.rf は入力値で正しい）。
- **evidence 走査上限**: `maxEvidenceRuns=60` 超過で `evidence-truncated` hold。現行履歴は僅少で実害なし。consumer-verify の運用頻度が上がった場合は上限拡大か evidence 永続化の再検討が必要（Deferred）。
- **`git ls-remote` への依存**: runner 標準 git 前提。GitHub HTTPS 障害時は `error` outcome（翌日再判定）。private 化時は anonymous が失敗 → `error` で可視化（credential 経路は別途人間作業）。
- **zip parser の最小性**: stored/deflate・単一 .json entry のみ対応。GitHub artifact zip はこの形状で安定（`actions/upload-artifact` の出力形式）。将来の形式変更は `jsonFromArtifactZip` の contract test で検出 → `unverifiable` 側に倒れ fail-open しない（verified 主張には寄与しない）。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 |
| --- | --- | --- |
| consumer-verify 失敗の自動 Issue 起票 | #368 tracker の責務境界 | Issue #368 |
| private consumer repo の credential 経路 | secret/権限は人間作業。anonymous で現行充足 | Issue #294 本体 |
| evidence artifact の長期保管・検索基盤 | 現状は run history + artifact で十分。走査上限超過時に再検討 | Issue #294 本体 |
| consumer 独立 Daily 証拠との tuple 再利用判定 | 「完全に同一候補 revision・条件」の証拠再利用は別設計 | Issue #294 本体 |
| GUI-VISUAL/OS・PLUGIN-NATIVE-DOCK・macOS lane | upstream gate が owner（`unsupported` 行は不変） | Issue #294 本体 |

## Verification Notes

- [ ] `mise run test-ci-daily` — tuple 判定行列・executeTuple orchestration・zip 抽出・fixture 拡張・既存単一 SHA 非回帰
- [ ] `mise run test-ci-consumer-verify` — tuple 回復依存契約の固定 test を含む既存 suite
- [ ] `mise exec -- actionlint .github/workflows/daily-ci.yml`（全 workflow）
- [ ] `mise exec -- prek run --all-files`
- [ ] `mise run fmt-check` / `mise run check`
- [ ] 実 dispatch は行わない（fixture のみ。実走確認は merge 後の日次/手動 run — NotRun として報告）
