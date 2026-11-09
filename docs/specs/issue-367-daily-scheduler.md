---
id: "SPEC-ElrrvM1k7RbACtLgfzKaxs"
type: "spec"
status: "approved"
title: "Issue #367 共通 Daily Scheduler — `daily-ci.yml` 軽量判定 + `ci.yml` 条件付き dispatch"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/366"
  changes:
    added:
      - type: "directory"
        path: "ci/daily"
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
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
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/plans/issue-367-daily-scheduler.md"
  depends-on:
    - "docs/specs/issue-366-full-ci-stage1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-366-full-ci-stage1.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/plans/issue-367-daily-scheduler.md"
    - "docs/specs/issue-294-daily-tuple-trigger.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-367-daily-scheduler.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/159"
    - "https://github.com/scarecrow173/synvelis/issues/167"
  defers: []
  deferred-by: []
---

# Issue #367 共通 Daily Scheduler — `daily-ci.yml` 軽量判定 + `ci.yml` 条件付き dispatch

## What Should Be Built

`.github/workflows/daily-ci.yml` を新設し、毎日 UTC schedule で「現在 `main` HEAD が有効な Full CI 済みか」を軽量 job で判定し、未検証の場合に限り `ci.yml` を `workflow_dispatch` で起動する Daily scheduler を実装する。判定器は `ci/daily/decide.mjs`（stdlib-only Node CLI）として GitHub Actions API 履歴を厳格に評価し、重複 dispatch 抑止・同一 SHA 自動 dispatch 上限・dispatch 後の SHA race 検出を行う。状態正本は GitHub Actions 履歴そのものであり、専用 DB・repository variable を持たない。汎用コードは Ruviform 所有で、両 Consumer（AudioLens #159 / Synvelis #167）が Ruviform の Cargo/Frontend を build/install せずに導入できる再利用契約を定義する。

## Why Now

- Issue #365（横断移行計画・正本）の順序: Full CI 全実行化（#366・PR #370 統合済み）の次が Scheduler/Tracker の安全稼働。`ci.yml` の `expected_sha` 入力契約は #366 で導入済みだが、それを発火する daily 起動側がまだ存在しない。
- #365 決定 1: 「PR/push 通常 CI を廃止し、Daily が未検証変更がある時だけ Full CI を起動」する最終形の判定側。移行中は PR/push trigger を残すため、push 発の Full CI run も「有効な検証」として正しく数えることが二重起動抑止に必須。
- #368（CI 失敗 Issue tracker）と並行実装。scheduler は Issue を作らず、失敗の Issue 集約は tracker の責務 — scheduler の retry 上限超過を「検証不可の継続」として tracker/運用が拾えるよう、判定理由を Job Summary/JSON で明示する。

## Users and Value

- Ruviform maintainers: `main` の未検証 revision が翌朝までに自動検証され、検証済み/実行中/リトライ枯渇が Job Summary と `daily-ci-report` JSON で追跡できる。
- Consumer repos（AudioLens/Synvelis）: `ci/daily/` + `daily-ci.yml` を薄い設定差分で導入できる汎用契約。権限は単一 job の `actions: write`（`contents: read` 含む）に限定。
- 後続の trigger 移行（#326/#365 順序 4）: required context 解除後に PR/push trigger を止めても、検証経路（daily dispatch + 手動 dispatch）が先に稼働済みとなる。

## Scope

### In Scope

- `.github/workflows/daily-ci.yml` 新設 — `schedule`（UTC、日本時間 Daily 相当・毎時 0 分以外）+ `workflow_dispatch`（手動再判定、`dry_run` 入力付き）、単一軽量 ubuntu job、job 限定 permissions（`contents: read` + `actions: write`）、`concurrency` による scheduler 自身の重複抑止、`test-ci-daily` 自己検証先行、`daily-ci-report` JSON artifact。
- `ci/daily/decide.mjs` — GitHub Actions API（`git/ref`・`actions/workflows/{w}/runs` ページネーション・`dispatches` POST）の stdlib-only Node CLI。fixture 再生（`--fixture`）と `--dry-run` を持つ。
- 判定契約: `main` SHA・workflow・event・head_branch・run_attempt・created_at・status/conclusion の厳格対応付け、success→skip / in-flight→抑止 / 非 success・履歴なし→retry 候補、同一 SHA の bot 自動 dispatch 上限 2（初回 1 + retry 1）、API 失敗・未知状態・履歴欠落の fail-closed。
- dispatch 契約: `ref: main` + `expected_sha` 入力、直前の `main` 再照合、dispatch 後の有界確認 poll・着地 SHA 報告。
- `mise.toml` に `test-ci-daily` task、`docs/development-environment.md` に workflow 構成・運用手順・Consumer 採用契約の同期。
- GitHub API は fixture/mock のみでテスト。実際の dispatch 等の副作用は発生させない。

