---
id: "PLAN-034TaI5Vr6iNYJxjNosslA"
type: "plan"
status: "completed"
title: "Issue #390 Driver Contract/Lab 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-390-driver-contract-lab.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-390-driver-contract-lab.md"
    - "docs/designs/issue-390-driver-contract-lab.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #390 Driver Contract/Lab 実装計画 Implementation Plan

## Goal

Issue #390 の Driver Boundary Guard・DriverContractFixture・Driver Lab
registry を実装し、driver-neutral 共通契約の GPUI/Toolkit 非依存を
機械検査可能にする。

## Scope

単一実装 stream。`ruviform-ui-test` crate 内の guard/lab/テスト/
fixture/manifest と `mise.toml` task、docs 更新を含む。Consumer 側
fixture・GPUI 実描画・実 OS・Native Dock・Visual Golden は範囲外
（owner 付き not_run/blocked として registry に記録）。

## Upstream Documents

- docs/specs/issue-390-driver-contract-lab.md
- docs/discovery/issue-390-driver-contract-lab-inventory.md

## Design Inputs

- docs/designs/issue-390-driver-contract-lab.md（approved）

## File Map

- `crates/ruviform-ui-test/src/driver_boundary.rs` — ruleset 型 +
  dep-graph/allowlist 検査。
- `crates/ruviform-ui-test/src/driver_boundary/sig.rs` — syn 公開
  signature 漏出検査（exposure-aware 2-pass）。
- `crates/ruviform-ui-test/src/driver_lab.rs` — case/story registry
  型・検証・evidence emission。
- `crates/ruviform-ui-test/src/bin/gui_boundary_guard.rs` — guard CLI。
- `crates/ruviform-ui-test/tests/driver_boundary.rs` — 正負 fixture +
  live workspace 検査。
- `crates/ruviform-ui-test/tests/driver_contract.rs` — manifest 整合 +
  UI-DVR-01/05/07/08 実行 fixture。
- `crates/ruviform-ui-test/fixtures/driver-lab/` — boundary.json
  ruleset + cases/stories + 正負 mini-workspace fixtures。
- `mise.toml` — check-ui-driver-boundary / test-ui-driver-contract。
- `crates/ruviform-ui-test/README.md` / `docs/development-environment.md`
  — 入口・語彙・限界の文書化。

## Implementation Sequence

- [x] Cargo.toml 依存（cargo_metadata/syn/proc-macro2）+ ruleset schema。
- [x] driver_boundary dep-graph 検査（direct/feature/dep-activation/
  transitive/conditional/allowlist/adapter-consumer）。
- [x] sig.rs 公開 signature 走査（exposure・alias・impl header）。
- [x] gui-boundary-guard CLI + `[[bin]]` rename。
- [x] 正負 mini-workspace fixtures（7 負例 + 1 正例）。
- [x] driver_lab registry・validate・evidence。
- [x] cases.json/stories.json（UI-DVR-01..08 + 2 story）。
- [x] tests/driver_boundary.rs + tests/driver_contract.rs。
- [x] mise tasks + docs/env・crate README 更新。
- [ ] 独立 review の P0/P1 修正 + targeted re-verify（実施済み —
  conditional 走査の manifest-level 化・adapter exempt・dep.features
  有効化検査を含む）。

## Dependencies and Risks

- 公開契約未着（#385/#386/#387/#388）— blocked binding で記録、
  stub 代替しない。
- cargo metadata が未 activation optional dep を resolve から
  省くため conditional 走査は manifest-level 再帰で補完
  （registry target 解決不能は既知の限界）。
- 他 worktree との競合 — 所有範囲（ruviform-ui-test + mise/docs
  各行）に限定。

## Task Breakdown

- task doc: docs/tasks/ 下の issue-390 系 task を参照。

## Verification

- [x] `mise run check-ui-driver-boundary` → verdict: pass（live ws）。
- [x] `mise run test-ui-driver-contract` → 5 test pass。
- [x] `cargo test -p ruviform-ui-test` → 全 test pass（fixture 10
  + contract 5 + 既存 47）。
- [x] `cargo clippy -p ruviform-ui-test --all-targets -- -D warnings` 。
- [x] `mise run fmt-check` 相当の fmt 適用。

## Open Questions / Gaps

- `required: false` GPUI grant は RF386 merge まで休眠（
  `allowlist-missing` は strict で検出される設計）。
- evidence 行の manifest binding 照合は後続改善（design §限界）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker 本体内で完結）。
- Independent work streams: none。
- User approval required before dispatch: yes。
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix。

## Review Handoff

- 焦点: dep-graph traversal の cargo 意味論・signature exposure
  モデルの誤検出/取りこぼし・manifest binding の status 語彙正直性。
