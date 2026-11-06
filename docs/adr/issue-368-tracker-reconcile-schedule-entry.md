---
id: "ADR-vJvyZNtrHi2xnycJxbnSPv"
type: "adr"
status: "accepted"
title: "Issue #368 Tracker reconcile 入口 — GITHUB_TOKEN 通知 gap を埋める別 workflow 定期/手動 reconcile（ADR-034SraR8CrkZ0SSa7tQEwh trigger 禁止の補足）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "docs/specs/issue-368-tracker-reconcile.md"
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
  implements:
    - "docs/specs/issue-368-tracker-reconcile.md"
  implemented-by:
    - "docs/designs/issue-368-tracker-reconcile.md"
  depends-on:
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
  refines:
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-368-tracker-reconcile.md"
  derived-by:
    - "docs/designs/issue-368-tracker-reconcile.md"
  verifies: []
  verified-by: []
  references:
    - "https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow"
    - "https://docs.github.com/en/actions/how-tos/manage-workflow-runs/disable-and-enable-workflows"
    - "https://docs.github.com/en/actions/how-tos/troubleshoot-workflows"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #368 Tracker reconcile 入口 — GITHUB_TOKEN 通知 gap を埋める別 workflow 定期/手動 reconcile

## Context and Problem Statement

ADR-034SraR8CrkZ0SSa7tQEwh（accepted）は `ci-issue-tracker.yml` を `workflow_run` 専用とし、「tracker は ci 完了にのみ反応する」ため `pull_request`/`push`/`schedule`/`workflow_dispatch` trigger を禁止した。しかし実受入で、同 ADR が前提とした検知モデルの穴が判明した: daily scheduler（#367）が `GITHUB_TOKEN` で `workflow_dispatch` した `ci` run は実行されるが、GitHub の anti-recursion 規約によりその完了は `workflow_run` イベントを発火させない（公式仕様: `GITHUB_TOKEN` 起因イベントは新しい run を作らず、例外は `workflow_dispatch`/`repository_dispatch` の dispatch 自体のみ — 下流 `workflow_run` は発火しない）。実測では bot dispatch の Full CI 成功後に Tracker が自動起動せず、失敗 Issue #375 は manual Replay まで open のままだった。gap は対称であり、bot dispatch の**失敗**も起票されない。

## Decision Drivers

- 恒久性: rerun seed（初回 run から 30 日・最大 50 回の公式上限）は有限であり恒久策にしない。通知欠落を pull 型で自己修復する構造が必要。
- ADR-034SraR8CrkZ0SSa7tQEwh の fail-closed 契約を緩めない: 権威履歴 reconciliation・NEWEST/attempt/updated_at 順序・compare 2 重 guard・pending/unknown 保留・Issue 冪等を共通判定器として共有する。
- event 入口の保全: GitHub は workflow **単位**で scheduled workflow を disable する（public repo 60 日無活動・disabled workflow は全 trigger に応答しない — 公式仕様）。`schedule` を event 入口ファイルに置くと auto-disable が `workflow_run` 入口まで及び得るため、cron を持つ要素は別ファイルに閉じ込める。
- 責務分離: daily-ci.yml（scheduler = dispatch 判定）と tracker（Issue ライフサイクル）の失敗・権限・ concurrency 領域を混ぜない。
- 最小権限・既存 secret のみ: PAT・GitHub App・deploy key 等の新 credential は作らない（task 制約）。`GITHUB_TOKEN` の job 単位最小権限（`contents:read`/`actions:read`/`issues:write`）をそのまま使う。
- default-branch trusted code のみ: schedule は default branch でのみ走る。dispatch 実行時も checkout を default branch ref に pin し、失敗 run のコード・artifact・PR head を実行しない信頼モデルを維持する。

## Considered Options

