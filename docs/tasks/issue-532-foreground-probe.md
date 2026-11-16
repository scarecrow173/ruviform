---
id: "TASK-034YxGX3MfQKaRtEc92pm7"
type: "task"
status: "in-progress"
title: "Issue #532 foreground-probe slice — 変異 probe case + 状態復元 + 二分実走"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-532-foreground-probe.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/plans/issue-532-foreground-probe.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-532-foreground-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-foreground-probe.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-532-foreground-probe.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-532-foreground-probe.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 foreground-probe slice — 変異 probe case + 状態復元 + 二分実走

## Work

`ruviform-plugin-dock-test` に real-lane case `real-dcomp-foreground`
を追加し、GPUI root の z-order/foreground を段階的に変異させて
外部 HWND stacking と compositor 内部遮蔽を二分する。新規
`src/osmut.rs`（snapshot/変異/復元、plain 値境界）を実装し、
`real_dock.rs` に `observe_editor_foreground_probe` を追加して
`real-dcomp-pixel` 直後に catalog/entry/console を
整合させ（全 14 case）、baseline + env-on の interactive lane を実走し、
二分結論を evidence/doc chain に記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-532-foreground-probe.md` (SPEC-034YxCZzURRuUuQPn2Wdgk)
- `docs/designs/issue-532-foreground-probe.md` (DESIGN-034YxD6W7IXEfl432tYKlt)
- `docs/plans/issue-532-foreground-probe.md` (PLAN-034YxG8sLN3ehSqJxikrYs)
- `docs/test-specs/issue-532-foreground-probe.md` (TSPEC-034YxD7ux8VjsYpPupK624)
- `docs/discovery/issue-532-foreground-probe.md` (DISC-034YxCYwvosUuX47paEbNO)

## Dependencies

- 前提: `osoccl`/`ospix`/`oschild` の既存 query/capture helper、
  `windows-sys 0.61` 既存 feature、`RealOsGuiTest` interactive
  session。

## Affected Files

- `crates/ruviform-plugin-dock-test/src/osmut.rs`（新規）
- `crates/ruviform-plugin-dock-test/src/lib.rs`
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`
- `crates/ruviform-plugin-dock-test/Cargo.toml`
- `crates/ruviform-plugin-dock-test/tests/console.rs`
- `crates/ruviform-plugin-dock-test/README.md`
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl}/issue-532-foreground-probe*`
- `evidence/rf631/plugin-dock-real-scenario*.jsonl`
- `evidence/rf631/verification.md`
- `evidence/rf532/foreground-probe-bisect.md`（新規）

## Implementation Steps

1. `src/osmut.rs` — `RootSnapshot`/`MutationOutcome`/
   `RestoreOutcome` + `observe`/`raise_to_top`/`set_foreground`/
   `set_topmost`/`restore`（Windows + 非 Windows stub、
   `osoccl::ExstyleFacts` 再利用、plain 値のみ）。
2. `lib.rs` — `#[cfg(feature = "real-dock")] mod osmut;` +
   module doc に case 追記。
3. `Cargo.toml` — windows-sys dep コメントを mutation+restore
   責務へ更新（feature 追加なし）。
4. `real_dock.rs` — `real_entries` を `[...; 14]` にし
   `real-dcomp-pixel` 直後に `real-dcomp-foreground` を挿入、
   `real_dcomp_foreground_case`/`observe_editor_foreground_probe`/
   `foreground_probe_children`/`ForegroundVerification` を実装、
   `blocked_reports`/`blocked_probe_violations`/件数 assertion/
   module doc case 表を 13→14 に整合。
5. `real_catalog.rs` — 14 行目追加 + doc。
6. `tests/console.rs` — 21→22・case 一覧・コメント。
7. `README.md` — case 表 + `--include-real` 件数。
8. `cargo fmt` + check/lint/test（default + real-dock）。
9. console/contract/boundary/prek lane。
10. interactive real lane baseline + env-on 実走、evidence 記録。
11. impl docs（ir/exp）+ `evidence/rf532/foreground-probe-bisect.md`。
12. review-protocol → commit/push/PR。

## Acceptance Criteria

- [ ] `real-dcomp-foreground` が catalog/entry/console で整合、
  件数 14。
- [ ] `osmut` が plain 値境界・Windows/stub を持ち、変異と復元を
  実装。
- [ ] baseline + env-on で case が実走し、各段階の適用結果・
  hit-test・screen green・復元結果が JSONL に記録される。
- [ ] `real-dcomp-pixel` verdict が `Fail` を維持。
- [ ] fmt/check/lint/test/console/contract/boundary/prek green。
- [ ] 二分結論が `evidence/rf532/` に記録され、未解決は
  `unresolved` と明記。
- [ ] Issue `#532` open のまま、PR merge しない。

## Verification

- `mise run fmt-check`
- `mise run check/lint/test ruviform-plugin-dock-test`（default）
- `mise run check/lint/test ruviform-plugin-dock-test --features real-dock`
- `mise run test-ui-plugin-dock`
- `mise run test-ui-plugin-dock-console`
- `mise run test-ui-scenario-contract`
- `mise run check-ui-driver-boundary`
- `mise exec -- prek run --all-files`
- `mise run test-ui-plugin-dock-real`（interactive baseline）
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` + `cargo run --features
  real-dock --bin plugin-dock-real-scenario -- --out
  evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl`
  （interactive env-on）

## Notes

- verdict は probe 実行+復元+lifecycle のみ、screen green 非依存。
- `SetForegroundWindow` FALSE は記録のみ（foreground 非対称規則）。
- 復元検証は topmost flag 一致、z-order/foreground は観測記録。
- docs index は Coordinator が統合時に再生成（本 worker は触らない）。