### Out of Scope

- `pull_request`/`push` trigger の除去・ruleset/required context 変更（Owner 作業・#365 順序 4）。
- `ci-issue-tracker.yml`・失敗 Issue 自動作成（Issue #368 並行実装。scheduler は Issue を作らない）。
- `ci.yml` の入力契約変更（`expected_sha` は #366 で確定済み。本件は新入力を追加しない）。
- `consumer-verify.yml`・Cross-Consumer 候補 SHA Gate（#294）。
- GitHub 障害による定期 run 欠落の自己検出・外部監視（運用手順と「最終成功検証時刻」の可視化を整備し、外部監視は後続判断 — #367 実装項目 8）。
- 実 Actions への dispatch 実演・実機動作確認（禁止事項。fixture 検証と運用手順の記録で代替）。

## Requirements

- R1: `daily-ci.yml` は UTC cron（日本時間 Daily 相当・分 ≠ 0）+ `workflow_dispatch` で起動し、軽量 Ubuntu job のみを持つ。`actions: write` は dispatch を行うその 1 job に限定し、Full CI は `ci.yml` `workflow_dispatch` 専用とする。
- R2: 判定は `ci.yml`（workflow file 指定）の run 履歴をページネーション取得し、`head_sha == 現在 main SHA`・`head_branch == main`・Full CI event allowlist（`push`/`workflow_dispatch`/`schedule`）で厳格に対応付ける。scheduler 自身・他 workflow・他 branch・他 SHA・pull_request run を混同しない。run の `status`/`conclusion`/`run_attempt` は最新 attempt を意味する（list API 契約）。
- R3: 同一 SHA の最新有効 Full CI が `conclusion: success` なら `skip-verified`（dispatch しない）。`queued`/`waiting`/`in_progress`/`pending`/`requested` が 1 件でもあれば `suppress-in-flight`（重複起動しない）。"最新" の同定は最新 attempt の活動時刻（`created_at`/`run_started_at` — rerun は `created_at` を動かさない）に基づき、tie・順序不能・`updated_at` が示す未解明の活動など曖昧な場合は `skip-verified` に倒さず `hold-unknown` とする（偽 success を記録しない）。
- R4: `failure`/`cancelled`/`timed_out`/その他既知の非 success 終了・履歴なし（保持期間外含む）は retry 候補。同一 SHA に対する自動 dispatch が上限 `2`（初回 + 再試行 1 回）に達したら `budget-exhausted` で dispatch せず、警告を発し手動実行または SHA 更新を待つ。`workflow_dispatch` run は `actor.login` と `triggering_actor.login` が**両方とも**識別可能な非 auto actor（既定 auto actor は `github-actions[bot]`）の場合のみ手動扱い — 片側が bot・null・空・非 object 等で識別不能なら自動発火を否定できないため計上する（人の rerun で budget は払い戻されない）。
- R5: API 失敗・履歴未取得・未知の status/conclusion 値・ref 解読不能は `error`/`hold-unknown` として失敗・保留を明示し、skip/dispatch を誤判定しない（fail-closed）。`workflow_runs` を欠く 200 等の malformed response、および各 record の識別・状態 metadata（`id`/`head_sha`/`head_branch`/`event`/`status`）欠落は `error`。schema 上は有効だが対象か否か立証できない record（例: 破損 sha）は `hold-unknown` — 除外は positive proof 制で、立証不能な record は success の影にも隠れて `skip-verified` にしない。ページネーション打ち切り（`truncated`）は `history-truncated` outcome で保留し warning・`historyTruncated` を記録する — 不完全な履歴では success 判定・dispatch のいずれも確定しない。
- R6: dispatch は `ref: main` + `inputs.expected_sha = 判定 SHA` で発火する。判定と dispatch の間に `main` が動いた場合（直前の ref 再照合で検出）は古い SHA で dispatch せず再判定し、有界回数を超えて動き続けるなら `main-moved` として次回に委ねる。GitHub API が `ref` の SHA 解決を保証すると仮定しない。
- R7: dispatch 後は新規 `workflow_dispatch` run の出現を有界 poll で確認し、着地 `head_sha` を記録する。着地 SHA ≠ 期待 SHA は `dispatch-landed-mismatch` として報告（その run は `expected_sha` 不一致で `ci-summary` が Fail にする — 誤 SHA を検証済み扱いしない）。poll timeout は `dispatched-unconfirmed`（dispatch 成功・確認遅延）として warning 報告する。
- R8: Job Summary と `daily-ci-report` JSON artifact に、判定結果・理由・判定 SHA・最新成功 SHA とその時刻・対象 run 一覧・自動 dispatch 回数/上限・dispatch の Run URL/着地 SHA を記録する。外部秘密・長期 DB・repository variable を状態正本にしない。
- R9: scheduler workflow は `concurrency` で直列化し `cancel-in-progress: false`（判定・dispatch の途中打ち切りをしない）。手動 `workflow_dispatch` でも同一経路を通し、`dry_run` 入力で判定のみの実行を可能にする。
- R10: 汎用コードは Ruviform 所有。Consumer 導入は `ci/daily/` + `daily-ci.yml` の複製（vendoring）と env 設定差分で完結し、Ruviform の Cargo/Frontend build/install・外部サービス・追加秘密を要求しない。権限最小化と採用契約を文書化する。
- R11: 定期 run の GitHub 側欠落は本方式で自己検出できないことを運用手順に明記し、「最終成功検証時刻」を Job Summary/report に常時記録して手動再判定の手順を提供する。