- **Option A（採用）**: 別 workflow `ci-issue-tracker-reconcile.yml`（`schedule` + `workflow_dispatch`）が `tracker.mjs --reconcile` を呼ぶ。reconcile モードは event payload を持たず権威履歴 + marker 状態のみで同一判定器を走らせる。既存 `workflow_run` 入口は不変更。
- **Option B**: 既存 `ci-issue-tracker.yml` に `schedule`/`workflow_dispatch` を追加。**却下** — (1) ADR-034SraR8CrkZ0SSa7tQEwh の accepted trigger 禁止を本 ADR なしに破る、(2) workflow 単位 auto-disable が `workflow_run` 入口まで及ぶ（schedule を持つ workflow が disable されると event 入口も応答しない）、(3) event 専用ファイルという blast radius 分離を失う。
- **Option C**: `daily-ci.yml` に reconcile job を追加。**却下** — scheduler の責務（dispatch 判定）と tracker の責務（Issue 集約）を混ぜ、reconcile 失敗が scheduler run の結論を濁す。権限も `actions: write`（scheduler）と `issues: write`（tracker）で別 job にできるが、同ファイル・同 concurrency・同 trigger への結合は契約を曖昧にする。reconcile 頻度を scheduler 頻度に拘束される点も不利。
- **Option D**: scheduler の dispatch に PAT/GitHub App token を使い `workflow_run` を発火させる。**却下** — 新 credential/secret を要求し task 制約に反する。GITHUB_TOKEN の anti-recursion は仕様であり、迂回は権限増量コストに見合わない（pull 型 reconcile で足りる）。
- **Option E**: tracker の workflow_run 入口が起動しなかった検知を別の監視（外部サービス・repo variable 時刻照合）で行う。**却下** — 外部依存・状態正本の増加で複雑。権威履歴は既に必要十分な情報源であり、定期 pull で同じ判定器を回す方が単純。
- **Option F**: `ci.yml` 完了時に自分で Issue を書く（ADR-034SraR8CrkZ0SSa7tQEwh Option B 再考）。**却下** — 理由は同 ADR のまま（権限混在・fork 権限昇格リスク）。今回の gap は tracker 側の入口問題であり、監視対象 workflow に責務を移す必要はない。

## Decision Outcome

Chosen option: **Option A**。

- `ci/tracker/tracker.mjs` に `--reconcile` モードを追加する。event payload を読まず `TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH` を参照せず、`--event` との同時指定は usage error（exit 2）とする。`run()` は `reconcile` 指定時に `classified` を `{kind:"reconcile"}` の合成分類として既存パイプラインを共通実行する — event 候補が無いだけで、label 冪等・Issue 列挙・marker merge・権威履歴取得・3 分類（in/out/unknown）・pending/unknown fail-closed・plan/apply は全て同一経路。
- `.github/workflows/ci-issue-tracker-reconcile.yml` を新設する。trigger は `schedule`（UTC・分 != 0・初期値 `23 */6 * * *` = 6 時間間隔）と `workflow_dispatch` のみ。`pull_request`/`push`/`workflow_run` は持たない。reconcile job のみ `contents:read`/`actions:read`/`issues:write`、workflow 直下 `permissions: contents: read`。`concurrency` は既存 tracker と同一 group（`ci-issue-tracker-${{ github.repository }}`・`cancel-in-progress: false`）で直列化する。checkout は `ref: ${{ github.event.repository.default_branch || 'main' }}` で default branch code に pin し（expression が空に解決した場合の triggering-ref fallback を明示既定値で防ぐ）、job は `github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch` の時のみ実行する。schedule は定義上 default branch でのみ走り payload に `repository` が欠落し得るため無条件通過とし、ref 比較は dispatch の非 default ref 起動を no-op にする防御としてのみ効く。job には `timeout-minutes: 15` を設け、hang が共有 concurrency group を占有し続けるのを防ぐ。
- `ci-issue-tracker.yml` は**一切変更しない**（`workflow_run` 入口・権限・concurrency・コメントの trigger 禁止記述を維持）。本 ADR は ADR-034SraR8CrkZ0SSa7tQEwh の trigger 禁止を「`ci-issue-tracker.yml` ファイルへの trigger 追加は引き続き禁止。同一 evaluator を呼ぶ別 workflow ファイルの `schedule`/`workflow_dispatch` は許可」と補足する — ADR-034SraR8CrkZ0SSa7tQEwh 自体の status・理由・禁止範囲（当該ファイル）は書き換えない。
- report/Job Summary に `mode`（`event`/`reconcile`）を記録し、どちらの入口が動いたかを診断可能にする。

### Consequences

- Good, because 通知 gap が default-branch trusted code の定期 pull で自己修復される（失敗起票・回復 close の両方向）。既存 fail-closed 契約・権限・secret を増やさず、cron auto-disable の影響範囲を event 入口から分離できる。同一 evaluator を呼ぶため判定ドリフトが起きない。有限 rerun seed・外部状態・新 credential に依存しない。
- Bad, because workflow が 1 本増え、cron 停止（60 日無活動・actor 停止）で reconcile が止まると gap が戻る（event 入口は残るため push/手動 dispatch の追跡には影響しない — README に運用確認手順を記録）。schedule の最小粒度・配送遅延で検知は即時ではなくなる（初期 6 時間間隔 — tracker は Issue 管理であり許容）。
- Neutral: ADR-034SraR8CrkZ0SSa7tQEwh の Option C（schedule ポーリング）却下は「主検知方式」としての評価であり、本 ADR はイベント主入口を維持した上での**補完入口**として schedule を採用する — 旧決定と矛盾しない。Issue model（scope あたり 1 open episode・手動 close 尊重）も変わらない。

