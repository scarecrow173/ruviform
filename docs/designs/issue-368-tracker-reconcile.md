---
id: "DESIGN-StMgstUBs2Df2UPm1oy2GF"
type: "design"
status: "approved"
title: "Issue #368 Tracker reconcile 設計 — event 非依存の同一判定器入口・別 workflow schedule/dispatch・mode 分離"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
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
    - "docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
  implemented-by:
    - "docs/plans/issue-368-tracker-reconcile.md"
  depends-on:
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
  derived-by:
    - "docs/plans/issue-368-tracker-reconcile.md"
    - "docs/test-specs/issue-368-tracker-reconcile.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-368-tracker-reconcile.md"
  references:
    - "https://docs.github.com/en/actions/how-tos/manage-workflow-runs/disable-and-enable-workflows"
  defers: []
  deferred-by: []
---

# Issue #368 Tracker reconcile 設計 — event 非依存の同一判定器入口・別 workflow schedule/dispatch・mode 分離

## Context

本体設計（`docs/designs/issue-368-ci-issue-tracker.md`）は event を「トリガ」権威履歴を「真実」とする reconciliation を既に持つ。今回の gap は **event 自体が発火しない**ケース — `GITHUB_TOKEN`（actor `github-actions[bot]`）で dispatch された `ci` run の完了には `workflow_run` が発火しない（GitHub 公式 anti-recursion）。本体設計の「イベントが落ちても次のイベントが履歴で復元する」収束条件は「次に処理されるイベント」に依存しており、bot dispatch 完了ではそのイベント自体が無い。

解決は同じ判定器を **event を持たない入口**から呼ぶだけでよい: 権威履歴 + marker 状態は reconcile に必要十分な入力であり、event payload は証拠構築で `toCand(classified.run, "event")` の 1 候補に過ぎない。残る設計上の問いは (1) reconcile をどこから起動するか、(2) event path の取り扱いを mode 間でどう分離するか、(3) `run()`/`main()` のどこに mode を入れるか。

## Components and Boundaries

### `.github/workflows/ci-issue-tracker-reconcile.yml`（新設）

- `name: ci-issue-tracker-reconcile`。`on.schedule: [{cron: "23 */6 * * *"}]`（UTC・分 != 0・6 時間間隔。daily-ci `17 21 * * *` と別時刻）+ `workflow_dispatch:`（入力なし）。`pull_request`/`push`/`workflow_run` は持たない。
- `permissions: contents: read`（workflow 直下・deny by default の上で read のみ）。
- `concurrency`: **既存 tracker と同一 group** `ci-issue-tracker-${{ github.repository }}`・`cancel-in-progress: false`。event 入口 run と reconcile run が同時に走らないよう workflow を跨いで直列化する（group は workflow 名を含まないため共有できる。pending 置換で reconcile が落ちても次回 schedule / event で回復する — reconcile は冪等で、どちらが最後に走っても権威履歴から同じ結論に収束する）。
- `env.MISE_ENABLE_TOOLS: node`（track job と同じ toolchain pin）。
- job `reconcile`（`ubuntu-latest`）:
  - `if: github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch` — `workflow_dispatch` を非 default ref で起動された場合に no-op skip とする防御。schedule は定義上 default branch でのみ走るため無条件通過とし、比較は dispatch 起動時にのみ効く。`github.event.repository` は schedule payload では欠落し得る（GitHub 仕様「Webhook event payload: Not applicable」）ため、schedule 側に `default_branch` 比較を掛けると空に解決して cron 全 skip のサイレント死に陥る — 比較は payload に `repository` を持つ dispatch に限定する。
  - `timeout-minutes: 15` — hang した step が共有 concurrency group を占有し続けて pending 置換窓を広げるのを防ぐ（daily-ci.yml と同じ上限）。
  - `permissions: {contents: read, actions: read, issues: write}` — track job と同一最小権限。
  - steps: `actions/checkout@v7.0.1`（`ref: ${{ github.event.repository.default_branch || 'main' }}` で default branch code に pin — dispatch で別 ref が選ばれても tracker コードは main のもの。expression が空に解決した場合 checkout が triggering ref に fall back するため明示的既定値を置く）→ `jdx/mise-action@v5.2.0`（`install: false`）→ `mise trust && mise install --locked node` → `mise run test-ci-tracker` 自己検証 → `GH_TOKEN`/`GITHUB_REPOSITORY` env で `node ci/tracker/tracker.mjs --reconcile --report ci-issue-tracker-report.json`（`TRACKER_EVENT_PATH` は**渡さない**。`GITHUB_EVENT_PATH` は runner が自動設定するが script は reconcile モードで参照しない）→ `actions/upload-artifact@v4` で report を `if: always()` upload（名前 `ci-issue-tracker-reconcile-report-attempt-<n>` で event 入口 artifact と区別）。
- `ci-issue-tracker.yml` への trigger 追加・変更はしない（ADR-034SraR8CrkZ0SSa7tQEwh の trigger 禁止は当該ファイルに限定して維持 — 新 ADR が補足）。

