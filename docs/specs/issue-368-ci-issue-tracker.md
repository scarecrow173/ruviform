---
id: "SPEC-4KMssIOpNuvh9jGgKKfp7R"
type: "spec"
status: "approved"
title: "Issue #368 CI failure Issue Tracker — workflow_run 完了監視で main×ci の未解決障害を 1 Issue に集約"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/366"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "directory"
        path: "ci/tracker"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-368-ci-issue-tracker.md"
    - "docs/plans/issue-368-ci-issue-tracker.md"
  depends-on:
    - "docs/specs/issue-366-full-ci-stage1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-366-full-ci-stage1.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-327-cross-repo-ci-baseline.md"
  derived-by:
    - "docs/designs/issue-368-ci-issue-tracker.md"
    - "docs/plans/issue-368-ci-issue-tracker.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-368-ci-issue-tracker.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
---

# Issue #368 CI failure Issue Tracker — `workflow_run` 完了監視で main×ci の未解決障害を 1 Issue に集約

## What Should Be Built

`.github/workflows/ci-issue-tracker.yml` を新設し、`workflow_run: {workflows: [ci], types: [completed]}` で `ci.yml`（`name: ci`）の完了を監視する。**この repo の `main` に対する有効な Full CI 実行**（同 repo・`head_branch == "main"`・許可 event・fork/PR 補助/scheduler 自身の run を除外）だけを対象に、失敗系結論では `main × ci` の未解決障害 Issue を 1 件に集約して作成・更新し、対象 revision をカバーする新しい完全成功でのみ回復記録付きで自動 close する。判定ロジックは `ci/tracker/tracker.mjs`（stdlib-only・GitHub API client 注入型）に置き、`node --test` の fixture/mock contract test で検証する。DB・外部 Bot・追加フレームワークは作らない。

## Why Now

- Issue #365（横断移行計画・正本）ルール 5: 「Full CI 終了は独立 `ci-issue-tracker.yml` で監視し、repo×workflow×main の未解決障害 Issue を原則 1 件に集約。失敗 Run URL・SHA・job を追記、後続最新 SHA の完全成功時のみ自動 close」。#366 Stage 1 で `ci-summary` fail-closed 集約が確定し、監視対象の run 契約（conclusion・revision 証跡）が揃った。
- Issue #367（Daily scheduler）が `workflow_dispatch` で `ci.yml` を起動するため、dispatch 起動の main Full CI 失敗も PR/push 経由と同一契約で追跡できる基盤が先に必要。#367 と並行実装。
- Full CI がマージ前ブロックでなくなっても（#365 ルール 1・4）、main の回帰を放置しないための最後の受け皿。

## Users and Value

- Ruviform maintainers: main の Full CI 失敗が自動的に 1 Issue へ集約され、失敗 SHA・Run URL・job 結果・検出期間・対応手順を追える。回復は「対象 revision をカバーする新しい成功」のみが close するため誤回復表示がない。
- Daily scheduler（#367）・手動 dispatch: `workflow_dispatch` 起動 run も同じ追跡契約に乗る。
- AudioLens / Synvelis（#159/#167 導入）: repo・workflow・branch・label・scope marker を設定値として差し替えられる汎用契約を Ruviform が所有し、両 Consumer が各自の Issue に自然展開できる。

## Scope

### In Scope

