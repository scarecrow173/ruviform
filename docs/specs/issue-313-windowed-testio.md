---
id: "SPEC-034WIND0WEDTESTI0S2SP"
type: "spec"
status: "approved"
title: "Issue #313 — windowed `TestIo` / `rvf.uitest.v1` 配線（GPUI backend、slice S2）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-313-windowed-testio-design.md"
      - type: "file"
        path: "docs/plans/issue-313-windowed-testio-plan.md"
      - type: "file"
        path: "docs/tasks/issue-313-task2-windowed-testio.md"
      - type: "file"
        path: "docs/test-specs/issue-313-windowed-testio.md"
      - type: "file"
        path: "docs/impl/ir/issue-313-windowed-testio-slice.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/windowed_io.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/bin/windowed_test_app.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/windowed_uitest.rs"
      - type: "file"
        path: "evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/io.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/headless.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/journal.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/agent.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fake_driver.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/bin/dummy_app.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
    - "docs/specs/issue-313-real-os-input-driver.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-314-real-window-adapter.md"
    - "docs/specs/issue-465-legacy-webview-p0p1.md"
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

# Issue #313 — windowed `TestIo` / `rvf.uitest.v1` 配線（GPUI backend、slice S2）

## What Should Be Built

Issue #313 RF-GT-04 slice S2。Issue #312 で確立した
`rvf.uitest.v1` test-control 基盤（`TestIo` port・`TestAgent`・
`TestDriver`・`Journal`・fixtures・faults）の語彙境界を
**neutral driver 語彙**（`DriverEvent`/`WindowLifecycle`/
`WindowDriver`/`WindowSpec`/typed `Event`）へ再表現し、
具象実装が `HeadlessIo`（legacy `FakeHost` 内蔵）のみだった
構造的残件を解消する。実ホスト（`GpuiDriver`）は既に neutral
語彙へ移行済みで、legacy `WindowHandle`/`WindowEvent` を返す経路は
存在しない — `TestIo` が legacy 語彙のままでは GPUI 上に
adapter を書けない。

本スライスは 2 つの成果物を出す。

1. **neutral 化**: `TestIo`/`TestApp`/`HeadlessIo`/`Journal` を
   neutral 語彙へ統一し、`HeadlessIo` を `FakeDriver` 上に
   再基盤化する。wire protocol（`rvf.uitest.v1` の closed op
   set・`proto.rs`/`driver.rs`/`transport.rs`）は不変。
2. **windowed adapter**: `GpuiDriver` 上に windowed `TestIo`
   （`gpui_driver/windowed_io.rs`）と AUT binary
   （`bin/windowed_test_app.rs`）を実装し、実 GPUI window で
   `TestDriver::spawn_stdio` + `TestAgent` serve 経路を駆動する。

## Why Now

- Issue #313 の残件として S1 doc が明示したのが windowed
  `TestIo`/`rvf.uitest.v1` 配線であり、`rvf.uitest.v1` の op
  契約・`TestIo` の語彙・`FakeDriver` の neutral 语义が全て
  main に揃った。
- #465「driver-neutral lane が正本」と整合するには、`TestIo`
  の legacy 語彙を残したまま第 3 の経路を生やすのでなく、
  port 自体を neutral へ統一する必要がある。
- `GpuiDriver` の `UiJob` seam・`capture_window`・
  `real-scenario` feature の dep が全て既存であり、
  cross-thread 注入と実 pixel capture の基盤は確立済み。

## Acceptance（本スライス）

- windowed host で `rvf.uitest.v1` の closed op set
  （windows / receipts / notices / clock / fault / screenshot /
  close / shutdown 系）が `TestIo` 経由で駆動される —
  `TestDriver::spawn_stdio` + AUT binary + agent serve 経路。
- `TestIo`/`TestApp`/`HeadlessIo`/`Journal` が neutral 語彙に
  統一され、既存 headless test 群（headless_contract /
  e2e_process / lifecycle_fixture / driver_contract）が移行後も
  pass する。
- 各 op が実 host の実イベントと同一経路で配送される
  （interceptor が journal + fault filter + app へ流し、`emit`
  注入も同一経路 — "the same delivery a real event loop
  performs"）。
- headless（FakeDriver 経路）で dispatch / journal / metadata /
  fault が検証され、実 window open / focus / capture は
  対話 desktop の manual lane で実走して verdict を記録する。
  environment 起因の失敗は `environment_blocked` / NotRun として
  正直に記録し、Pass に偽装しない。
- capability は正直に claim する: lifecycle / metadata /
  typed-event は実装時のみ claim、screenshot は
  `capture_window` が機能する backend のみ claim、OS input
  injection と DOM access は `TestIo` で claim しない
  （`DeniedCapabilities` のまま）。未対応 op は protocol-level
  `unavailable` を返し、成功に偽装しない。
- 既存 S1 実装（`test-ui-real-os-scenario`・`os_input.rs`・
  RealOs claim）を回帰させない。

## Non-Goals（本スライス）

- OS input injection（SendInput 等）を `TestIo` に追加すること。
  uitest 配線は OS 注入を使わない。
- DOM access を `TestIo` に追加すること。
- macOS / Linux 実機の windowed lane 実走・動的 DPI・IME/a11y。
- pointer 右/中ボタン・wheel・2-window real-OS scenario。
- `ScenarioDriver::RealOs` adapter 本体（#464 系の scope）。
- WebView `run_multi` 経路（backend は GPUI 一本）。
- Linux Xvfb + openbox + lavapipe の CI 化（後続可・本 slice 外）。
- wire protocol closed op set の変更（`proto.rs`/`driver.rs`/
  `transport.rs` は不変）。
