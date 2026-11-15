---
id: "DISC-034YaXLgv895taUPDbx0B6"
type: "discovery"
status: "resolved"
title: "Issue #631 GPUI Test Lab / Scenario Console GUI v1 + mise lane 簿記 — 調査記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
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
  related:
    - "docs/specs/issue-631-scenario-console.md"
    - "docs/designs/issue-631-scenario-console.md"
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

# Issue #631 GPUI Test Lab / Scenario Console GUI v1 + mise lane 簿記 — 調査記録

## Summary

Issue #631 後続 slice（GPUI Test Lab / Scenario Console GUI v1 +
mise lane 簿記）の実装前調査。catalog 消費・GUI 構成・threading・
fail-closed・boundary の 5 論点を既存コードで確認し、lab app を
`ruviform-plugin-dock-test` の `lab-gui` feature + `plugin-dock-lab`
bin に置く設計を確定した（confidence: high — 全て merge 済み API
の実読に基づく）。

## Exploration Goal

- GUI app の置き場所（plugin-dock-test bin / ui-gpui bin / ui-test
  bin / 新規 crate）を boundary 契約と依存方向から確定する。
- catalog 投影に API 拡張が必要かを `ScenarioMatrix`/`real_catalog`
  の実装から判定する。
- GPUI の `!Send`/event-loop 制約下で scenario 実行をどう
  thread 分離するかを `RealOsGuiTest`/`windowed_test_app` の先例から
  確定する。
- headless/非対話 host での fail-closed（`environment_blocked` →
  `NotRun`・exit 0）の実現方法を確定する。
- mise lane 3 変更（console 新設・lab 新設・real description 修正）
  の記述規約を既存 task 文体から確定する。

## Key Issues and Assumptions

- `boundary.json` は `ruviform-plugin-dock-test` を
  `composition-root` 登録済み（real-window-dock 用途を理由に明記）
  — lab app を同 crate に置けば新規登録・`ask` は不要（confirmed:
  fixtures/driver-lab/boundary.json 実読）。
- `ruviform-ui-gpui` の `GpuiDriver`/`ViewMount`/`COMMAND_INTENT_OP`
  は `gpui` feature export。`set_ui_jobs`/`UiJob` は
  `real-scenario`/`uitest` 専用で `lab-gui` では使えない — lab は
  `GpuiDriver::run` の `on_event` を直接駆動する（confirmed:
  lib.rs・real_window.rs 実読）。
- `finalize`/`case_environment`/`not_run` は `pub(crate)` — bin
  crate から見えない。blocked report は bin 内で同形 JSON を構成
  する必要がある（confirmed: scenario.rs L270-313）。
- `run_selected(env, keep)` は `pub` で bin から呼べる。
  `LaneEnvironment` は `Clone`（confirmed: scenario.rs L69-126）。
- `plugin-dock-console` の selector/exit-code 規約（0=Fail なし・
  1=Fail あり・2=CLI misuse）は canonical 化の手本（confirmed:
  plugin-dock-console.rs 実読）。
- `mise.toml` の `test-ui-plugin-dock-real` description は
  `test_gain` 前提の stale 文言 — `real_dock.rs` は
  `test_editor` descriptor を選択済み（confirmed: Cargo.toml・
  real_dock.rs・verification.md L371-372 指定済み）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) plugin-dock-test `lab-gui` feature + bin | composition-root 登録済み・boundary.json 不変・plugin+UI 両方を見られる・`real-dock` feature 先例と同型 | bin が大きくなる（~700 行見込み） | yes |
| (b) ui-gpui bin/example | GPUI driver に近い | catalog/plugin 層に依存できず composition-root 違反・plugin-dock-test への逆依存が必要 | no |
| (c) ui-test bin | driver-neutral 層 | GPUI 依存を ui-test に入れると driver-neutrality 違反 | no |
| (d) 新規 test-only crate | 責務分離 | workspace member 登録にルート Cargo.toml（lease 外）が必要・`ask` が要る・既存登録済み root で足りる | no |

## Tentative Conclusions and Hypotheses

- lab app = `ruviform-plugin-dock-test` の `lab-gui` feature +
  `plugin-dock-lab` bin（confirmed — boundary・依存・先例の全てが
  一致）。