## Acceptance Criteria

- [ ] 同一 SHA `success` → `skip-verified` で dispatch しない。SHA 変更/履歴なし → `dispatch`。同一 SHA 実行中/queued → `suppress-in-flight` で dispatch しない。
- [ ] `failure`/`cancelled`/`timed_out` → 上限内 retry、自動 dispatch が上限 2 に到達 → `budget-exhausted` で警告・dispatch しない。次の SHA では budget がリセットされ再開する。
- [ ] API 障害・履歴未取得・未知 status・判定~dispatch 間の main 更新・他者の手動 dispatch 重複を誤成功/誤 dispatch しない（fixture test で証明）。
- [ ] Job Summary/report・`expected_sha` 入力・判定 SHA の一致が fixture test と文書で検証される。
- [ ] Daily に変更がない（=成功履歴がある）場合、重い Runner（Full CI）は起動しない（`skip-verified`）。
- [ ] `mise run test-ci-daily`（`node --test ci/daily/`）が判定行列・dispatch 確認・report 形状をカバーし exit 0。
- [ ] `mise exec -- actionlint .github/workflows/daily-ci.yml`・`mise exec -- prek run --all-files` が pass。
- [ ] `docs/development-environment.md` が daily-ci 構成・運用手順（手動再判定・dry_run・最終成功時刻の読み方）・Consumer 採用契約・権限最小化を反映する。
- [ ] 実 Actions への dispatch 等の副作用なし（全検証は fixture/offline）。

## Deferred Design Concerns

- 定期 run 欠落の外部監視 | GitHub 障害時に scheduler 自身が起きないと自己検出不能（#367 項目 8 の固有制約） | 運用観測後に別 Issue で外部監視を判断 | 最終成功時刻の可視化で代替
- 失敗 Issue 自動集約 | #368 `ci-issue-tracker.yml` の責務 | tracker 実装完了時 | scheduler が Issue を重複作成しない
- `bot actor` による自動 dispatch 識別の厳密化 | list API に run inputs が無く、`triggering_actor.login` で代替（他 workflow からの自動 dispatch も同一 budget に計上 — 安全側） | API が inputs を公開した時点で厳密化候補 | 現時点では保守的に計上
- AudioLens/Synvelis 実導入 | 各 repo の Owner 作業（AL #159 / SV #167） | 本契約の横展開時 | 本 spec は採用契約の定義まで

## Sources and Relations

- [Issue #367](https://github.com/scarecrow173/ruviform/issues/367): 共通 Daily Scheduler 実装要件（実装項目 1-8・Acceptance 6 件）。
- [Issue #365](https://github.com/scarecrow173/ruviform/issues/365): 横断移行計画（決定 1・4「同一 main SHA の有効な Full CI 最新結果で判定」・移行不変条件）。
- [Issue #366](https://github.com/scarecrow173/ruviform/issues/366) / PR #370: `ci.yml` Full CI 化・`ci-summary` fail-closed・`expected_sha` 入力契約（本 spec の dispatch 先）。
- [Issue #368](https://github.com/scarecrow173/ruviform/issues/368): 並行実装の失敗 Issue tracker（scheduler は Issue を作らない境界）。
- Consumer 導入: [AudioLens #159](https://github.com/scarecrow173/audio-lens/issues/159) / [Synvelis #167](https://github.com/scarecrow173/synvelis/issues/167)（Ruviform 共通部分と Consumer 固有設定の Owner 分離）。
