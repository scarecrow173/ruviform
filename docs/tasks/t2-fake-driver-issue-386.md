---
id: "TASK-fG12kYJjj6jfGGJmfuxaeY"
type: "task"
status: "proposed"
title: "T2 FakeDriver（ruviform-ui-test・WindowDriver headless 実装）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/src/fake_driver.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/fake_driver.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  implemented-by: []
  depends-on:
    - "../tasks/t1-driver-neutral-window-contract-issue-386.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/issue-386-driver-neutral-gui-host.md"
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

# T2: FakeDriver

## Work

`ruviform-ui-test` に `FakeDriver` を新設 — `FakeHost`（`WindowHandle`
実装）と同じ close-veto/Destroyed/drop-oldest 意味論を
`WindowDriver` port で実装する。`inject`/`pump` の event 注入・
`delivered`/`windows` の観測面は FakeHost の API 形状を踏襲し、
`post_event` inbox を検査できる inspection API を持つ。

## Acceptance

- `cargo test -p ruviform-ui-test`（headless lane）で
  lifecycle 試験 pass（R2）。
- `UiError::Unsupported`/`Host` の各分岐が試験される。