- catalog projection API 拡張は不要（confirmed — `project()`/
  `cells()`/`real_catalog` で v1 要件を満たす。richer projection は
  Deferred Design Concerns へ）。
- GUI 構成は単一 Panel + Label/Table/Button のみ（Tabs 不使用 —
  v1 要件充足・KISS）。
- threading は app thread（`GpuiDriver::run`）+ scenario worker
  thread（`run_selected`）+ `mpsc` + Tick drain。
- blocked path は Ready timeout → 全 catalog row `NotRun` +
  `environment_blocked` → `render_jsonl` → exit 0。

## Open Questions

- `lab-gui` feature 名の最終確認 — `real-dock`/`gpui-test`/
  `uitest` と衝突しないことを Cargo.toml で確認済み。
- interactive 実走の可否は worker 環境依存 — headless なら
  `environment_blocked` を実測記録。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-631-test-lab.md`（作成済み）
- [ ] adr-doc needed: なし（composition-root 先例の適用で新規 ADR
  不要 — boundary 契約の変更なし）

## Sources and Relations

- Issue `scarecrow173/ruviform#631`。
- `docs/specs/issue-631-scenario-console.md`、
  `docs/designs/issue-631-scenario-console.md`（前 slice）。
- `docs/designs/issue-464-scenario-runner.md`（runner 制約）。
- `crates/ruviform-ui-test/fixtures/driver-lab/boundary.json`。
- `crates/ruviform-plugin-dock-test/src/{lib,scenario,real_catalog,
  real_dock,jsonl}.rs`、`src/bin/plugin-dock-console.rs`。
- `crates/ruviform-ui-gpui/src/{lib.rs,gpui_driver/real_window.rs,
  bin/windowed_test_app.rs}`。
- `crates/ruviform-ui/src/{driver.rs,view.rs,ipc.rs,behavior.rs}`。
- `mise.toml` #631 lane 節。

## Evidence

- 2026-11-15: `boundary.json` 実読 — `ruviform-plugin-dock-test` が
  `composition-root`、reason に Issue #631 real-window dock
  composition と明記。新規登録不要を確認。
- 2026-11-15: `lib.rs`（ui-gpui）実読 — `GpuiDriver`/`ViewMount`/
  `IntentSink`/`create_view_window`/`update_view`/`run` の signature
  と `gpui` feature cfg を確認。`set_ui_jobs` は
  `real-scenario`/`uitest` cfg で `lab-gui` 対象外。
- 2026-11-15: `driver.rs`/`view.rs`/`ipc.rs`/`behavior.rs` 実読 —
  `WindowSpec`/`WindowLifecycle`/`DriverEvent`/`WindowDriver`/
  `PanelSpec`/`TabsSpec`/`TableSpec`/`ButtonSpec`/`LabelSpec`/
  `CommandIntent::Action`/`CommandReceipt::from_result`/
  `deliver_event`/`drain_events` の契約を確認。
- 2026-11-15: `render.rs` L726-768 実読 — view intent は
  `Intent::new(command_id, "rvf.ui.command.v1", {command_id,
  expect_receipt, intent})` として Action channel に流れ、
  receipt は `post_event` の `Event::Result` で `data.status` が
  `applied`/`accepted`/`rejected` を運ぶことを確認。
- 2026-11-15: `scenario.rs`/`real_catalog.rs`/`jsonl.rs`/
  `plugin-dock-console.rs` 実読 — `run_selected`/`LaneEnvironment`/
  `REAL_CATALOG`/`render_jsonl`/`repo_sha` の pub 面と console の
  selector/exit-code 規約を確認。`finalize` 系は `pub(crate)` で
  bin 不可視。
- 2026-11-15: `real_dock.rs` 実読 — `RealOsGuiTest::run` の
  app-thread + ready-channel + join-on-every-path 構成、
  `blocked_reports` の `environment_blocked`/`NotRun` 正直形、
  `test_editor` descriptor 選択を確認。
- 2026-11-15: `Cargo.toml`（両 crate）実読 — `real-dock` feature
  構成・optional dep・`required-features` bin 先例を確認。
  `lab-gui` = `dep:ruviform-ui-gpui` + `ruviform-ui-gpui/gpui` で
  新規 dependency なし。
