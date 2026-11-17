---
id: "TASK-R7rbYG8Ew6lrrKHrYcKDbA"
type: "task"
status: "done"
title: "Issue #631 T1 — real-window-close/dpi catalog flip"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/plans/issue-631-real-window-catalog-flip.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-real-window-catalog-flip.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 T1 — real-window-close/dpi catalog flip

## Work

`rf631.plugin-dock-real` の `real-window-close`/`real-window-dpi`
2 row を `Unsupported` stub から実判定へ flip する slice。

1. `real_dock.rs` session 構成: window C（`ViewInstanceId` 633・
   `ViewHost`・`WindowSpec` position (2900,80)）を第 3 mount と
   して追加し、`RealDockContext` に `window_c: Option<WindowId>`
   と `session_scale_factor: Option<f64>`（session 開始時に
   `window_geometry(window A)` で 1 回実測）を追加。
2. `real_dock.rs` case 実装:
   - `close_body`: window B に armed `Veto`→armed `Defer`→
     `respond_close(Approve)`（`Destroyed` まで）、window C に
     未回答 `request_close`（既定 `Approve`→`Destroyed`）を駆動。
     `window_events`/`windows()` のみで観測し、sentinel A の
     非干渉・terminal 退役・`windows()==[A]` を assert。
   - `dpi_body`: `window_geometry(A)` の live 値と `window_events`
     の最新 `Resized`/`ScaleFactorChanged` を照合して判定。
     `window_geometry_sha256` digest・`dpi_change_driven: false`
     を observed に記録。
   - `finalize_case` wrapper で `session_scale_factor` を
     `environment.scale_factor` へ注入し、全 case の
     `finalize_real` 呼出を置換。
   - module doc の honest-outcome 表を flip 後の verdict 語彙へ
     更新（`real-window-ime` は `Unsupported` 維持）。
3. `real_catalog.rs` + `real_entries`: `real-window-close` の
   required を `MultiWindow` へ協調更新（`real-window-dpi` は
   `SemanticAction` 維持）+ doc コメント整合。
4. `jsonl.rs`: `case_line` に `receipts`（`CommandReceipt` は
   `Serialize`）と deterministic `case_seed`（sha2、既存依存）を
   追加 + unit test。
5. 検証 lane 実行 → `test-ui-plugin-dock-real` 実走で
   `plugin-dock-real-scenario.jsonl` 再生成、
   `test-ui-plugin-dock-real-headless` で
   `plugin-dock-real-headless.jsonl` 再生成、
   `verification.md` へ slice 記録追記。
6. doc chain（discovery/spec/design/plan/task/test-spec/ir/exp）
   を完成させる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-real-window-catalog-flip.md`
- `docs/specs/issue-631-real-window-catalog-flip.md`
- `docs/designs/issue-631-real-window-catalog-flip.md`
- `docs/discovery/issue-631-real-window-catalog-flip.md`
- `docs/test-specs/issue-631-real-window-catalog-flip.md`

## Dependencies

- `RealOsGuiTest` の `window_events`/`request_close`/
  `arm_close_response`/`respond_close`/`window_geometry`
  （window-surface slice で実装済み・変更なし）。
- `WindowLifecycleEvent`/`CloseDecision`/`OsWindowGeometry`
  （`ruviform-ui`/`ruviform-ui-gpui`、変更なし）。
- `settle_until`/`run_scenario`/`steps_outcome`/`step`/
  `finalize_real`/`not_run`/`fail` 既存 helper。
- `CommandReceipt: Serialize`・`sha2` 既存依存（AC4、lockfile
  不変）。

## Affected Files

- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`
- `crates/ruviform-plugin-dock-test/src/jsonl.rs`
- `evidence/rf631/plugin-dock-real-scenario.jsonl`（再生成）
- `evidence/rf631/plugin-dock-real-headless.jsonl`（再生成）
- `evidence/rf631/verification.md`（slice 記録追記）
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-631-real-window-catalog-flip.*`

## Done When

- [ ] `real-window-close` が 3 経路全てを実観測して判定し、
  sentinel/terminal 退役を assert する。
- [ ] `real-window-dpi` が live geometry と lifecycle 記録の整合を
  判定する。
- [ ] `REAL_CATALOG`/`real_entries` の required が一致し、
  catalog 整合 test が green。
- [ ] headless lane が全行 `NotRun`+`environment_blocked` を
  exit 0 で維持する。
- [ ] `case_line` が `receipts`/`case_seed` を出力し、real lane
  の `environment.scale_factor` が実測値を持つ。
- [ ] 非回帰 lane が green、`verification.md` に記録が追記される。

## Verification

- [ ] `mise run fmt-check` — pass
- [ ] `mise run check/lint/test ruviform-plugin-dock-test`
  （default/`real-dock`/`lab-gui`）— pass
- [ ] `mise run test-ui-plugin-dock-console` — pass
- [ ] `mise run test-ui-plugin-dock-real-headless` — pass（exit 0）
- [ ] `mise run test-ui-plugin-dock-real` — 実走、flip 後 verdict
  を JSONL に記録（`real-dcomp-pixel` 既知 Fail による exit 1 は
  設計どおり）
- [ ] `mise run test-ui-scenario-contract` — pass
- [ ] `mise run check-ui-driver-boundary` — pass

## Notes

- `real-window-ime` は `Unsupported` のまま（`Ime` capability 未
  claim・#389 領域）。
- DPI 変更・pixel Pass・consumer 投影・IME/verb は後続 slice/gate
  として `Unsupported`/`Fail` を正直に保持する。
- docs index（`docs/**/README.md`）は Coordinator が統合時に
  再生成 — 本 slice では編集しない。
- PR 本文は `Refs #631` のみ（auto-close 禁止）。