### `ci/tracker/tracker.mjs`（reconcile モード追加）

- `main()` の引数解析を `parseTrackerArgs(argv, env)` に分離して export する（contract test で mode 分岐を直接検証するため）。返り値は `{ok:true, args:{mode:"event"|"reconcile", event, repo, branch, workflow, workflowFile, label, report, summary}}` または `{ok:false, error}`。`main()` は `ok:false` を usage error（exit 2）に、`--help` を exit 0 に変換する（既存挙動と同じ）。
- `--reconcile` フラグ（boolean・値を取らない）を追加。`args.mode = "reconcile"`。
- mode 規則:
  - `mode:"reconcile"` のとき `args.event` は常に `null` — `TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH`/`--event` を **一切読まない**（mode 間の暗黙解釈を禁止）。明示的な `--event <file>` と `--reconcile` の同時指定だけは矛盾指定として usage error（`ok:false`）にする。env 由来の ambient path は無視（`GITHUB_EVENT_PATH` は schedule run でも常に存在するため、存在自体をエラーにしない）。
  - `mode:"event"` は従来どおり `TRACKER_EVENT_PATH ?? GITHUB_EVENT_PATH ?? --event` を必須とし、無ければ usage error。
- `run()` に `reconcile` オプションを追加する: `run({payload, client, scope, reconcile = false})`。`reconcile` 時は `classifyRun` を呼ばず `classified = {kind:"reconcile"}`（`run` なし）とし、それ以外は既存の `classifyRun(payload, scope)`。`classified.run` が `null`/`undefined` であることは既存コードで既に `.filter(Boolean)` / `toCand` の null 返却で吸収される — evidence 構築・ordering・plan・apply は**一切変更しない**（fail-closed guard を共有するための意図的な同一路径）。mode 境界の contract として、(a) `reconcile:true` と非 null payload の同時指定は throw（programmatic な mode 混在を潰す）、(b) event モードで payload が JSON object 以外（null/スカラ/配列）に parse された場合は throw（破損 event path を正規 skip と区別する — reconcile が `payload:null` を正当化したことで event 側の曖昧さが増すため、同じ mode 境界で塞ぐ）。
- `run()` の result に `mode: "event"|"reconcile"` を載せ、`summaryLines` 先頭に `- mode:` 行を追加、report JSON にも `mode` field を追加する（`result.classified.kind === "reconcile"` は report/summary にそのまま乗る。report の追加 field は `schemaVersion: 1` 後方互換で schema 名 `ci-issue-tracker` は不変）。診断時にどちらの入口が動いたかを判別できるようにする（spec R8）。
- CLI usage 文・header コメントに `--reconcile`（event 非依存入口。別 workflow `ci-issue-tracker-reconcile.yml` から呼ばれる）を追記する。

### 変更しない部分（境界）

- `plan()`/`applyActions()`/`classifyRun`/`classifyHistoryRecord`/`mergeRunRecords`/`pickNewest`/`compareRunOrder`/marker/sanitize/`makeGhClient` は一切変更しない。
- `ci-issue-tracker.yml`・`ci.yml`・`daily-ci.yml`・`consumer-verify.yml`・`ci/daily/`・`ci/summary/` は無変更。
- `mise.toml`: `test-ci-tracker` は既存のまま（reconcile test は同じ `ci/tracker/` に置くため同タスクで網羅される — 新規 task は不要）。

## Data and Control Flow

- reconcile 実行: schedule/dispatch 発火 → `reconcile` job（tracker と同一 concurrency group で直列）→ `test-ci-tracker` 自己検証 → `tracker.mjs --reconcile` → `parseTrackerArgs` → `run({reconcile:true, client, scope})` → `ensureLabel` → `listOpenIssues` → managed marker 走査・merge → `getRunHistory`（権威一覧 — event が無くても必須証拠）→ evidence 構築（marker refs + history + unknown。event 候補なし）→ `getBranchHead` → `compareCommits`/`getRunJobs`（必要時のみ）→ `plan` → `apply` → Job Summary + report artifact。
- event 実行との差分は `classified` が `{kind:"reconcile"}` で event 候補が無い点のみ — evidence の残りは全て権威履歴と marker で構成されるため、bot dispatch の成功・失敗はどちらも通常どおり拾われる。
- `newest` が `null`（履歴空 + marker なし）→ `noop: no-evidence`。`canonical == null` + 未解決失敗 → `create`。`canonical` あり + 権威履歴の最新成功が coverage を満たす → `close-recovered`（#375 シナリオと同じ plan 経路）。
- summary/report に `mode` が出るため、診断時に event 入口と reconcile 入口を区別できる。

## Non-Functional Constraints

