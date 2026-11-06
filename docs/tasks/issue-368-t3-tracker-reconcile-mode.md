---
id: "TASK-6LaGohoePjG92Eb5u6lUv7"
type: "task"
status: "done"
title: "T3 tracker.mjs reconcile モード + parseTrackerArgs + contract tests（Issue #368）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ci/tracker/tracker.mjs"
      - type: "file"
        path: "ci/tracker/tests/tracker.test.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-368-tracker-reconcile.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-368-tracker-reconcile.md"
  blocks:
    - "docs/tasks/issue-368-t4-reconcile-workflow.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-368-t4-reconcile-workflow.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-368-tracker-reconcile.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T3 tracker.mjs reconcile モード + parseTrackerArgs + contract tests（Issue #368）

## Work

`ci/tracker/tracker.mjs` に event 非依存の `--reconcile` モードを追加する。`main()` の引数解析を `parseTrackerArgs(argv, env)` に分離して export し、`run({payload, client, scope, reconcile})` で `classified={kind:"reconcile"}` として既存パイプラインを共通実行する。`tests/tracker.test.mjs` に reconcile 契約 test と mode 分岐 test を追加する。**先に RED（reconcile test が fail）→ 実装で GREEN**。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-368-tracker-reconcile.md`
- `docs/designs/issue-368-tracker-reconcile.md`
- `docs/test-specs/issue-368-tracker-reconcile.md`

## Dependencies

- なし（先行タスクなし。T4 が本タスクの成果物に依存する）

## Affected Files

- `ci/tracker/tracker.mjs`（変更 — `parseTrackerArgs` 分離 export・`--reconcile` フラグ・`run({reconcile})`・`mode` 出力・usage/header 追記）
- `ci/tracker/tests/tracker.test.mjs`（変更 — reconcile 契約 test・`parseTrackerArgs` mode 分岐 test 追加）

## Done When

- `parseTrackerArgs` が export され、`--reconcile` → `mode:"reconcile"`/`event:null`（`TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH` があっても非参照）、`--reconcile --event` → `ok:false`、flag なしは従来どおり `TRACKER_EVENT_PATH ?? GITHUB_EVENT_PATH` 必須を返す。
- `run({payload:null, reconcile:true, client, scope})` が `classified={kind:"reconcile"}` で既存の ensureLabel→list→history→evidence→plan→apply 経路を実行し、event 候補が evidence に混入しない。
- reconcile で bot 成功履歴 + open 失敗 Issue → `close-recovered`、bot 失敗履歴 + Issue 無し → `create`、成功のみ → `noop`、pending/unknown/branch head 失敗/履歴 API 失敗で close しない・例外伝播する test が `node --test` で pass する。
- report JSON・Job Summary に `mode`（`event`/`reconcile`）が記録される。
- `mise run test-ci-tracker` が exit 0。
- `plan`/`applyActions`/`classifyRun`/`classifyHistoryRecord`/marker/sanitize/`makeGhClient` は無変更。
