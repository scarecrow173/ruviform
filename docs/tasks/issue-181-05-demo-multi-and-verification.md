---
id: "TASK-xyyP2DWC1H1s2hnWVJ6bbA"
type: "task"
status: "done"
title: "demo_multi example + docs sync + full verification + impl record"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-webview/examples/demo_multi.rs"
      - type: "file"
        path: "docs/impl/ir/issue-181-multi-window.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-ui-webview/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-181-multi-window.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-181-03-fake-host-contract-tests.md"
    - "docs/tasks/issue-181-04-webview-run-multi.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-181-multi-window.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-181-multi-window.md"
  references: []
  defers: []
  deferred-by: []
---

# demo_multi example + docs sync + full verification + impl record

## Work

- `crates/ruviform-ui-webview/examples/demo_multi.rs`: `SessionConfig` で 2 Window（main + sub）を起動する multi-window 実機検証例。`handle_window`/`drain_jobs_routed` → `eval` の consumer 配線例、Window 追加、サブ Window の `CloseRequested`→`Approve`（Job 非取消を表示）・最終 Window で `exit()` の last-window policy 例。`required-features = ["webview"]`。
- `crates/ruviform-ui/README.md`/`crates/ruviform-ui-webview/README.md`/`crates/README.md`/`AGENTS.md` に multi-window 契約（`WindowId`/`WindowEvent`/`RoutedEvent`/`WindowHandle`/`run_multi`/close 決定・wire 不変・Job/Window 寿命分離）を転記。
- 全検証: `mise run fmt-check`/`check`/`lint`/`test`/`test-ui-webview`/`lint-ui-webview`/`check-ui-webview`。
- 実機検証は実施した範囲のみ `docs/impl/ir/issue-181-multi-window.md` に記録（未検証 OS/条件・異 DPI・モニター切断を `unverified` 明示）。
- consumer 影響分類（synvelis: impacted、audio-lens: no impact — 単一 Window baseline 不変）を IR に記録。
- docs 索引（各 `docs/*/README.md`）を doc-maintenance で再生成。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-181-multi-window.md`
- `docs/test-specs/issue-181-multi-window.md`
- `docs/designs/issue-181-multi-window.md`（Verification 節）

## Dependencies

- `docs/tasks/issue-181-03-fake-host-contract-tests.md`
- `docs/tasks/issue-181-04-webview-run-multi.md`

## Verification

- `mise run fmt-check`・`check`・`lint`・`test`・`check-ui-webview`・`lint-ui-webview`・`test-ui-webview` green。
- `demo_app`/`demo_cli` 非回帰。IR に実機検証の実施/未実施範囲を記録。