- default-branch trusted code のみ: schedule は default branch でのみ走り、dispatch 時も checkout を default branch ref に pin する。reconcile 実行は失敗 run のコード・artifact・PR head を一切触れない。
- `TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH` は reconcile モードで参照しない — event 入口と reconcile 入口で event path の解釈を共有しない。明示 `--event` + `--reconcile` は usage error。
- 新 credential/secret/権限増量なし — `secrets.GITHUB_TOKEN` の既存最小権限のみ。
- fail-closed を維持: 履歴 API 失敗・非配列レスポンス・close 候補での branch head 不在は job 失敗。pending/unknown evidence は close と stale-failure 抑止を止める。API 失敗は伝播する。
- schedule auto-disable（public repo 60 日無活動・workflow 単位 disable・disabled は全 trigger 不応答）を event 入口から分離するため、cron は reconcile 専用ファイルに閉じ込める。`ci-issue-tracker.yml` には `schedule` を置かない。
- 有限 rerun seed（30 日/50 回）に依存しない — reconcile は履歴 API の pull 型であり rerun を前提としない。

## Risks and Trade-offs

- **reconcile 頻度**: 6 時間間隔は bot dispatch 完了から Issue 反映までの最悪遅延を ~6h に抑える。`23 */6` の tick は 00:23/06:23/12:23/18:23 UTC — daily scheduler が 21:17 UTC に dispatch し Full CI が ~1-2h で終わる想定では、次回 00:23 UTC の tick で回復 close に至る（典型的には完了後 ~1-3h、最悪 1 周期）。頻度は運用パラメータであり、必要なら後で調整する — Issue 管理の即時性要求は分〜時間であり、cron 最小粒度・高頻度の混雑コストより 6h を採る。
- **concurrency group 共有の pending 置換**: reconcile と event 入口が同一 group のため、片方 pending の間に他方が来ると pending が置き換わる。reconcile は冪等で最新履歴を見るため、置き換わっても失われる情報は無い（どちらの run も同じ権威履歴を見る）。
- **`--reconcile` + env event path の無視**: ambient `GITHUB_EVENT_PATH`（runner が常に設定）は無視するが、明示 `--event` は矛盾としてエラーにする非対称は意図的 — 「ambient 環境変数の存在」は呼出者の意図を表さず、「明示 flag」は表すため。
- **`workflow_dispatch` の非 default ref 起動**: `if: github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch` で skip するが、workflow file 自体は選択 ref のものが使われる（GitHub 仕様）— これは `daily-ci.yml` を含む全 workflow に共通の制約であり、reconcile で新たに増えるリスクではない。guard+pin は「未改変のこのファイルが常に default branch の `tracker.mjs` を実行する」ことを保証するものであり、workflow file 自体を編集できる write 権限者からは守れない（daily-ci.yml と同一の露出）。schedule payload で `github.event.repository` が欠落し得るため、ref 比較は dispatch 起動に限定する（schedule は定義上 default branch でのみ走る）。
- **event 候補を持たない残余リスク**: event モードは `toCand(classified.run, "event")` で「trigger run が履歴一覧に未反映」でも自分自身を証拠に含められるが、reconcile にはその anchor が無い。直前に完了した失敗 run が runs 一覧へまだ現れていない（eventual consistency）tick では、より古い成功が newest covering となり理論上は premature close が起き得る — 次回 tick または event で新しい失敗が再集計され新 episode として再発行されるため喪失は無いが、一時的な close→再 open の揺らぎはあり得る。pull モデル固有の制約として受け入れる（event 入口も非 trigger run には同じ露出を持つ）。
- **reconcile run 自体の失敗通知**: event 入口と同じく通知先は無い。Actions UI の `ci-issue-tracker-reconcile` run + report artifact が診断経路。60 日無活動 disable 後の復帰は手動（README の運用節に記載）。
- **reconcile が event 入口より先に失敗を起票する場合**: 順序上は先に書いた方が canonical になる — plan の fail-closed 契約は共通なので、どちらの入口が先に書いても同じ結論に収束する（duplicate は merge される）。

## Deferred Design Concerns

- reconcile 頻度・閾値の運用調整 | 6h は初期値 | 実運用観測後に変更判断 | cron を変えるだけの変更
- schedule auto-disable の検知・再有効化 | README 運用節に手順を残すのみ | 外部監視は後続判断（#367 と同じ制約） | 本設計は blast radius 分離まで
- `GITHUB_EVENT_PATH` が reconcile で ambient に存在することを summary に残すか | mode 記録で十分と判断 | 必要なら report に追記 | 変更不要

## Verification Notes

- [ ] `mise run test-ci-tracker`（`node --test ci/tracker/`）: reconcile 契約 test（bot 成功履歴 + open Issue → `close-recovered` / bot 失敗履歴 + Issue 無し → `create` / 成功のみ → `noop` / pending・unknown で close しない / 履歴 API 失敗は伝播）+ `parseTrackerArgs` の mode 分岐（`--reconcile`/`--event` 同時指定は error・reconcile で event path 非参照）を含む。
- [ ] `mise exec -- actionlint .github/workflows/ci-issue-tracker-reconcile.yml` pass。
- [ ] `mise exec -- prek run --all-files` pass。
- [ ] `git diff` で `ci-issue-tracker.yml`/`ci.yml`/`daily-ci.yml`/`consumer-verify.yml` 無変更。
