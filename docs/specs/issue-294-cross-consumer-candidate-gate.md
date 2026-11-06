---
id: "SPEC-034S9nR4tW7xZ2cF5hJ8kM"
type: "spec"
status: "proposed"
title: "Issue #294 Cross-Consumer候補tuple Gate — consumer-verify.yml 証拠契約・UI lane 段階拡張・候補連携接続点"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/discovery/issue-294-cross-consumer-candidate-gate.md"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-294-cross-consumer-candidate-gate.md"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #294 Cross-Consumer候補tuple Gate — consumer-verify.yml 証拠契約・UI lane 段階拡張・候補連携接続点

## What Should Be Built

既存 `consumer-verify.yml` を、Issue #294 の 2026-11-05 改訂（Daily Full CI＋候補tuple判定、正本は #365/#367/#368）と GUI Testing v1.0 追加 AC に対応させる。

1. **候補tuple証拠契約**: run が検証した tuple =（候補 Ruviform SHA, 解決 AudioLens SHA, 解決 Synvelis SHA, **検証契約版**）を、各 Consumer lane が実際に checkout した `rev-parse HEAD` から捕捉し、構造化 JSON artifact と Job Summary に記録する。契約版は証拠 JSON の `schemaVersion`（JSON 形状の版）とは別物の `contract` 識別子（本実装では `"consumer-verify-tuple/v1"`）として記録し、lane 集合・検証意味論の変更時に bump する。任意入力の期待 Consumer SHA と fail-closed 照合する。
2. **Consumer ref の SHA 受理**: branch/tag に加えて候補 PR head 等の raw SHA を checkout 可能にし、RF #305＋SV #84・AL #143 等の候補連携を同じ lane で検証できるようにする。GitHub は ref 非公開（unadvertised）commit の `git fetch` も受理するため、境界は ref 到達可能性ではなく**サーバ側オブジェクト存在**（GC 済み・未 push の SHA は fetch 失敗として正直に記録）。
3. **UI headless lane 段階接続**: AudioLens は vendor 再生成＋`setup-ui`（npm ci、check-ui/test-ui が install depends を持たないため必須）＋`check-ruviform-rev --require-vendor`＋`check-ui`/`test-ui`。Synvelis は `check-ruviform-rev`（fetch 直後の lockfile 単一 SHA 検証）＋`test-rt`＋`check-headless-deps`＋vendor＋`setup-ui`＋`check-ui`/`test-ui`/`build-ui`＋build 後の `check-ruviform-rev --require-vendor`（lock+REV+assets manifest の完全整合 — manifest は build-ui 産物のため build 後でなければ検証が vacuous）。各 Consumer 自身の mise 入口で lane 化し、実窓 host 依存 lane は `unsupported`/`not_run` 理由付きで記録、skip を pass にしない。
4. **汎用 task 接続点**: Consumer 別の dispatch 入力（`audiolens_extra_tasks`/`synvelis_extra_tasks`）で、その Consumer が定義する追加 mise task（RF #306 の fixture/test command 等）を lane として実行・記録する。pattern `^[A-Za-z0-9][A-Za-z0-9_-]*$`・最大 8 件を preflight で検証し、job レベル `timeout-minutes` で暴走を抑止する。dispatch 入力は operator 信頼入力であり、pattern は shell 注入の排除であって task の意味論を制約しないことを明示する。
5. **検証器自身の検証**: tuple 照合・lane 結果集約・status 導出のロジックを `ci/consumer-verify/` の stdlib-only Node スクリプトに集約し、`node --test` の fixture test と `mise run test-ci-consumer-verify` を用意する。workflow はこの script を自己検証してから使う。script の checkout ref は **workflow 自身の dispatch ref（`github.sha`）**とする — 候補 SHA が script 導入前の revision でも lane が成立する（候補 tree に script 不在でも検証できる）。

## Why Now

- #367 scheduler が dispatch 側の判定・重複抑止・bounded retry を担う設計で確定済み。scheduler が「期待 tuple を入力として渡し、run 証拠から tuple を回復する」には consumer-verify 側の入力・証拠契約が先に必要。
- 現行 lane は Ubuntu/Rust/CLI のみで、GUI Testing v1.0 が要求する UI vendor/lockfile 整合・両 Consumer UI headless 回帰の証拠が取れない。両 Consumer に既に headless UI task が揃っており、未接続なのは workflow 側のみ。
- RF #305（旧 Media API 廃止）＋SV #84・AL #143 の候補連携が「Consumer 候補 revision を SHA で指定できる」ことを前提にするが、現行入力は branch/tag のみ。

## Users and Value

