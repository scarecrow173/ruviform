---
id: "TASK-034XWO3lADUXxGJf44pNX3"
type: "task"
status: "in-progress"
title: "Issue #631 T1 — Headless Scenario Console（plugin-dock-console bin）"
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
  implements:
    - "docs/plans/issue-631-scenario-console.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-scenario-console.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-631-scenario-console.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 T1 — Headless Scenario Console（plugin-dock-console bin）

## Work

merge 済み scenario catalog の headless consumer として
`plugin-dock-console` bin を追加する。

1. `crates/ruviform-plugin-dock-test/src/real_catalog.rs` 新規 —
   `rf631.plugin-dock-real` の driver-neutral catalog metadata
   （`RUN_ID`/`SCENARIO`/`LANE`/`DRIVER`/`RealCatalogRow`/
   `REAL_CATALOG`）。`lib.rs` に `pub mod` + `run_selected`
   re-export を追加。
2. `src/scenario.rs` — `run_selected(env, keep)` を追加し、
   `run_all` をその select-all 投影に（挙動不変）。
3. `src/real_dock.rs` — `"real-window-dock"` literal を
   `real_catalog::LANE` に置換し、`REAL_CATALOG`↔`real_entries()`
   一致 test を追加。
4. `src/bin/plugin-dock-console.rs` 新規 — `--list`（project()
   の 1 行 1 JSON、verdict 語彙なし）/ `--include-real`（feature
   build のみ、`session_required:true`+`lane` 付き 4 行）/
   `--run`（selector filter + `run_selected` + `render_jsonl`、
   lane=`plugin-dock-console`）/ `--out`。fail-closed: real-lane
   selector → per-selector unknown → combination empty の順で
   exit 2。
5. `tests/console.rs` 新規 — list/run/selector/refusal を網羅
   （default + `--features real-dock`）。README 更新。
6. canonical 検証・`evidence/rf631/` 更新・doc chain 完成・
   index 再生成。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-scenario-console.md`
- `docs/specs/issue-631-scenario-console.md`
- `docs/designs/issue-631-scenario-console.md`

## Dependencies

- 前 slice merge 済: `issue-631-scenario-catalog`（catalog +
  projection、`origin/main` = `cade9764` 以降 `bff3603b`）。
- `mise.toml` は別 Worker 占有 — console lane task は追加しない。

## Affected Files

- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`（新規）
- `crates/ruviform-plugin-dock-test/src/{lib,scenario,real_dock}.rs`
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs`
  （新規）
- `crates/ruviform-plugin-dock-test/tests/console.rs`（新規）
- `crates/ruviform-plugin-dock-test/README.md`
- `evidence/rf631/plugin-dock-console.jsonl`・`verification.md`
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/
  issue-631-scenario-console.*`

## Done When

- [ ] `plugin-dock-console --list` が 8 headless row を verdict
  語彙なしで出力し、`--include-real`（feature build）が 4 real
  row を `session_required:true`/`lane:"real-window-dock"` 付きで
  追記、非 feature build では exit 2。
- [ ] `--run` の selector 絞込みが lane と同一 JSONL verdict を出し、
  unknown selector・一致 0 件・real-lane 指定が exit 2。
- [ ] `tests/console.rs` が上記を網羅し pass する。
- [ ] `test-ui-plugin-dock` の JSONL verdict が非回帰、
  `check-ui-driver-boundary` green。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check|lint|test ruviform-plugin-dock-test`
  （default + `--features real-dock`）
- [ ] `mise run test-ui-plugin-dock` / `test-ui-scenario-contract` /
  `check-ui-driver-boundary`
- [ ] `mise exec -- prek run --all-files`
