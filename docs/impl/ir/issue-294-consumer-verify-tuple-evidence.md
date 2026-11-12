---
id: "IMPL-034Sd294TupleEvidenceX9f"
type: "impl"
status: "completed"
title: "Issue #294 consumer-verify 候補 tuple 証拠契約（解決 SHA 捕捉・期待照合・SHA ref・UI lane・extra tasks・unsupported 状態）"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "file"
        path: "ci/consumer-verify/evidence.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/evidence.test.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
    modified:
      - type: "workflow"
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
    - "../../tasks/issue-294-t1-consumer-verify-evidence-script.md"
    - "../../tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  implemented-by: []
  depends-on:
    - "../../designs/issue-294-cross-consumer-candidate-gate.md"
    - "../../plans/issue-294-cross-consumer-candidate-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-294-cross-consumer-candidate-gate.md"
    - "../../discovery/issue-294-cross-consumer-candidate-gate.md"
    - "../../test-specs/issue-294-consumer-verify-tuple-evidence.md"
    - "../../impl/ir/issue-219-consumer-verify-gate.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/issue-294-cross-consumer-candidate-gate.md"
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

# Issue #294 consumer-verify 候補 tuple 証拠契約

## Summary

`consumer-verify.yml` を Issue #294（2026-11-05 改訂: 候補 tuple 証拠契約＋GUI headless lane）に対応させた。統合前 review の CONFIRMED 指摘 A-1（GUI-CONTRACT/fake oracle と実 window 必須 lane の `unsupported` 混同）は固定行を `not_run`（未接続・Owner/再実行条件付き）と `unsupported`（capability 不足）へ分離して修復、B-1（concurrency group の tuple 衝突）は expected SHA + extra tasks を key に追加して修復。各 Consumer lane が `git rev-parse HEAD` の解決 SHA を捕捉し任意入力の期待 SHA と fail-closed 照合、ref 入力は branch/tag に加え 40-hex SHA を統一 fetch 経路で受理、AudioLens/Synvelis の headless UI lane を段階接続、Consumer 別 `*_extra_tasks` 接続点、lane 結果語彙に `unsupported`/`skipped` を加え全終端経路で構造化 JSON artifact + Job Summary を出力する。status 導出・証拠生成は `ci/consumer-verify/evidence.mjs` に集約し、`mise run test-ci-consumer-verify`（61 tests）で fixture 化 — preflight が run ごとに証拠器を自己検証してから lane 使用。

## Context

- 実施タスク: `docs/tasks/issue-294-t1-consumer-verify-evidence-script.md`・`issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md`（`docs/plans/issue-294-cross-consumer-candidate-gate.md` 配下）
- 上流: `docs/designs/issue-294-cross-consumer-candidate-gate.md`・`docs/specs/issue-294-cross-consumer-candidate-gate.md`・`docs/discovery/issue-294-cross-consumer-candidate-gate.md`
- 既存実装: `docs/impl/ir/issue-219-consumer-verify-gate.md` の #219 workflow を拡張（機構差し替えではなく lane/証拠層の拡張）
- review-protocol: spec/design 段で Reviewer A/B の指摘を全て取込済み（契約版と `schemaVersion` の分離、AudioLens `setup-ui` 前提、Synvelis manifest 検証順序、status 語彙行列、script checkout ref=dispatch ref、全終端経路 artifact、credential 順序、node toolchain、SHA 受理の境界=サーバ側オブジェクト存在）。実装段の独立レビュー（Reviewer B adversarial）: dep/pin step の id 化と lane 行対応（未記録 fail が skipped 行だけになり unsupported 化する洗濯を排除）、extra-task 集約 backstop 行、lanes ファイル truncate（persistent runner の stale 行対策）、post-lane HEAD drift 検出、result step の tuple env 固定（GITHUB_ENV 偽装対策）、finalize 失敗時の最小証拠 JSON、inputs の run: 直接混入排除（env 経由化）、readLanes の行 schema 検証、非 run gate での EXPECTED_SHA/RESOLVED_SHA 受容 — すべて取込済み。deploy key と Consumer 制御コードの共存・job cancel 中の証拠欠落は design の受容リスクとして記録。

## Implementation

### Changes

