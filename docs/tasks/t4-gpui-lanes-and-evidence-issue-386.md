---
id: "TASK-DktMpNJBhrTSp3LAfeKMjo"
type: "task"
status: "proposed"
title: "T4 mise lane・索引・GPUI 採用証跡（Issue #386）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-386-driver-neutral-gui-host.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  implemented-by: []
  depends-on:
    - "../tasks/t3-gpui-driver-minimal-window-issue-386.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T4: mise lane・索引・GPUI 採用証跡

## Work

- `mise.toml` に `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` lane
  を `check-ui-webview` 前例で追加（CI workflow 配線は #294 範囲外）。
- `crates/README.md` に `ruviform-ui-gpui` を追記（索引の
  generated 領域を tool で再生成）。
- `docs/impl/ir/` に実装記録 + GPUI version/license/transitive/
  Windows build 証跡を保存。

## Acceptance

- `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` が
  実行可能で結果が記録される。
- `crates/README.md` の索引が再生成済み。
- 実 OS/実 plugin/IME/DPI 未検証項目が NotRun として IR に明記。
