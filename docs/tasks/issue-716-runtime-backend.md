---
id: "TASK-Tl2K23IPFpdnJ46b7lJJAc"
type: "tasks"
status: "approved"
title: "Issue #716 runtime ScriptBackend — タスク分解"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/plans/issue-716-runtime-backend.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-716-runtime-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-716-runtime-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-716-runtime-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/plans/issue-716-runtime-backend.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
  defers: []
  deferred-by: []
---

# Issue #716 runtime ScriptBackend — タスク分解

## T1 — docs chain 作成

- [x] discovery / spec / design / plan / tasks / test-specs /
  adr（本ファイル群）

## T2 — runtime backend 実装

- [ ] `src/runtime.rs`: `LuauBackend`・`LuauSession`・
  `InterruptState`・lift・error 写像・sandbox 構成
- [ ] `src/lib.rs`: `mod runtime` + re-export + docs 更新
- [ ] `README.md` / `Cargo.toml` コメント更新
- [ ] `crates/README.md` luau 行更新

## T3 — deterministic OS-L1 test

- [ ] `tests/runtime.rs`: descriptor/open/evaluate/cancel/
  budget/artifact/lifecycle/fail-closed/registry resolve→eval
- [ ] test-specs の expected/observed を実測で埋める

## T4 — 検証

- [ ] `mise run check|lint|test ruviform-script-luau`
- [ ] `mise run test ruviform-script`（非回帰）
- [ ] `mise run fmt-check` + `prek run --all-files`

## T5 — evidence・impl 文書・PR

- [ ] `evidence/rf716/`（consumer gate 委譲明記）
- [ ] `docs/impl/ir|exp` + `-exp.jsonl`
- [ ] review-protocol → commits → push → PR → worker_done
