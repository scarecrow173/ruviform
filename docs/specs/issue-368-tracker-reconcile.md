---
id: "SPEC-9R6tNeU4aiEuUseUczer9o"
type: "spec"
status: "approved"
title: "Issue #368 Tracker reconcile 入口 — GITHUB_TOKEN dispatch 完了の通知 gap を default-branch 定期/手動 reconcile で埋める"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "docs/specs/issue-368-ci-issue-tracker.md"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker-reconcile.yml"
    modified:
      - type: "file"
        path: "ci/tracker/tracker.mjs"
      - type: "file"
        path: "ci/tracker/tests/tracker.test.mjs"
      - type: "file"
        path: "ci/tracker/README.md"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
    - "docs/plans/issue-368-tracker-reconcile.md"
  depends-on:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
    - "docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-367-daily-scheduler.md"
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-368-ci-issue-tracker.md"
  derived-by:
    - "docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
    - "docs/plans/issue-368-tracker-reconcile.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-368-tracker-reconcile.md"
  references:
    - "https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow"
    - "https://docs.github.com/en/actions/how-tos/manage-workflow-runs/disable-and-enable-workflows"
  defers: []
  deferred-by: []
---

# Issue #368 Tracker reconcile 入口 — GITHUB_TOKEN dispatch 完了の通知 gap を default-branch 定期/手動 reconcile で埋める

## What Should Be Built

`ci/tracker/tracker.mjs` に **reconcile モード**（`--reconcile`。event payload を持たず、権威ある run 履歴と marker 状態だけで同一の判定器を実行する入口）を追加し、それを呼ぶ **別 workflow** `.github/workflows/ci-issue-tracker-reconcile.yml`（`schedule` + `workflow_dispatch` のみ）を新設する。既存 `ci-issue-tracker.yml` の `workflow_run` 入口は**変更せず維持**し、同じ fail-closed 判定契約（最新証拠・attempt/updated_at 順序・compare 2 重 guard・pending/unknown 保留・Issue 冪等）を共有する。

## Why Now

- Issue #368 の実受入で確認済みの実運用 gap: daily scheduler（#367）が `GITHUB_TOKEN`（actor `github-actions[bot]`）で `workflow_dispatch` した Full CI は成功したが、その完了に対する `workflow_run` イベントは GitHub の anti-recursion 規約により発火せず、Tracker が自動起動しない（negative run 42108036586 → Tracker 42110069604 が #375 作成 → Scheduler 42110554416 dispatch → bot CI 42110620504 成功 → 自動通知なし。manual Replay attempt 2 が #375 を close したが自動通知復旧の証拠ではない）。
- GitHub 公式仕様: `GITHUB_TOKEN` が起こしたイベントは新しい workflow run を作らないが `workflow_dispatch`/`repository_dispatch` は例外として **dispatch 自体**は走る — ただしその完了に対する下流 `workflow_run` イベントは発火しない（公開先例・公式規約と整合する観測）。
- gap は対称: bot dispatch した Full CI の**失敗**も `workflow_run` を起こさず起票されない。recovery 専用の仕掛けではなく、履歴正本に基づく完全な状態 reconcile が必要。
- #368 レビュー判定（Issue コメント）: 既存 `ci-issue-tracker.yml` への `schedule`/`workflow_dispatch` 追加は accepted ADR（ADR-034SraR8CrkZ0SSa7tQEwh）の trigger 禁止に抵触し BLOCKED。本 spec は新 ADR（`docs/adr/issue-368-tracker-reconcile-schedule-entry.md`）でその契約を正式に補足した上での実装要求を定義する。

## Users and Value

- Ruviform maintainers: scheduler が dispatch した Full CI の失敗・回復が、人の Replay なしに Issue に反映される（起票漏れ・open 残りの両方を防ぐ）。
- Daily scheduler（#367）: 「dispatch した run の完了は tracker が拾えない」という構造的制約を、別入口の定期 reconcile が埋める。
- AudioLens / Synvelis: 同一 evaluator + 同一 workflow 骨格を固定 commit から vendor するだけで同じ gap 解消を導入できる。

## Scope

### In Scope

