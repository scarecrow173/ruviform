---
id: "TASK-auXqAVyXzCi31qldUWzY6t"
type: "task"
status: "done"
title: "T2 ci.yml Shadow wiring（impact/ci-gate job・trigger types・mise task・AGENTS.md）（Issue #333）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/333"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-333-impact-based-ci.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-333-t1-impact-classifier.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-333-impact-based-ci.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-333-impact-based-ci.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  task:
    lane: "implementation"
---

# T2 ci.yml Shadow wiring（Issue #333）

## Goal

`impact`/`ci-gate` job を Shadow Mode で `ci.yml` に追加し、trigger types・
mise task・AGENTS.md を同期する。既存 6 job・12 required contexts は無変更。

## Acceptance

- `actionlint` と `prek run --all-files` が pass。
- `impact` job が `node --test ci/impact/` → classify → GITHUB_OUTPUT/artifact
  を出力する構成になっている。
- `ci-gate` が 7 needs + `if: always()` で gate.mjs を実行する構成になって
  いる。
- PR の CI run で両 job が実際に走り summary/artifact が観測できる。

## 関連文書

- 実装計画: [docs/plans/issue-333-impact-based-ci.md](../plans/issue-333-impact-based-ci.md)
- 検証仕様: [docs/test-specs/issue-333-impact-based-ci.md](../test-specs/issue-333-impact-based-ci.md)