- `ci-issue-tracker.yml` 新設: `workflow_run`（workflows: `[ci]`・types: `[completed]`・branches: `[main]`）・tracker job のみ `actions: read`/`issues: write`/`contents: read`・`concurrency` 直列化（`cancel-in-progress: false`）。`pull_request`/`push`/`schedule`/`workflow_dispatch` trigger は持たない。
- `ci/tracker/tracker.mjs`: 有効 run 判定（repo・branch・event allowlist・fork 除外）→ conclusion 分類（success/failure/operational）→ Issue 一覧（label+scope marker の強い同定・`search` API ではなく `issues?labels=` list endpoint）→ create/update/close の plan → `gh api` 経由の apply。完了順逆転は run id 順で「最新の有効 run」のみ現在状態に反映。
- close 条件: `成功 run.id > 記録失敗 run.id` かつ `compare(失敗sha → 成功sha) ∈ {identical, ahead}` の両方を満たす時のみ。`behind`（古い成功）・`diverged`（履歴書換え等）は close しない。
- 失敗時 Issue 内容: 失敗 SHA・Run URL・job 結果表・初回検出/最終検出・対応手順。連続失敗は同一 Issue 更新、Issue 無しは新規作成、複数 open tracker Issue は最小番号を canonical として残りを duplicate close。
- `cancelled`/`skipped`/`neutral`/`stale`/`action_required`/未知・null conclusion: 成功でも失敗起票でもなく operational として記録（open Issue があれば operational note 更新、無ければ Job Summary のみ）。
- 出力 sanitize: SHA 形式検証・URL は `https://github.com/` prefix のみ・job 名は制御文字/`| `/改行除去+長さ上限・conclusion allowlist。
- mise task `test-ci-tracker`・`ci/tracker/README.md`（権限不足・label 未作成・GitHub Actions 障害・手動対応・Consumer 再利用契約の文書化）・`docs/development-environment.md` 同期。
- GitHub API は全て fixture/mock client で contract test。実 Issue 作成/更新/close・Actions dispatch・外部通信は行わない。

### Out of Scope

- `pull_request`/`push` trigger の除去・ruleset/required context 変更（#365 ルール 6・Owner 作業・#326）。既存 trigger の削除も変更もしない。
- `daily-ci.yml`（#367 並行担当）・`consumer-verify.yml` 失敗追跡（#294 の別ステップ — `repo×workflow×検証対象` の識別は scope marker で拡張可能だが本 Issue では実装しない）。
- 失敗種別（コード/依存障害/Runner 障害）の自動判定・横断 Consumer Issue の自動作成（#368 実装 6 で明示禁止）。
- 手動 close 済み Issue の再 open（方針: 再 open しない。永続する失敗は新 Issue として起票 — design で明文化）。
- GitHub Actions 自体の停止中の障害検知（tracker も起動しない — 制約として文書化・#367 scheduler 側の最終成功時刻表示と併せて運用）。
- 実 GitHub API を叩く E2E・実 Issue 起票の確認（本環境禁止・workflow 有効化は merge 後の運用段階）。

## Requirements

- R1: tracker は `workflow_run` 完了だけで起動し、対象を「この repo・`head_branch == main`・event ∈ {push, workflow_dispatch} の `ci` run」に限定する。fork run・PR run・他 workflow run・非対象 event は一切 Issue を変更しない。
- R2: 権限は tracker job のみ `contents: read`/`actions: read`/`issues: write`。workflow 全体・他 job に権限を広げない。失敗 run のコード・artifact・PR head を checkout/実行/ダウンロードしない（実行コードは default branch のもの・API メタデータのみ使用）。
- R3: 障害 Issue は label `ci-failure` + 機械可読 scope marker（repo・workflow・branch・バージョン）で同定し、marker を持たない Issue（label だけの人間 Issue 含む）を変更しない。一覧は Issue list endpoint（検索インデックス遅延の影響を受けない）を使う。
- R4: `failure`/`timed_out`/`startup_failure` を失敗系、`success` を成功、それ以外（`cancelled`/`skipped`/`neutral`/`stale`/`action_required`/null/未知）を operational として分類し、cancelled/unknown/API 失敗/履歴欠落を成功扱いしない。
- R5: 失敗時は open tracker Issue が無ければ作成・あれば `(run.id, run_attempt)` 順で新しいイベントのみ現在状態を更新（完了順逆転で古い run が新しい状態を上書きしない。同一 run の rerun は attempt で新旧を区別）。同時完了の二重起票は `concurrency` 直列化 + list endpoint 再確認で防ぐ。複数 open は最小番号 canonical + 他を duplicate close。
- R6: 成功時は `(id, attempt)` 順で失敗 run より新しく、かつ `compare` が `identical`/`ahead` の時のみ close し、回復 SHA・Run URL を記録。`behind`/`diverged`/古い run の成功では閉じない。compare API 失敗時は close せず tracker を失敗終了させる。
- R7: Issue 本文・Job Summary・ログへ出す外部由来値（SHA・URL・job 名・OS・conclusion・branch 名）は全て sanitize/allowlist 検証する。
- R8: API 呼出し（label 確保・Issue 一覧/作成/更新/close・jobs 取得・compare）の失敗は握り潰さず tracker job を失敗終了させ、Job Summary に診断可能な情報を残す。label 未作成は起動時に冪等確保する。
- R9: GitHub API は注入 client 経由とし、全判定経路を `node --test` の fixture/mock で検証する。実 Issue/実 dispatch/外部通信を伴うテストは作らない。
- R10: `ci.yml` の既存 trigger・job・required context・fail-closed `ci-summary` 契約に変更を加えない（`ci-issue-tracker.yml` は独立ファイル）。
- R11: Consumer 再利用契約 — scope（repo/workflow/branch/label/title prefix）を script 定数・workflow env で差し替えれば AudioLens/Synvelis が同一 script で運用できる形を README に明記する。

