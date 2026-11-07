---
id: "TASK-034TaJ4jQd92g4mWa81cxX"
type: "task"
status: "todo"
title: "Issue #390 Driver Contract/Lab 実装"
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
    - "docs/plans/issue-390-driver-contract-lab.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-390-driver-contract-lab.md"
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
    - "docs/test-specs/issue-390-driver-boundary-lab.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #390 Driver Contract/Lab 実装

## Work

Driver Boundary Guard（ruleset + dep-graph + signature 検査 +
CLI）、Driver Lab registry（cases/stories manifest + 検証 +
evidence）、UI-DVR-01〜08 registry + UI-DVR-01/05/07/08 実行
fixture、正負 mini-workspace fixtures、canonical mise task の実装。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/issue-390-driver-contract-lab.md
- docs/designs/issue-390-driver-contract-lab.md
- docs/specs/issue-390-driver-contract-lab.md

## Dependencies

- 公開契約未着（#385/#386/#387/#388）は blocked binding で記録 —
  この task 自体はそれらに依存しない（stub 代替なし）。

## Affected Files

- crates/ruviform-ui-test/{Cargo.toml,src/lib.rs,
  src/driver_boundary.rs,src/driver_boundary/sig.rs,
  src/driver_lab.rs,src/bin/gui_boundary_guard.rs,
  tests/driver_boundary.rs,tests/driver_contract.rs,
  fixtures/driver-lab/**,README.md}
- mise.toml（check-ui-driver-boundary / test-ui-driver-contract）
- docs/development-environment.md（task 表 2 行）
- docs/{specs,designs,plans,test-specs,tasks}/issue-390-*
- docs/discovery/issue-390-driver-contract-lab-inventory.md

## Done When

- [x] `mise run check-ui-driver-boundary` が live workspace で
  exit 0 / verdict pass。
- [x] `mise run test-ui-driver-contract` が manifest 整合 + 実行
  fixture 5 件 pass。
- [x] `cargo test -p ruviform-ui-test` 全件 pass・
  `cargo clippy -p ruviform-ui-test --all-targets -- -D warnings`
  クリア。
- [x] 7 negative + 1 positive fixture が期待 verdict で pin。
- [x] 判定語彙・layer・requiredCases が manifest で宣言・検証済み。

## Verification

- [x] `mise run check-ui-driver-boundary` → verdict: pass。
- [x] `mise run test-ui-driver-contract` → 5 tests ok。
- [x] `cargo test -p ruviform-ui-test` → 62 tests ok。
- [x] `cargo clippy -p ruviform-ui-test --all-targets -- -D warnings`
  → clean。
- [x] `RUVIFORM_DRIVER_LAB_EVIDENCE=target/driver-lab/evidence.jsonl`
  で 4 実行 case の evidence emission 確認。

## Phase 2（Issue #390 DoD 残件の実装・stale status 修正）

- [x] UI-DVR-04（ResolvedTheme mode/layer 優先順位・宣言外 mode 拒否）と
  UI-DVR-06（NativeSurfaceSlot Unsupported 報告・destroy/revoke・
  recycle 後 StaleGeneration）の fake 実行 fixture を追加。どちらも
  #385/#388 の契約が main に存在するため blocked 表記を修正。
- [x] UI-DVR-05/06 の gpui binding を `test-ui-gpui`（mise-task）で
  pass 化（UI-DVR-09 の gpui pass は既存）。根拠は
  `lifecycle_fixture::gpui_driver_matches_shared_lifecycle_trace`、
  `rvf_view_native_slot_*` / `surface::tests`。
- [x] GPUI binding の未接続分を owner=#390 の `not_run` + 具体的 rerun
  条件へ修正（UI-DVR-02/03/04 と LAB-SV-FADER-DOCK / LAB-AL-WAVEFORM-SEEK の
  gpui は旧 `blocked` の stale な契約未着表記、UI-DVR-01/07/08 は旧 `not_run` の
  owner/rerun 文言を更新）。
- [x] Story の fake binding（LAB-SV-FADER-DOCK / LAB-AL-WAVEFORM-SEEK）は
  Phase 1 の `pass` が oracle（2 scale factor × 2 theme × state 遷移）を
  覆っていなかったため `not_run`（owner=#390）へ修正。既存 filter は部分
  検証として note と mechanism に残す。
- [x] `ruviform-ui-gpui` allowlist grant を `required:true` へ更新
  （crate は main 上の workspace member で、`required:false` の休眠記述は stale）。
- [x] manifest の mechanism 検査を stories へ拡張し、`mise-task` の
  `[tasks.<name>]` 実在を検査（`verify_story_mechanisms` /
  `verify_binding_mechanism`）。
- [x] boundary ruleset に `protectedPackages`（`ruviform-ui`・
  `ruviform-ui-theme`）を追加し、存在と allowlist 非例外を名指しで pin
  （`protected-missing` / `protected-allowlisted`）。
- [x] Phase 2 検証（同一 tree・`mise run` 経由）: `fmt-check` / `check-ui-driver-boundary`
  （verdict pass）/ `test-ui-driver-contract`（7 tests ok）/ `test-ui-component-view`（15 ok）/
  `check-ui-headless` / `test-ui-headless` / `check-ui-gpui` / `lint-ui-gpui` /
  `test-ui-gpui`（34 ok）/ `lint`（`-D warnings` clean）/ `check` / `test`（workspace 3026 passed,
  0 failed）/ `test-ui-test-msrv`（`rustc 1.89.0`、95 ok）。
- [ ] Case manifest 上の GPUI lab binding（UI-DVR-01/02/03/04/07/08 と
  Story 2 件）の接続は未実装（owner=#390 の `not_run`）。
- [ ] Pixel Golden（色/配置）・実描画 screenshot は未実装（#314 の
  Golden Utility 待ち）。OS 実機（#313）・Native Plugin Dock（#388/#389）・
  IME/a11y・製品 precision/RT は別 Gate で未達。
- [ ] Consumer 通常画面の GPUI 型不使用は、本 repo に Consumer コードが
  無いため機械検査できない（boundary guard は workspace member のみ検査）。