- **#367 scheduler / 運用者**: dispatch に期待 tuple を載せ、run artifact から解決 tuple を機械回復できる。証拠取得不可・期限切れは未検証として扱える。
- **Ruviform 変更者**: 候補 SHA が両 Consumer の Rust・UI headless・vendor 整合を壊さないか、Consumer 別に pass/fail/not_run/unsupported で判る。
- **Consumer 側 Owner（AL #127 / SV #28 / RF #306 系）**: 追従 PR 候補 SHA や Consumer 固有 fixture コマンドを、workflow 編集なしの dispatch 入力で候補検証に接続できる。

## Scope

### In Scope

- `.github/workflows/consumer-verify.yml`: 入力拡張（期待 SHA・SHA ref・追加 task）、checkout 経路の ref/SHA 統一、解決 SHA 捕捉と tuple 照合、構造化 JSON evidence artifact、UI headless lane 追加、`unsupported` 語彙、同一候補の直列化 concurrency、preflight での script 自己検証。
- `ci/consumer-verify/`: evidence 集約・tuple 照合・status 導出スクリプトと fixture test（stdlib-only Node、`ci/summary`・`ci/daily` と同一構成）。
- `mise.toml`: `test-ci-consumer-verify` task。`docs/development-environment.md` の task 対応表同期。
- 対象 chain docs（discovery/spec/design/plan/test-spec/task/impl-record）。

### Out of Scope

- Daily tuple 判定・dispatch・重複抑止・retry（#367 scheduler の責務。本 spec はその入力・証拠契約のみ）。
- `ci-issue-tracker.yml` の監視対象拡大・失敗 Issue 自動起票（#368 tracker の責務境界）。
- `ci.yml`・各 Consumer 独立 CI・Consumer repo 内ファイルの変更。
- 実 GUI desktop（GPUI Driver-neutral）/Native Plugin Dock/実 OS lane（GUI-CONTRACT #390・GUI-VISUAL/OS #313/#314・PLUGIN-NATIVE-DOCK #388/#389・実機 Gate #243/#262）— 理由付き `unsupported` 記録に留める。GUI v2 方針では PR 必須 GUI gate・新規独立 GUI workflow・PR/push 起動 GUI workflow を追加しない。既存 Daily Full・manual・real-OS 経路が authoritative。
- macOS runner lane — 正式受入 Gate の対象 revision 実測要件は残るが、本 run では「macOS 未実測を NotRun/Unsupported として正直に記録する」契約のみ（実 lane 追加は後続）。
- AudioLens Export portability 失敗等 Consumer 側既知 bug の修正（Consumer repo 側 Issue の責務）。

## Requirements

- **tuple 証拠**: 各 Consumer job が解決 SHA（checkout 実測）・要求 ref・期待 SHA・照合結果を Job Summary と JSON artifact に記録する。GitHub Run 履歴だけで Consumer SHA が判ると仮定しない。
- **tuple 照合の fail-closed**: 期待 SHA が提供され解決 SHA と不一致の場合、その Consumer は `fail` + reason=`tuple_mismatch` で lane を実行しない（別 tuple の結果を期待 tuple の証拠にしない）。期待 SHA 未提供時は解決 SHA を記録のみ。
- **SHA ref 受理**: `audiolens_ref`/`synvelis_ref` に 40-hex SHA を許可し、branch/tag と同一の検証 lane を通す。checkout は fetch＋`FETCH_HEAD` で統一し、解決 SHA を必ず捕捉する。
- **lane 結果語彙と集約行列**: lane 結果は `pass`/`fail`/`not_run`/`unsupported`/`skipped`、Consumer status は `pass`/`fail`/`not_run`/`unsupported`。status=`pass` は「実行された lane が全て pass（fail 0 件・実行 lane ≥1）」のときのみ。`unsupported` は workflow が静的に宣言する capability-absent 行（実窓 host・macOS/実 OS）でのみ生成し、lane/task の実行失敗を `unsupported` へ再分類しない（実行して落ちたものは `fail`）。`skipped` は同一 job 内の上流 lane 失敗・gate 不達で実行されなかった派生記録に限る。Missing/NotRun/Unsupported/Fail を Pass 扱いしない。理由と追跡先（Issue/owner）を記録する。
- **UI lane**: Consumer 自身の mise task を使い、vendor REV/lockfile 整合を Consumer 側 authoritative 検証（`check-ruviform-rev` 系）で担保する。headless 完結 lane は既定実行、実窓 host 依存 lane は `unsupported` 記録。UI lane の rust/npm ツール要件（synvelis の `node` 導入を含む）を lane 前提として明示する。
- **追加 task 接続点**: 追加 task 名は Consumer 別入力・pattern 検証（preflight、lane 実行前）・上限付きで lane 化し、個別結果を記録する。失敗は lane 失敗として扱い、skip しない。
- **証拠 artifact**: `schemaVersion`・`contract` を持つ JSON を Consumer 別に upload-artifact へ出す。Job Summary は API で機械再取得できないため、**全終端経路**（gate=not_run・tuple_mismatch・workspace 不在・lane 途中失敗を含む）で artifact を `if: always()` 出力し、名前は attempt 固有（`-attempt-${{ github.run_attempt }}`）とする。Job Summary の Markdown 表は従来どおり人間可読の正本として併記する。
- **権限・秘密**: `contents: read` 既定、秘密は deploy_key 経路のみ・runner temp のみ・cleanup 必須。匿名 HTTPS 経路は秘密なしで維持。入力 Media/Prompt/Grant に相当する任意文字列（追加 task 名）をログに出しても秘密は含まれないが、pattern 検証で注入を防ぐ。**git/ssh を使う lane（clone・cargo fetch・`sync-ui-deps` の内部 git fetch 等）は全て credential cleanup step より前に置く** — deploy_key 復帰時に credential 破棄後の git 操作が fail する順序不具合を防ぐ。
- **直列化**: 同一候補 SHA の同時重複 run を `concurrency` で直列化する（cancel-in-progress=false）。scheduler 側の抑制と二重化しない範囲で runner 浪費を防ぐ。

