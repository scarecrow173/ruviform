---
id: "TASK-034ZkLmNpQrStUvWxYzIj5"
type: "task"
status: "completed"
title: "Issue #692 T1 — ruviform-script 中立契約 port crate 実装・検証・docs chain"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-692-script-port.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-692-script-port.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-692-script-port.md"
    - "docs/specs/issue-692-script-port.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-692-script-port.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-692-script-port.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-692-script-port.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #692 T1 — ruviform-script 中立契約 port crate 実装・検証・docs chain

## Work

`crates/ruviform-script` を新設し、RF-SCRIPT-00 中立契約の design
完全形（C1〜C6）+ canonical digest utils + codec を headless に
実装する。Issue #692 first slice のみ — 実 Luau backend/Analyzer・
synvelis 接続・依存選定は後続 slice / consumer gate。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-692-script-port.md`（approved）
- `docs/designs/issue-692-script-port.md`（approved）
- `docs/plans/issue-692-script-port.md`（approved）
- `docs/test-specs/issue-692-script-port.md`（approved）
- `docs/designs/issue-558-script-contract-design.md`（approved — 契約正本）
- `docs/discovery/issue-692-script-port.md`
- `POLICIES.md`

## Dependencies

- `poc/script-coexist/src/{contract,value,canonical,codec,analyzer}.rs`
  （#559 — 意味論 port 元、READ-ONLY）
- `ffi-poc/src/{contract,analyzer}.rs`（#560 — analyzer 契約 shape）
- synvelis `crates/synvelis-script/src/{backend,analyzer,canon}.rs`
  （cross-repo READ-ONLY — `analyze(&self)`・`canonical_json`/
  `digest_of` 先例）
- `crates/ruviform-ui-theme/Cargo.toml`（crate 雛形先例）
- serde_json `"1"`・sha2 `"0.10"`（Cargo.lock 既在）

## Affected Files

- `crates/ruviform-script/`（新設 — Cargo.toml/README/src/tests）
- `Cargo.toml`（root — `members` 追加のみ）
- `Cargo.lock`（再生成 — 新規クレート分のみ）
- `crates/README.md`（索引行追加 + クレート数更新）
- `docs/{discovery,specs,designs,plans,tasks,test-specs}/
  issue-692-script-port.md`（docs chain）
- `docs/impl/{ir,exp}/issue-692-script-port.md` +
  `issue-692-script-port-exp.jsonl`（impl 記録）
- `evidence/rf692/`（検証記録）
- `docs/*/README.md`（docs index — Coordinator が統合時に再生成）

## Done When

- [x] `crates/ruviform-script` が workspace member として build でき、
  `mise run check|lint|test ruviform-script`・`mise run fmt-check`・
  `mise exec -- prek run --all-files` が Pass する。
- [x] C1〜C6 の中立契約型一式が design 完全形で実装され、registry
  fail-closed 行列が型付きエラーで検証される。
- [x] `ScriptValue` 全 variant の codec 往復が等値で、canonical digest
  が同一意味 payload で一致・意味差で不一致になることが検証される。
- [x] `AnalyzerRegistry` の fail-closed resolve・fake analyzer の
  `analyze(&self)` 往復・`source_revision` 不一致の stale 検出が
  検証される。
- [x] `HostApiSurface` の中立性検証が OS/FS/Network/Shell 相当の
  binding を拒否することが検証される。
- [x] AC4（mlua vendored vs native bridge 選定）は s2 slice へ
  Deferred と design 文書・PR 本文に正直に記録されている。
- [x] Issue #692 の残受入（実 Luau backend/Analyzer = AC2、synvelis
  thin adapter 接続 = AC3）は本 slice 対象外として docs に明記され、
  実 Luau/実 OS/GUI 経路は NotRun/Deferred と記録されている。

## Verification

- [x] `mise run fmt-check` — Pass
- [x] `mise run check ruviform-script` — Pass
- [x] `mise run lint ruviform-script` — Pass
- [x] `mise run test ruviform-script` — Pass
- [x] `mise exec -- prek run --all-files` — Pass
- [x] macOS/Linux build・実 Luau VM/FFI・実 OS・GUI は NotRun/Deferred