## Acceptance Criteria

- [ ] `ci-issue-tracker.yml` が `workflow_run: {workflows: [ci], types: [completed], branches: [main]}` で存在し、tracker job のみ最小権限・`concurrency` 直列化を持ち、`mise exec -- actionlint .github/workflows/ci-issue-tracker.yml` が pass する。
- [ ] `mise run test-ci-tracker`（`node --test ci/tracker/`）が以下を検証して pass: 有効 run フィルタ行列・conclusion 分類行列・初回失敗→作成・連続失敗→更新・stale run→非更新・複数 open→canonical 集約・成功 close 条件行列（identical/ahead→close・behind/diverged/R≤failureRunId→非 close・compare 失敗→非 close+失敗終了）・cancelled→operational・foreign Issue 非接触・marker 往復・sanitize・API 失敗伝播。
- [ ] `ci.yml`・`consumer-verify.yml`・ruleset 相当物に一切変更なし（`git diff` で trigger/context 不変を机上確認）。
- [ ] `docs/development-environment.md` が tracker workflow・`test-ci-tracker`・運用制約を反映する。
- [ ] `prek run --all-files` pass（check-yaml 等）。
- [ ] 実 GitHub API・実 Issue・実 dispatch を起こさない（test は mock client のみ・ローカル検証は offline）。

## Deferred Design Concerns

- `consumer-verify` 失敗の Issue 追跡 | scope marker の `workflow`/`検証対象` 軸を増やせば同契約で拡張可能だが candidate SHA+両 Consumer SHA 条件が別設計 | #294 の別ステップ | 本 Issue では `ci`×`main` のみ
- stale な open Issue（対象 workflow が rename 済み等）の TTL/手動整理 | 運用観測が無い段階で自動消込規則を決めない | 運用観測後に必要なら別 Issue | marker の `v` で将来移行可能
- GitHub Actions 全障害の外部監視 | tracker は同基盤上で動くため自力検知不可（#367 実装 8 と同じ制約） | 外部監視は後続判断 | README に手動確認手順を記載

## Sources and Relations

- [Issue #368](https://github.com/scarecrow173/ruviform/issues/368): 実装手順 1-7・Acceptance（重複抑止・誤 close 禁止・false success 禁止・権限/障害/手動対応文書化・Consumer 展開契約）。
- [Issue #365](https://github.com/scarecrow173/ruviform/issues/365): ルール 5（`ci-issue-tracker.yml` 独立監視・原則 1 件集約・後続 SHA 完全成功時のみ自動 close・逆転/二重起票/権限分離/機密保護考慮）。
- [Issue #366](https://github.com/scarecrow173/ruviform/issues/366) / `docs/specs|designs/issue-366-full-ci-stage1.md`: `ci.yml`（`name: ci`）の trigger・`ci-summary` fail-closed・`expected_sha` 契約の正本 — tracker の監視対象 run 契約。
- [Issue #367](https://github.com/scarecrow173/ruviform/issues/367): Daily scheduler（並行作業。`workflow_dispatch` 起動 run は本 tracker の対象 event）。
- [Issue #294](https://github.com/scarecrow173/ruviform/issues/294): `consumer-verify` 追跡の別ステップ候補。