## Acceptance Criteria

本 workflow は `workflow_dispatch` 専用で、dispatch は default branch の定義を使う — **PR 時点で実 runner 検証は不可能**。以下「構造証拠」は PR 時点で検証可能な契約（fixture test・contract test・actionlint）、「実走証拠」は merge 後初回 dispatch で取得するものと明記する。

- [ ] (構造) `consumer-verify.yml` が `_expected_sha`/`_extra_tasks` 入力・解決 SHA 捕捉 step・`if: always()` の artifact 出力・`unsupported` 固定行を持ち、両 verify job で対称であることを yml contract test が保証。
- [ ] (構造) status 導出の全分岐（pass/fail/tuple_mismatch/not_run/unsupported/coverage）と証拠 JSON schema・`shaMatch` 三値が `mise run test-ci-consumer-verify` で pass。
- [ ] (構造) `actionlint` と `prek run --all-files`（check-yaml）が pass。
- [ ] (実走) dispatch で指定した期待 tuple と解決 SHA が JSON artifact / Job Summary に記録され、`gh api`（runs + artifacts）で再取得できる。
- [ ] (実走) 期待 SHA 不一致の Consumer が `fail`+`tuple_mismatch` となり、期待 tuple の Pass に偽装されない。
- [ ] (実走) `_ref` に 40-hex SHA（repo に存在する commit）を渡した dispatch が clone 成功・解決 SHA 捕捉・lane 実行できる。
- [ ] (実走) 両 Consumer の status が `pass`/`fail`/`not_run`/`unsupported` で独立記録され、全 lane not_run/unsupported の run は workflow 失敗のまま。
- [ ] (実走) AudioLens（`setup-ui`→`check-ui`/`test-ui`）と Synvelis（`check-ui`/`test-ui`/`build-ui`＋`test-rt`/`check-headless-deps`）の headless UI lane が lane 表に独立結果で出る。
- [ ] (実走) 追加 task 入力が lane として実行・記録され、未提供時は lane 表に出ない。
- [ ] (構造+実走) 実 GUI/実 OS/macOS lane を pass と偽装せず、理由・追跡先（RF #313/#314・正式受入）を lane 理由に記録する。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 |
| --- | --- | --- |
| tuple 判定・scheduler 側の consumer-verify dispatch | #367 の責務境界（`DAILY_TARGET_WORKFLOW` 切替と tuple 対応判定は別設計） | Issue #367/#365 |
| consumer-verify 失敗の自動 Issue 起票 | #368 tracker の監視対象拡大は別判断 | Issue #368 |
| macOS/実 OS lane・実 GUI Gate | RF #313/#314・正式受入フェーズ | Issue #294 本体 |
| AudioLens lockfile 主体移行「後」形態の追加差異 | 現行 sed 注入は rev 有無を同一ルールで処理し両形態で有効（監査済み）。Consumer 側が manifest から git 依存自体を外す設計変更が入った場合のみ再検討 | Issue #294 本体 |

## Sources and Relations

- Issue #294（2026-11-05 改訂・GUI Testing v1.0・横断依存正規化 v1.1・Synvelis 4領域節を含む本文を正本として確認済み。コメントなし）
- 監査: `docs/discovery/issue-294-cross-consumer-candidate-gate.md`
- 親 spec: `docs/specs/issue-219-consumer-compat-contract.md`
- 関連 design: `docs/designs/issue-367-daily-scheduler.md`（scheduler 境界）、`docs/designs/issue-327-cross-repo-ci-baseline.md`（証拠 DB を作らない原則）
- 既存 test-spec: `docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md`（本 spec で拡張する gate 意味論の先行保証）
