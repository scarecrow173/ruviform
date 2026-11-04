---
id: "TASK-8hTQjGqBYDGbC2jTMks1j8"
type: "task"
status: "done"
title: "T1 versioned fixture corpus + ui/guitest package 骨格（Issue #311）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "dir"
        path: "ui/guitest"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-311-ipc-conformance-replay.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-311-ipc-conformance-replay.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 versioned fixture corpus + ui/guitest 骨格

## Work

- `ui/guitest/` に private package を作成（package.json `typecheck`/`test` scripts、tsconfig、README）。
- `fixtures/README.md` に形式 v1・バージョン規則・denylist/allowlist 語彙を正本化。
- fixture を作成: `ipc/intents.v1.json`、`ipc/events.v1.json`、`ipc/data.v1.json`、`binding/messages.v1.json`、`scenarios/binding/*.v1.json`、`scenarios/host/*.v1.json`、（recordings fixture は見送り — recorder 往復は inline JSONL で検証）。
- `src/fixtureload.ts` — `v`/`suite`/case 構造の fail-closed 検査付きローダ。
- （`test/env.ts` 不作成 — node --test が .ts を直接実行するため hook 不要）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-311-ipc-conformance-replay.md`
- `docs/designs/issue-311-ipc-conformance-replay.md` §2

## Downstream / Verification

- `test/fixtures.test.ts` で全 fixture の構造健全性（T2 で実装・依存関係上このタスクに同梱可）。
- `mise run check-ui-guitest`・`test-ui-guitest` が通る（mise タスク追加は T4。本タスク時点は `npm --prefix ui/guitest run typecheck`/`test` 直接確認も不可 — 規約上 mise タスク追加を本タスク内に先行してよい）。
