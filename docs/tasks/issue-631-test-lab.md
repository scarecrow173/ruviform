---
id: "TASK-034YaXXtkSHtOycwJibVwz"
type: "task"
status: "in-progress"
title: "Issue #631 GPUI Test Lab / Scenario Console GUI v1 + mise lane 簿記"
created: "2026-11-15"
updated: "2026-11-15"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-631-test-lab.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 GPUI Test Lab / Scenario Console GUI v1 + mise lane 簿記

## Work

`ruviform-plugin-dock-test` に `lab-gui` feature と `plugin-dock-lab`
bin を追加し、merge 済 catalog（`HEADLESS_MATRIX` +
`real_catalog::REAL_CATALOG`）を ruviform-ui ViewSpec vocabulary +
`GpuiDriver` の実 Window で dogfood する GUI v1 を実装する。
catalog 一覧（capability projection・verdict なし）・headless
selector 実行（`run_selected` 経由）・verdict/evidence 表示・
`--out` JSONL（blocked は `environment_blocked`/`NotRun` fail-closed、
exit 0）を持つ。あわせて `mise.toml` に
`test-ui-plugin-dock-console`・`test-ui-plugin-dock-lab` を新設し、
`test-ui-plugin-dock-real` の stale description を `test_editor`
実態へ修正する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-test-lab.md`
- `docs/designs/issue-631-test-lab.md`
- `docs/specs/issue-631-test-lab.md`
- `docs/test-specs/issue-631-test-lab.md`

## Dependencies

- merge 済み: `HEADLESS_MATRIX`/`REAL_CATALOG`・`ScenarioMatrix::
  {cells,project}`・`run_selected`・`plugin-dock-console`・
  `RealOsGuiTest`・`test_editor` fixture（PR `#670`）。

## Affected Files

- `crates/ruviform-plugin-dock-test/Cargo.toml`
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-lab.rs`（新規）
- `mise.toml`（#631 lane 節のみ）
- `docs/development-environment.md`
- `evidence/rf631/verification.md`
- `docs/impl/ir|exp/`（impl 記録）

## Done When

- [x] `plugin-dock-lab` が `lab-gui` feature で build され、
      ViewSpec/Table/Button のみで catalog・実行・verdict/evidence
      を表示する windowed app として動作する。
- [x] headless/非対話 host で `environment_blocked` → 全 catalog
      row `NotRun` JSONL + exit 0（fail-closed）。— 実装・unit
      test 済み。実 headless host での lane 実走は未検証
      （verification.md 参照）。
- [x] real catalog row は表示のみで実行しない（`test-ui-plugin-dock-real`
      へ委譲を status 表示）。— GUI 実走で JSONL 非増加を確認。
- [x] `test-ui-plugin-dock-console`/`test-ui-plugin-dock-lab` task が
      description どおり動作し、`test-ui-plugin-dock-real`
      description が `test_editor` 実態と一致する。
- [x] headless lane verdict 非回帰・`check-ui-driver-boundary` green。

## Verification

- [x] `mise run fmt-check`（`cargo fmt --all -- --check` exit 0）
- [x] `mise run check|lint|test ruviform-plugin-dock-test`（default /
      `real-dock` / `lab-gui`）— check/test は exit 0。canonical
      `mise run lint`（deps 込み clippy）は path dep
      `ruviform-audio-test` の既存 `too_many_lines`
      （stream.rs:885・本 slice 無関係の upstream drift）で
      exit 101 — 本 crate 自体は `--no-deps` scoped clippy で
      `-D warnings` clean。scope 外のため未対処・PR で報告。
- [x] `mise run test-ui-plugin-dock` / `test-ui-plugin-dock-console` /
      `test-ui-plugin-dock-lab` / `test-ui-scenario-contract` /
      `check-ui-driver-boundary`
- [x] `mise exec -- prek run --all-files`
