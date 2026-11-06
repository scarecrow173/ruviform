---
id: "TSPEC-qvpqeonK5xwqk0hZpF3ARO"
type: "test-spec"
status: "approved"
title: "Issue #368 Tracker reconcile test specification — mode 分離・bot 履歴 reconcile・fail-closed 維持"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker-reconcile.yml"
    modified:
      - type: "file"
        path: "ci/tracker/tracker.mjs"
      - type: "file"
        path: "ci/tracker/tests/tracker.test.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-368-tracker-reconcile.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-368-tracker-reconcile.md"
    - "docs/plans/issue-368-tracker-reconcile.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #368 Tracker reconcile test specification — mode 分離・bot 履歴 reconcile・fail-closed 維持

既存 `ci/tracker/tests/tracker.test.mjs` の contract test 群に追加する検証仕様。全て mock client のみ・外部通信なし。`run({payload:null, reconcile:true, client, scope})` と `parseTrackerArgs` を対象に、本体テストが pin した fail-closed 契約を reconcile 経路でも保つことを確認する。

## Feature: reconcile モードの証拠経路（`run({reconcile:true})`）

### Rule: event payload が無くても権威履歴と marker で同一判定を行う

- Example: open 失敗 Issue（marker: failure run 100@T1 sha A）+ 履歴に新しい成功 run 200@T2 sha B・`compare(A→B)=ahead`・`compare(mainHead→B)=identical` → `close-recovered`（bot 成功の通知欠落を回復する #375 相当シナリオ）。
- Example: open Issue なし + 履歴に未解決の失敗 run 300@T3 sha C（成功で未カバー）+ jobs あり → `create`（bot 失敗の起票漏れを補完する）。
- Example: open Issue なし + 履歴が成功のみ → `noop`（`success:no-unresolved-failure`）。
- Example: `classified.kind === "reconcile"` で result/report に反映され、event 候補が evidence に混入しない。

### Rule: fail-closed guard は reconcile でも緩めない

- Example: 履歴に live `pending` run が残る → `close` しない（update/noop のみ）。
- Example: 履歴に識別不能 `unknown` record が残る → `close` も resolved-suppression もしない。
- Example: close 候補で `getBranchHead` が失敗 → job 失敗（propagate）・close しない。
- Example: `getRunHistory` が非配列/失敗 → 例外が伝播し `run()` が reject する。
- Example: 記録最新失敗より古い成功（`compare=behind`・順序 ambiguous）は `close` しない。

### Rule: marker/冪等契約は event 入口と同一

- Example: managed open Issue が複数 → 最小番号 canonical に merge して duplicate close。
- Example: label・marker の無い外部 Issue は触れない。
- Example: 既存失敗より新しい失敗履歴 → `update`（`create` ではない）。

## Feature: mode 分離（`parseTrackerArgs`）

### Rule: `--reconcile` は event path を一切読まない

- Example: `--reconcile`（env `TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH` ありでも）→ `mode:"reconcile"`・`event:null` — ambient path は無視する。
- Example: `--reconcile --event x.json` → usage error（明示指定の矛盾 — `ok:false`）。
- Example: flag なし + `TRACKER_EVENT_PATH` → `mode:"event"`・event path を引く（従来挙動）。
- Example: flag なし + event path 無し → usage error（`ok:false`・従来挙動）。

## Feature: report/summary の mode 識別

### Rule: 入口 mode を診断できる

- Example: reconcile 実行の report JSON に `mode:"reconcile"`・`classified.kind:"reconcile"` が乗る。
- Example: event 実行の report/summary は `mode:"event"`（後方互換の追加 field）。
