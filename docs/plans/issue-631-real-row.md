---
id: "PLAN-034YqGSA93gczwU51UuDt3"
type: "plan"
status: "approved"
title: "Issue #631 real-row 拡充 実装計画"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/specs/issue-631-real-row.md"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-631-real-row.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-631-real-row.md"
  implemented-by:
    - "docs/tasks/issue-631-real-row.md"
  depends-on:
    - "docs/specs/issue-631-real-row.md"
    - "docs/designs/issue-631-real-row.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-631-real-row.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-real-row.md"
    - "docs/designs/issue-631-real-row.md"
  derived-by:
    - "docs/tasks/issue-631-real-row.md"
    - "docs/test-specs/issue-631-real-row.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-631-real-row.md"
  references:
    - "evidence/rf631/verification.md"
  defers: []
  deferred-by: []
---

# Issue #631 real-row 拡充 実装計画

## スコープ分割

1. **catalog metadata** — `real_catalog.rs` に 7 `RealCatalogRow` 追加。
2. **executable entries + cases** — `real_dock.rs`:
   `real_entries()` を 12 要素へ、`RealDockContext.window_b` 追加、
   2 window mount、`observe_open` に `lease_raw` 追加、新規 case
   7 本（`real-multiwindow-focus` / `real-os-input` /
   `real-window-dpi` / `real-window-ime` / `real-window-close` /
   `real-editor-resize` / `real-editor-redock`）、redock 用 event
   系列 check、test の行数更新。
3. **headless probe** — `plugin-dock-real-scenario.rs` に
   `--blocked-probe`（生成 + 自己検査 + exit code）。
4. **lane/docs** — `mise.toml` task `test-ui-plugin-dock-real-headless`
   + コメント整合、`docs/development-environment.md`、crate
   `README.md`/`lib.rs` doc、`evidence/rf631/` 記録。

## 依存関係

- (1) は (2) の前提（pairwise test が両者を比較）。
- (2) の非画素 editor case は既存 `attempt_real_attach`/
  `oschild`/`conclude_editor_session` を再利用。
- (3) は (2) 完了後（catalog 行数が確定してから probe を検査）。
- (4) は (2)(3) 完了後。

## 検証

- `cargo check`/`clippy -D warnings`（`real-dock` feature 含む）。
- `cargo test -p ruviform-plugin-dock-test`（feature 有無両方）。
- `mise run test-ui-plugin-dock-real-headless`（probe + evidence）。
- `mise run test-ui-plugin-dock` / `test-ui-plugin-dock-console`
  非回帰。
- `check-ui-driver-boundary`（GPUI 型非漏洩）。
- `prek run --all-files`。
- interactive host があれば `test-ui-plugin-dock-real` 実走し
  新行の実 verdict を evidence に記録（無ければ NotRun を正直に
  記録）。

## ロールバック

catalog 行追加は metadata + feature-gated module に閉じるため、
revert で headless lane・console listing の双方から除去できる。
db・生成物への不可逆変更なし。