- `tracker.mjs` reconcile モード: `--reconcile` フラグで event payload 非読み・非必須とし、`classified` を `{kind:"reconcile"}` の合成分類として既存パイプライン（ensureLabel → open Issue 列挙 → marker merge → 権威履歴取得 → evidence 構築 → plan → apply）をそのまま実行する。`--reconcile` と `--event` の同時指定は usage error（exit 2）。reconcile モードでは `TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH` を一切参照しない（mode 間で暗黙解釈しない）。report/summary に `mode` を記録する。
- `.github/workflows/ci-issue-tracker-reconcile.yml` 新設: `schedule`（UTC・分 != 0・daily-ci の 21:17 UTC とは別時刻）+ `workflow_dispatch` のみ。checkout は default branch pin、job は `contents:read`/`actions:read`/`issues:write` の既存 track job と同一最小権限、`concurrency` は既存 tracker と**同一 group**（`ci-issue-tracker-<repo>`・`cancel-in-progress: false`）で直列化、実行前に `mise run test-ci-tracker` 自己検証、実行は `node ci/tracker/tracker.mjs --reconcile`、report artifact を `if: always()` で upload。
- `ci/tracker/README.md`: reconcile 入口の契約・手動運用（Actions UI `workflow_dispatch`）・schedule auto-disable の分離根拠・Consumer 導入契約の追記。
- `docs/development-environment.md`・`docs/adr/README.md` 等の生成 index: 新 workflow・新 ADR・新文書を反映。
- 回帰テスト: reconcile 経路の contract test を既存 `ci/tracker/tests/tracker.test.mjs` に追加（mock client のみ・外部通信なし）。

### Out of Scope

- `ci-issue-tracker.yml` への trigger 追加・変更（`workflow_run` 入口はそのまま）。
- `ci.yml`・`daily-ci.yml`・`consumer-verify.yml`・ruleset/required context の変更。PR/push trigger の撤去（後段の coordinator 作業）。
- 新 credential/secret/権限増量（PAT・GitHub App・deploy key 等は作らない）。
- scheduler（decide.mjs）への reconcile 責務の混入 — daily-ci.yml の責務は dispatch 判定のみ。
- 有限 rerun seed（30日/50回上限）に依存する恒久策。reconcile は履歴 API を正本とする pull 型であり rerun に依存しない。
- 実 GitHub API を叩く E2E・実 Issue 起票・実 cron 受入（merge 後の運用段階・本 spec では offline contract test まで）。
- AL/SV への展開そのもの（pin 更新・導入は後段の coordinator 作業。本 spec は再利用契約の記述まで）。

## Requirements

- R1: reconcile モードは event payload を必要とせず、権威ある run 履歴（`actions/workflows/{file}/runs?branch=main&exclude_pull_requests=true`）と marker 状態のみで既存判定器と同一の plan/apply を行う。event 候補が無いだけで、証拠構築・ordering・close guard・Issue 冪等は全て共通。
- R2: `ci-issue-tracker.yml` の trigger・job・権限・concurrency を一切変更しない。
- R3: 新 workflow は `schedule` + `workflow_dispatch` のみを持ち、`pull_request`/`push`/`workflow_run` は持たない。未改変の workflow は常に default branch のコードを実行する（schedule は default branch のみで走る。dispatch 実行時も checkout を default branch ref に pin し、非 default ref での起動は job guard で skip する。dispatch が選択 ref の workflow file を使う GitHub 仕様上、workflow file 自体を編集できる write 権限者からは守れない — `daily-ci.yml` と同一の露出）。
- R4: 権限は既存 track job と同一（`contents:read`/`actions:read`/`issues:write` を reconcile job のみに付与・workflow 直下は `contents: read`）。新規 secret・追加権限・外部サービスは作らない。
- R5: `concurrency` group は既存 tracker と同一（`ci-issue-tracker-${{ github.repository }}`）とし、event 入口 run との書き込み競合を直列化する（`cancel-in-progress: false`）。
- R6: reconcile 実行も fail-closed を維持する — 履歴 API 失敗・非配列レスポンス・close 候補での branch head 不在は job 失敗、unknown/pending evidence は close・stale-failure 抑止を止める、API 失敗は握り潰さず伝播する。
- R7: `--reconcile` と `--event`（明示 event 指定）の同時指定は usage error。reconcile モードは `TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH` を参照しない — event 入口と reconcile 入口で event path の解釈を共有しない。
- R8: report（JSON）と Job Summary に `mode`（event/reconcile）を記録し、診断時にどちらの入口が動いたか識別できるようにする。
- R9: schedule の cron は UTC・分 != 0・daily-ci（`17 21 * * *`）と別時刻とし、公開 repo の 60 日無活動による scheduled workflow auto-disable が **event 入口ファイルに及ばない**よう、`schedule` は reconcile 専用ファイルに閉じ込める（GitHub は workflow 単位で disable し、disabled workflow は全 trigger に応答しない — 公式仕様）。
- R10: 全判定経路を `node --test` の mock client contract test で検証する。実 Issue/実 dispatch/外部通信を伴うテストは作らない。

