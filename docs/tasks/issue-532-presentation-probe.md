---
id: "TASK-BelO1OS2d9cPSnLoYe0rKH"
type: "task"
status: "in-progress"
title: "Issue #532 presentation-probe slice — cover inventory + capture variant + fingerprint case"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-532-presentation-probe.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/plans/issue-532-presentation-probe.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-532-presentation-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-presentation-probe.md"
    - "docs/designs/issue-532-presentation-probe.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-532-presentation-probe.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-532-presentation-probe.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #532 presentation-probe slice — cover inventory + capture variant + fingerprint case

## Work

`real-dcomp-present` case（観測専用 probe）と `ospres` module を
実装し、foreground-probe の残余 3 候補を判別する実走 evidence
を採取する。Issue `#532` は open のまま、修正・方式選定・
merge は行わない。

## Classification

- Type: diagnostic probe（実装 slice、観測のみ）
- Risk: medium（real lane の interactive 実走を含む）
- Estimated: single-session

## Upstream Documents

- `docs/specs/issue-532-presentation-probe.md`
- `docs/designs/issue-532-presentation-probe.md`
- `docs/plans/issue-532-presentation-probe.md`
- `docs/test-specs/issue-532-presentation-probe.md`

## Dependencies

- `ospix`/`osoccl`/`oschild`/`osmut` 既存 module（再利用、変更
  なし — `ospix` は additive extension のみ）。
- `RealOsGuiTest` interactive desktop（real lane 実走に必須）。

## Affected Files

- `crates/ruviform-plugin-dock-test/src/ospres.rs`（新規）
- `crates/ruviform-plugin-dock-test/src/ospix.rs`
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`
- `crates/ruviform-plugin-dock-test/src/lib.rs`
- `crates/ruviform-plugin-dock-test/src/Cargo.toml`（comment のみ）
- `crates/ruviform-plugin-dock-test/tests/console.rs`
- `crates/ruviform-plugin-dock-test/README.md`
- `docs/**`（本 chain + impl/ir/exp）
- `evidence/rf532/presentation-probe.md`（新規）
- `evidence/rf631/verification.md` + lane JSONL 3 本

## Implementation Steps

1. `ospix::capture_print_window_flags(hwnd, flags)` を追加し、
   `capture_print_window` を同 flags の wrapper に整理
   （backend 文字列・既存挙動を維持）。
2. `ospres.rs` を新設 — `inventory()`（`EnumWindows` top-level、
   z-order/class/rect/visible/alive/disabled/exstyle/owner/pid、
   cap+truncated）と `attributes(hwnd)`（`DWMWA_CLOAKED`・
   `GetWindowRgn`/`GetRgnBox`・`GetUpdateRect`・
   `GetLayeredWindowAttributes`）。Windows impl + non-Windows
   stub。全値 plain、HWND は `u64`。
3. `real_dock.rs` に `real-dcomp-present` case を追加 —
   `real-dcomp-foreground` と同じ attach lifecycle、probe 本体
   `presentation_probe_children`（inventory → attributes →
   capture variants → screen → fingerprint）、verdict は
   lifecycle+evaluated+completed のみ。
4. `real_catalog.rs`/`real_entries`/`tests/console.rs`/
   `lib.rs`/`Cargo.toml` comment/`README.md` を 15 行に整合。
5. `cargo fmt` → check/lint/test（default + real-dock）→
   `test-ui-plugin-dock`/`console`/`scenario-contract`/
   `driver-boundary`/`prek`。
6. `plugin-dock-real-scenario --blocked-probe` →
   `mise run test-ui-plugin-dock-real`（baseline）→
   `GPUI_DISABLE_DIRECT_COMPOSITION=1` env-on（`--pixel-artifacts`
   付き）→ headless lane JSONL 更新。
7. `evidence/rf532/presentation-probe.md` に判別結果を記録、
   `evidence/rf631/verification.md` に追記、impl/ir・exp docs
   と exp jsonl を作成。
8. review-protocol で self-review → blocker 解消 → `origin/main`
   merge → 最終 verify → commit/push → PR → `worker_done`。

## Acceptance Criteria

- [ ] spec 受入基準の全項目（catalog 整合・ospres stub・
  flag variant・interactive 実走・既存 case verdict 維持・
  lane green・evidence 記録・Issue open）。
- [ ] test-spec F1-F6 の全 Feature/Rule を満たす。
- [ ] `real-dcomp-pixel`/`real-dcomp-foreground` の verdict が
  変わらない。

## Verification

```sh
cargo fmt --all -- --check
mise run check ruviform-plugin-dock-test
mise run lint ruviform-plugin-dock-test
mise run test ruviform-plugin-dock-test
mise run check ruviform-plugin-dock-test --features real-dock
mise run lint ruviform-plugin-dock-test --features real-dock
mise run test ruviform-plugin-dock-test --features real-dock
mise run test-ui-plugin-dock
mise run test-ui-plugin-dock-console
mise run test-ui-scenario-contract
mise run check-ui-driver-boundary
mise exec -- prek run --all-files
cargo run --locked -p ruviform-plugin-dock-test --features real-dock \
  --bin plugin-dock-real-scenario -- --blocked-probe \
  --out evidence/rf631/plugin-dock-real-headless.jsonl
mise run test-ui-plugin-dock-real   # baseline
GPUI_DISABLE_DIRECT_COMPOSITION=1 cargo run --locked \
  -p ruviform-plugin-dock-test --features real-dock \
  --bin plugin-dock-real-scenario -- \
  --pixel-artifacts evidence/rf631/presentation-probe \
  --out evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl
```

## Notes

- probe は観測専用 — `osmut` には触れない。変異候補
  （RedrawWindow/DwmRegisterThumbnail/Magnification/WGC/
  osmut 再利用/DComp visual 走査）は design で評価・不採用/
  deferred を記録済み。
- fingerprint・green・hit-test は全て note 級データ —
  verdict 入力にしない（foreground slice の先例どおり）。
- docs index（`docs/**/README.md`）は coordinator が統合時に
  再生成 — 本 slice では触らない。