### Confirmation

`ci/tracker/tests/tracker.test.mjs` の mock client contract test に reconcile 経路を追加し、`mise run test-ci-tracker` で検証する。`mise exec -- actionlint .github/workflows/ci-issue-tracker-reconcile.yml` で workflow 構文を検証する。実 cron・実 Issue・実 dispatch の受入は merge 後の運用段階（本 ADR では offline 検証まで）。

## Implementation Plan

- Affected paths: `ci/tracker/tracker.mjs`（reconcile モード + arg 解析分離）、`ci/tracker/tests/tracker.test.mjs`（reconcile 契約 test）、`.github/workflows/ci-issue-tracker-reconcile.yml`（新設）、`ci/tracker/README.md`・`docs/development-environment.md`（同期）。
- Patterns to follow: `daily-ci.yml` の schedule + workflow_dispatch 骨格・`ci-issue-tracker.yml` の最小権限/concurrency/self-test/checkout pin・`ci/daily/decide.mjs` の `--dry-run` 的な明示モード引数設計。
- Patterns to avoid: `ci-issue-tracker.yml` への trigger 追加、`daily-ci.yml` への tracker 責務混入、`TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH` の mode 間暗黙解釈、新規 secret/権限増量、未信頼コード/artifact の実行、event なしでの `classifyRun` 呼出し（payload 必須前提）、有限 rerun seed への依存、fail-closed guard の緩和。
- Dependencies or configuration: 既存 `mise`/`node`/`gh` CLI・`secrets.GITHUB_TOKEN` のみ。cron は `23 */6 * * *`（UTC・分 != 0・daily-ci `17 21 * * *` と別時刻）。

## Verification

- [ ] `mise run test-ci-tracker` → pass（reconcile 契約 test 含む）。
- [ ] `mise exec -- actionlint .github/workflows/ci-issue-tracker-reconcile.yml` → pass。
- [ ] `mise exec -- prek run --all-files` → pass。
- [ ] `git diff` で `ci-issue-tracker.yml`/`ci.yml`/`daily-ci.yml`/`consumer-verify.yml` 無変更。
- [ ] merge 後・実 cron 受入（本工程範囲外 — 別工程で実測記録）。

## Pros and Cons of the Options

### Option A（採用）— 別 workflow 定期/手動 reconcile

- Good, because event 入口の契約を温存しつつ gap を pull 型で埋める・auto-disable の blast radius を分離・判定器を共有するため契約ドリフトなし・新 credential 不要。
- Bad, because workflow が増える・cron 依存の検知遅延（即時性は event 入口が担う）・cron 停止自体の検知は運用確認に依存。

### Option B — 既存 tracker への trigger 追加

- Good, because 実装が最小。
- Bad, because ADR-034SraR8CrkZ0SSa7tQEwh 違反・auto-disable が event 入口に及ぶ・ファイルの責務が曖昧。

### Option C — daily-ci.yml 内 reconcile job

- Good, because 既存 schedule の再利用でファイル増なし。
- Bad, because scheduler と tracker の責務・失敗領域・権限・concurrency が混ざり、「scheduler green = 判定完走」の意味を濁す。

### Option D — PAT/App token で dispatch

- Good, because `workflow_run` が普通に発火する。
- Bad, because 新 credential/secret が必須 — task 制約と最小権限の原則に反する。

### Option E — 外部監視/時刻照合

- Good, because pull 型 reconcile より検知を早められる可能性。
- Bad, because 外部依存・状態正本の追加で複雑。権威履歴で足りる。

### Option F — ci.yml 内 Issue 管理

- Good, because 実装が 1 workflow にまとまる。
- Bad, because ADR-034SraR8CrkZ0SSa7tQEwh Option B の却下理由（権限混在・fork 昇格）が変わらない。

## More Information

- Issue #368（通知 gap の実測記録・レビュー判定）・#365（ルール 4/5）・#367（scheduler — gap の起点）。
- ADR-034SraR8CrkZ0SSa7tQEwh: 本 ADR が補足する本体決定（trigger 禁止は当該ファイルに限定して維持）。
- GitHub 公式文書: anti-recursion・scheduled auto-disable（public 60 日・workflow 単位・disabled は全 trigger 不応答）・cron 停止の追加ケース（actor 停止時は cron が enabled のまま動かなくなり得る）・rerun 30 日/50 回上限。
- `docs/specs/issue-368-tracker-reconcile.md`（実装要求・受入基準）・`docs/designs/issue-368-tracker-reconcile.md`（設計詳細）。