## Acceptance Criteria

- [ ] `.github/workflows/ci-issue-tracker-reconcile.yml` が `schedule` + `workflow_dispatch` のみで存在し、reconcile job のみ最小権限・tracker と同一 concurrency group・self-test 先行で `tracker.mjs --reconcile` を呼び、`mise exec -- actionlint` が pass する。
- [ ] `mise run test-ci-tracker` が pass し、reconcile 経路の contract test を含む: bot 成功履歴 + open 失敗 Issue → `close-recovered`（#375 相当の回復）、bot 失敗履歴 + Issue 無し → `create`（起票漏れの補完）、成功履歴のみ + Issue 無し → `noop`、pending/unknown 履歴では close しない、履歴 API 失敗は伝播、`--reconcile`+`--event` 同時指定は usage error、reconcile で event env path を参照しない。
- [ ] `git diff` で `ci-issue-tracker.yml`・`ci.yml`・`daily-ci.yml`・`consumer-verify.yml` の trigger/権限に変更がない（reconcile の追加のみ）。
- [ ] `docs/adr/issue-368-tracker-reconcile-schedule-entry.md` が ADR-034SraR8CrkZ0SSa7tQEwh との関係（trigger 禁止の補足・event 入口維持）を記録し、独立レビュー済み。
- [ ] `docs/development-environment.md`・`ci/tracker/README.md`・生成 index が新 workflow・新モードを反映する。
- [ ] `mise exec -- prek run --all-files` pass。
- [ ] 実 GitHub API・実 Issue・実 dispatch・実 cron を起こさない（offline 検証のみ。実 cron 受入は merge 後・別工程）。

## Deferred Design Concerns

- schedule auto-disable（60 日無活動）自体の監視 | reconcile workflow が止まっても event 入口は残る設計。disabled 検知・再有効化手順は README 運用節に記載 | 外部監視は #367 と同じく後続判断 | 本 spec ではファイル分離による blast radius 限定まで
- reconcile 頻度と Actions コスト | 初期値は 6 時間間隔（UTC・分 != 0）。運用観測で必要なら調整 | 観測後に別判断 | 頻度は契約ではなく運用パラメータ
- fork/外部 schedule 実行 | public fork では scheduled workflow が既定 disable。reconcile は upstream repo 運用のみを想定 | Consumer README に同じ注記を残す | 変更不要
- tracker 自身（reconcile run）の失敗通知 | reconcile が失敗しても通知先は無い（event tracker と同じ制約）。Job Summary/report artifact の日次確認 + daily 運用で観測 | 運用段階で判断 | 変更不要

## Sources and Relations

- [Issue #368](https://github.com/scarecrow173/ruviform/issues/368): Tracker 本体要求・最新コメントのレビュー判定（既存 tracker への trigger 追加は BLOCKED・新 ADR + 契約更新が先・GITHUB_TOKEN gap の恒久策は default-branch 定期/手動 reconcile）。
- [Issue #365](https://github.com/scarecrow173/ruviform/issues/365): ルール 5（独立監視・1 Issue 集約・後続完全成功のみ close）とルール 4（scheduler 起動 Full CI の追跡）— bot dispatch の追跡不能は両ルールの前提を崩す。
- [Issue #367](https://github.com/scarecrow173/ruviform/issues/367): Daily scheduler — `GITHUB_TOKEN` による `workflow_dispatch` が通知 gap の起点。
- `docs/specs/issue-368-ci-issue-tracker.md`（SPEC-4KMssIOpNuvh9jGgKKfp7R）: Tracker 本体 spec — 本 spec はその入口拡張。
- `docs/adr/issue-368-tracker-reconcile-schedule-entry.md`: 本変更の trigger 追加を正式に決定する新 ADR（ADR-034SraR8CrkZ0SSa7tQEwh の trigger 禁止を補足）。
- GitHub 公式文書: anti-recursion（`GITHUB_TOKEN` 起因イベントは run を作らない・`workflow_dispatch`/`repository_dispatch` のみ例外）/ scheduled workflow auto-disable（public repo 60 日無活動・workflow 単位 disable）/ rerun 上限（初回 run から 30 日・最大 50 回 — 有限 seed を恒久策にしない根拠）。
