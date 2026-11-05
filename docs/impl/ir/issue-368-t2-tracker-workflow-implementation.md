---
id: "IMPL-034Sr2JnjfRbqjfTRtkvRZ"
type: "impl"
status: "completed"
title: "Issue 368 T2 tracker workflow implementation"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "docs/plans/issue-368-ci-issue-tracker.md"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "file"
        path: "ci/tracker/README.md"
    modified:
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-368-t2-tracker-workflow.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
    - "docs/impl/ir/issue-368-t1-tracker-evaluator-implementation.md"
  refines: []
  refined-by: []
  derives-from: []
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

# Issue 368 T2 tracker workflow implementation

## Summary

`.github/workflows/ci-issue-tracker.yml` — `ci` workflow の完了を監視する
独立 workflow を新設。`workflow_run {workflows:[ci], types:[completed],
branches:[main]}` のみで起動し、`track` job に最小権限
（contents:read / actions:read / issues:write）を限定付与、repo 単位
concurrency（cancel-in-progress:false）で直列化する。`ci/tracker/README.md`
に運用契約・再利用 contract を、`docs/development-environment.md` に
`test-ci-tracker` タスクと workflow の所在を追記した。

## Context

`docs/tasks/issue-368-t2-tracker-workflow.md`（T1 の evaluator に依存）。
Issue #365 の「失敗の見逃し防止・Issue 管理を別 workflow の監視層として
実装する」方針に従い、Full CI（ci.yml）本体には一切変更を加えない構成。

## Implementation

### Changes

- `.github/workflows/ci-issue-tracker.yml`
  - trigger: `workflow_run` のみ（pull_request / push / schedule /
    workflow_dispatch trigger は付けない）。
  - `concurrency: group: ci-issue-tracker-${{ github.repository }},
    cancel-in-progress: false` — 同時完了をキャンセルせず直列処理。
    **post-review hardening**: pending 枠 1 件・新 pending が既存 pending
    を置き換える限界をコメントに明記 — 直列化だけでは全通知の順序処理を
    保証できないため、evaluator 側の run-history reconciliation で補う
    設計を workflow コメントと README に同期。
  - workflow レベル `permissions: {contents: read}` を fail-closed な
    baseline として置き、`track` job だけに
    `{contents: read, actions: read, issues: write}` を限定付与
    （job 単位の上書き）。
  - step: actions/checkout（default branch の tracker コード取得 —
    失敗 run 側のコード/artifact は checkout も download もしない）、
    `mise run test-ci-tracker` 自己検証を GitHub API 呼出し前に実行、
    `TRACKER_EVENT_PATH=$GITHUB_EVENT_PATH` で `tracker.mjs` を起動し
    report JSON + Job Summary を出力。
- `ci/tracker/README.md` — 運用正本: 有効 run scope、marker/Issue 同定、
  dedup・全 marker merge、evidence reconciliation（pending 置き換え・
  通知逆転・eventual consistency の限界）、atomic tuple ordering（同 id
  → attempt・異 id → updated_at・ambiguous → close 不使用）、自動
  close の多重 guard（最新成功・曖昧なく新しい・失敗 coverage・現在
  main tip coverage）、cancelled/unknown/API 失敗の扱い、権限不足・GH
  障害・Actions 自体の障害・manual close・diverged の手動対応、
  Consumer（AudioLens/Synvelis）への adoption contract。
- `docs/development-environment.md` — `test-ci-tracker` タスクと
  ci-issue-tracker workflow を検証入口表に追記。

### Decisions During Implementation

- **checkout するのは default branch の自分自身**: `workflow_run` は
  default branch の workflow ファイルで実行されるため、checkout しても
  失敗 run 側のコードには触れない。tracker が使う入力は event payload と
  REST メタデータのみ — artifact を実行入力にしない。
- **self-test を API 呼出し前に置く**: `test-ci-tracker` が失敗したら
  GitHub API に一切触れず job が落ちる順序にし、判定器の regression を
  Issue 破壊前に止める。
- **権限は track job のみ**: workflow 内の他に job が無い構成だが、将来
  job を追加しても権限が波及しないよう job レベル permissions に限定。
- **actions/checkout の version**: 既存 ci.yml が使う major と整合
  （repo 標準に追従）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml` → OK。
- `mise exec -- actionlint .github/workflows/ci.yml` → OK（ci.yml 無変更を
  `git diff 7454d84..HEAD` で確認）。
- `mise exec -- prek run --all-files` → 全 hook pass。
- `git diff --check` → clean。
- workflow の実起動（workflow_run 発火）は未検証 — GitHub Actions 実行が
  本タスクのスコープ外かつ禁止事項のため、actionlint + contract test で
  担保（README/design に未検証として記録）。

## Risks

- `workflow_run` は default branch に merge されるまで起動しない — 本
  branch 上の新 workflow は PR merge 後に初めて有効化される。
- workflow レベルの baseline `contents: read` に加え job 権限で絞って
  あるため permission 波及の余地は小さいが、Consumer 展開時は repo 側の
  Actions permission 設定（GITHUB_TOKEN default）も合わせて確認するよう
  README の permission 節に記載済み。

## Follow-ups

- merge 後の初回 workflow_run 実測（Issue #368 の受入検証）。
- Issue #367 daily scheduler 着地後、`workflow_dispatch` 起動 run が
  tracker の allowed event に一致することの統合確認。
- AudioLens #159 / Synvelis #167 への横展開（README adoption contract
  参照）。