- `ci/consumer-verify/evidence.mjs` — 新設。stdlib-only の 3 コマンド: `classify-ref`（40-hex→`sha`・安全形→`ref`・`..`/先頭末尾`/ `/`.lock`→`invalid`）、`lane`（JSONL 追記、outcome 語彙 `pass`/`fail`/`not_run`/`unsupported`/`skipped`）、`finalize`（status 導出→証拠 JSON+Job Summary+`status=` output）。`shaMatch` は `EXPECTED_SHA`/`RESOLVED_SHA` から finalize 側で導出（期待未提供=`null`、提供時に解決 SHA 未捕捉=`false`）。`GATE=run` 時のみ `RUVIFORM_REV` の 40-hex を要求し、blocked/not_run 終端経路では生値を「試行された入力」として記録する。env は allowlist のみ読み秘密系を転写しない。
- `ci/consumer-verify/tests/evidence.test.mjs` — 26 tests。classify-ref 行列、lane 記録検証、status 導出全枝（blocked/not_run/tuple_mismatch/lane fail/pass+unsupported coverage/unsupported のみ/空）、`shaMatch` 三値、schema・contract・summary 形状、secret 非転写、CLI append/finalize/classify-ref。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` — 34 tests。trigger=workflow_dispatch のみ、tuple+extra-task inputs、concurrency（cancel-in-progress=false）、preflight 検証+自己検証順序、両 job の rev-parse 捕捉・統一 FETCH_HEAD checkout・`if: always()` 証拠/artifact・attempt 固有名・unsupported 固定行・timeout、期待 SHA 失敗系、UI lane 配線（SV の `--require-vendor` が build-ui 後である順序含む）、extra task ループ・summary 集約、evidence.mjs checkout が dispatch ref（候補 SHA 参照なし）。
- `.github/workflows/consumer-verify.yml` — inputs に `audiolens_expected_sha`/`synvelis_expected_sha`/`audiolens_extra_tasks`/`synvelis_extra_tasks` を追加し `_ref` を branch/tag/SHA 受理に拡張。`concurrency`（候補組合せ直列化・cancel-in-progress=false）。preflight: dispatch ref checkout→node→入力検証（classify-ref 含む）→`test-ci-consumer-verify` 自己検証→probe。verify 両 job: `Lane path env`（runner.temp 派生 path を GITHUB_ENV 供給 — job env では runner context 不可）→dispatch ref checkout→gate→credential/mise→**統一 clone**（`git init`+`remote add`+`fetch --depth 1 origin <ref-or-sha>`+`checkout --detach FETCH_HEAD`）→`resolve-sha`（rev-parse 捕捉+tuple guard、`always() && gate==run` で clone 失敗も fail lane 記録）→dep probe→rev 注入→fetch+lockfile guard→lane 群→`Record evidence`（`if: always()`・step outcome 写像で lane 記録+static unsupported 行+finalize）→`Upload evidence`（`if: always()`・attempt 固有名・`if-no-files-found: error`）→cleanup。summary job: 候補 tuple 表（両解決 SHA+expected）+status 集約+not-run-is-not-pass 強制（`unsupported` 語彙追加）。
- `mise.toml` — `test-ci-consumer-verify` 追加（`node --test ci/consumer-verify/`）。
- `docs/development-environment.md` — task 対応表・workflow 節の consumer-verify 記述を新構造に更新。

### Lane 集合（確定）

- **verify-audiolens**: fetch+lockfile guard→`check-ruviform-lock`→`check-ci`→`al-cli` ci build+`check-cli-headless`→`test-ci`（LFS pull）→UI 独立 chain `sync-ui-deps`→`check-ruviform-rev --require-vendor`→`setup-ui`→`check-ui`→`test-ui`。固定行（未接続と capability 不足を分離）: GUI-CONTRACT checks（GPUI 不要・Owner RF #390）と GUI semantic/fake-oracle（Owner GUI-SEMANTIC + AL #166/#127）= `not_run`、GUI visual/OS + 実 window（RF #313/#314 + AL #166/#127）・macOS/実 OS lanes = `unsupported`。
- **verify-synvelis**: presence check→fetch+lockfile guard→`check-ruviform-rev`→`check`→`test`（LFS pull）→`test-rt`→`check-headless-deps`→`sync-ui-deps`→`setup-ui`→`check-ui`→`test-ui`→`build-ui`→`check-ruviform-rev --require-vendor`（manifest は build-ui 産物のため必ず後置）。固定行（同分離）: GUI-CONTRACT（Owner RF #390）と GUI semantic/fake-oracle（Owner GUI-SEMANTIC + SV #175-177）= `not_run`、GUI visual/OS + 実 window・PLUGIN-NATIVE-DOCK（RF #388/#389 + SV #176、Floating/Generic fallback 成功を Pass にしない）・macOS/実 OS lanes = `unsupported`。
- **extra tasks**: 各 job `fetch` 成功後・`EXTRA_TASKS` 非空時のみ、pattern 済み task を順に `mise run`、各 task を lane 行として記録（途中失敗でも後続を試行、任一失敗で step fail）。

### Decisions During Implementation

- **evidence.mjs の checkout ref**: `actions/checkout`（ref 未指定=dispatch 対象 ref）を preflight・両 verify job の先頭に配置し候補 SHA（`RUVIFORM_REV`）からは参照しない — script 導入前の候補 revision を検証する run でも証拠器が存在する（Validator C4 の設計不具合を修正）。
- **`runner` context が job env で不可**: actionlint が `jobs.<job>.env` での `${{ runner.temp }}` 使用を拒否 → 無条件先頭 step `Lane path env` が `$GITHUB_ENV` に `CONSUMER_DIR`/`LANES_FILE`/`EVIDENCE_FILE` を供給（blocked/not_run 経路の record/upload step でも使える）。
- **resolution step を `always() && gate==run`**: clone step 自体の失敗時に GitHub が後続を skip するため `always()` でなければ rev-parse 失敗を fail lane として記録できず証拠が欠落する。
- **証拠器は runner 同梱 `node` で実行**: lane/finalize 呼出しは `mise exec -- node` ではなく plain `node`（runner 同梱 Node で十分・gate=blocked/not_run 経路では mise install が走らず壊れうるため）。self-test の pin toolchain 検証は preflight の `mise run test-ci-consumer-verify` が担う。
- **blocked/not_run で `RUVIFORM_REV` 生値を許容**: gate=blocked の原因が `ruviform_rev` 自身の不正形である経路で finalize が throw すると証拠が消失する — `GATE=run` 時のみ 40-hex を強制。
- **`extra` step の `if:`**: `env.EXTRA_TASKS != ''` で未提供時は lane 表に出さない（skip=pass にしない前提として存在自体を消す）。

### Known Limitations（正直記録）

- **実 runner 検証は review 基準 head `30205f2` で実測済み（修復後 head は別候補・正式 Pass へ転用しない）**: coordinator dispatch [run 42302538464](https://github.com/scarecrow173/ruviform/actions/runs/42302538464)（`gh workflow run --ref feature/consumer-verify-gui-candidate-gates`、RF `30205f2` + AL `04f1861` + SV `d8bf8ca` 期待 SHA 照合付き）。preflight success、両 Consumer clone・expected-SHA 照合 success（AL resolved=expected=`04f1861`、SV resolved=expected=`d8bf8ca`、`shaMatch: true`）。verify-synvelis 全 lane **pass**（check/test/test-rt/headless-deps/sync-ui/setup/check/test/build-ui/--require-vendor manifest 検証）。verify-audiolens は dep/pin/fetch/`check-ruviform-lock` まで pass し `check-ci` で **fail** — `al-observability/src/lib.rs:264` の `SinkConfig.span_events` 初期化子欠落（E0063）。該当 field は RF #335（PR #348・`33d93dd`）で RF main `49de02cc` に既導入済みで本 PR が追加したものではない — AL main `04f1861` 側の未追随が実不整合として検出された（AL127 互換修復は別 task 登録・workflow で補償しない）。fail 以降の AL lane は `skipped` 派生記録、summary の not-run-is-not-pass 強制も発火。artifact: `F:/repositry/audio-products/.devin/evidence/gui-v2-replan-20261107/pr392-run-artifacts/`（旧 artifact の GUI 行名は修復前の混同名 — 修復後 head は分離行を出力）。`workflow_dispatch` は `--ref` でブランチ指定可能という前記述（merge 前実走不可）は実測で訂正済み。
- **SHA fetch の境界**: GitHub は unadvertised commit の fetch を受理するが、GC 済み・未 push の SHA は clone step fail → `fail` 証拠として正直に出る。
- **extra_tasks は operator 信頼入力**: pattern は shell 注入排除のみで意味論は制約しない。job `timeout-minutes: 120` が暴走境界。
- **unsupported 行は ubuntu headless の宣言**: 実窓 host（RF #313/#314）・macOS/実 OS 正式受入の実測は本 run の範囲外。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| 証拠器 fixture + yml 契約 | `mise exec -- node --test ci/consumer-verify/` | **61 pass / 0 fail / 0 skip**（`mise run test-ci-consumer-verify` 相当） |
| workflow 構文 | `mise exec -- actionlint`（全 workflow） | exit 0・エラー 0（`runner` context 誤用を検出→`$GITHUB_ENV` 方式に修正） |
| hygiene | `mise exec -- prek run --all-files` | 全 hook pass（check-yaml/whitespace/eof/bom/private-key） |
| checkout 機構 smoke | temp repo: `git init`+`fetch --depth 1 origin <sha|master>`+`checkout --detach FETCH_HEAD`+`rev-parse HEAD` | branch・40-hex SHA 双方で解決成功（SHA fetch は `uploadpack.allowAnySHA1InWant` 有効 remote で再現） |
| 実 runner dispatch（基準 head `30205f2`） | [run 42302538464](https://github.com/scarecrow173/ruviform/actions/runs/42302538464) `gh workflow run --ref <branch>` + artifact `pr392-run-artifacts/` | **partial pass**: preflight・clone・tuple 照合（両 Consumer shaMatch=true）・verify-synvelis 全 lane pass・verify-audiolens `check-ci` fail（`SinkConfig.span_events` E0063 — RF #335/#348 で RF main 既導入・AL 未追随の実不整合、AL127 別 task 登録）・fail 以降は skipped 派生・summary 強制発火。修復後 head は未実走 |
| Consumer scratch lane 実走 | run 42302538464（基準 head） | **SV pass（全 14 lane）/ AL fail**（`check-ci` E0063 `SinkConfig.span_events` — RF #335/#348 で RF main 既導入・AL `04f1861` 未追随。修復は AL repo 責務・workflow 補償なし。修復後 head は未実走） |

## Risks

- **実 runner 検証は基準 head `30205f2` のみ** — review 修復後 head は未実走であり、修復内容の実 runner での正式 Pass は未取得（Known Limitations 記録どおり、修復後 head は別候補・正式 Pass へ転用しない）
- **verify-audiolens の `check-ci` fail は AL repo 側の未追随が実不整合** — `SinkConfig.span_events` 初期化子欠落（E0063）は RF #335/#348 由来で AL `04f1861` が未追随。修復は AL repo 責務であり workflow 側で補償しない
- **SHA fetch の境界** — unadvertised commit の fetch は GitHub が受理するが、GC 済み・未 push の SHA は clone step fail → `fail` 証拠として正直に出る
- **`extra_tasks` は operator 信頼入力** — pattern は shell 注入排除のみで意味論は制約しない。job `timeout-minutes: 120` が暴走境界
- **unsupported 行は ubuntu headless の宣言** — 実窓 host（RF #313/#314）・macOS/実 OS 正式受入の実測は本 run の範囲外で、宣言と実測の乖離は残る

## Follow-ups

- AL repo 側の `SinkConfig.span_events` 追随修復（AL127 互換修復・別 task 登録済み）と、修復後 head での実 runner 再 dispatch
- `unsupported` 行の実測は RF #313/#314（実窓 host・macOS/実 OS 正式受入）の完了待ち
- #219（長期 cross-consumer owner）は Open 維持、#305/#306 は候補連携・extra task 接続点の利用側、#365/#367/#368 は scheduler/tracker 責務の正本（`## Links` 参照）

## Links

- Issue: <https://github.com/scarecrow173/ruviform/issues/294>
- 関連: #219（長期 cross-consumer owner・Open 維持）、#305/#306（候補連携・extra task 接続点の利用側）、#313/#314（unsupported 行の追跡先）、#365/#367/#368（scheduler/tracker 責務の正本）
