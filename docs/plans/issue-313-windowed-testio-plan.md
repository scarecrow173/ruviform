---
id: "PLAN-034WIND0WEDTESTI0S2PL"
type: "plan"
status: "approved"
title: "Issue #313 — windowed `TestIo` / `rvf.uitest.v1` 実装計画（slice S2）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-313-task2-windowed-testio.md"
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
    generated:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/designs/issue-313-windowed-testio-design.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-313-windowed-testio-design.md"
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
    - "docs/test-specs/issue-313-windowed-testio.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — windowed `TestIo` / `rvf.uitest.v1` 実装計画

## File map

| file | 変更 |
| --- | --- |
| `crates/ruviform-ui-test/src/io.rs` | `TestIo` を neutral 語彙へ: `emit(DriverEvent)`・新規 `emit_ipc_body(WindowId, &str)`・`create_window(&WindowSpec)`。module doc を `DriverEvent`/`WindowDriver`/`WindowSpec` 契約へ更新 |
| `crates/ruviform-ui-test/src/fixture.rs` | `TestApp::on_event(DriverEvent, &mut dyn WindowDriver)` + `shutdown(mode, &mut dyn WindowDriver)` へ |
| `crates/ruviform-ui-test/src/journal.rs` | `record_in(&DriverEvent)`・`record_suppressed(window, bytes)`・`JournalHandle: WindowDriver`・`deliver_routed` → `post_event` |
| `crates/ruviform-ui-test/src/fake_driver.rs` | `pub fn ready(&self, WindowId) -> bool` accessor 追加（1 点のみ） |
| `crates/ruviform-ui-test/src/headless.rs` | `FakeHost` → `FakeDriver` へ再基盤化。interceptor が lifecycle notice を生成、`emit_ipc_body` 実装、`windows()` は fake の実 state 読み |
| `crates/ruviform-ui-test/src/agent.rs` | emit 2 箇所を `DriverEvent::Tick` / `Windowed{CloseRequested}` へ。import 差替 |
| `crates/ruviform-ui-test/src/bin/dummy_app.rs` | `WindowSpec`/`DriverEvent`/`WindowLifecycle`/`WindowDriver` へ移行。`UiRuntime::dispatch_window_intent` + `deliver_routed`（`post_event`）。`ipc_emit` → `emit_ipc_body` |
| `crates/ruviform-ui-test/tests/` | legacy 語彙使用 test を neutral へ移行（`headless_contract` 等）+ `emit_ipc_body` malformed/suppressed 検証追加 |
| `crates/ruviform-ui-gpui/src/lib.rs` | `#[cfg(feature = "uitest")] pub mod windowed_io` 宣言。`UiJob`/`ui_jobs`/`set_ui_jobs`/`drain_ui_jobs` の cfg を `any(real-scenario, uitest)` に widen |
| `crates/ruviform-ui-gpui/src/gpui_driver/windowed_io.rs` | **新規**: `WindowedIo`（control-thread `TestIo`）+ `UiSide`（`deliver` interceptor + `pending_shutdown` 処理）。`call`/settle 2-job パターン、emit per-event mapping、`windows()` job 実 slot 読み、screenshot `capture_window`→`rgba-raw`、fault（`ipc_disconnect` shared set・`window_crash` destroy+settle） |
| `crates/ruviform-ui-gpui/src/bin/windowed_test_app.rs` | **新規**: AUT。main=serve 側・app thread=`driver.run`（`catch_unwind`→`process::exit` fail-closed）。`windows`/`ipc_emit`/`close_policy` fixtures・`--fail-init`/`--windows N` |
| `crates/ruviform-ui-gpui/Cargo.toml` | `uitest` feature（`["gpui","dep:ruviform-ui-test","dep:base64"]`）+ `base64` optional dep + `[[bin]] windowed_test_app` |
| `crates/ruviform-ui-gpui/tests/windowed_uitest.rs` | **新規**: `RVF_WINDOWED_UITEST=1` opt-in の manual lane テスト。`CARGO_BIN_EXE_windowed_test_app` を `TestDriver::spawn_stdio` で spawn → hello/windows/fixture/close/screenshot/shutdown 実走 + verdict JSON 出力 |
| `mise.toml` | `test-ui-windowed-uitest` manual lane 追加 |
| `evidence/issue-313-windowed-uitest/` | verdict JSON + report |

## Task 分割

1. T1: neutral 語彙化 — `io.rs`/`fixture.rs`/`journal.rs`/
   `fake_driver.rs`/`agent.rs`/`headless.rs`/`dummy_app.rs` +
   tests 移行。headless lanes が pass すること。
2. T2: `windowed_io.rs` + `bin/windowed_test_app.rs` + `uitest`
   feature + lib.rs cfg 変更。`check-ui-gpui`/`lint-ui-gpui`/
   `test-ui-gpui` pass。
3. T3: `tests/windowed_uitest.rs` + `test-ui-windowed-uitest` lane
   + evidence verdict + impl IR。

## 検証行列

| 対象 | 検証 |
| --- | --- |
| 契約回帰 | `mise run fmt-check` `check` `lint` `test` `test-rt` `check-ui-gpui` `lint-ui-gpui` `test-ui-gpui` `test-ui-scenario-contract` `check-ui-driver-boundary` |
| neutral 化回帰 | `cargo test -p ruviform-ui-test`（headless_contract/e2e_process/lifecycle_fixture/driver_contract + 新規 dispatch）— canonical は `mise run test` |
| windowed lane | `RVF_WINDOWED_UITEST=1 mise run test-ui-windowed-uitest`（対話 desktop 実走。desktop session 無しなら `environment_blocked`/`not_run` 記録） |
| S1 回帰 | `mise run test-ui-real-os-scenario` は OS 注入経路 — desktop 環境で可能なら実走、不可なら code 未変更で NotRun 記録 |

## リスク / ロールバック

- **UiJob gate widen**: `any(real-scenario, uitest)` は cfg 結合のみ
  で機能不変 — `real-scenario` build/test で回帰確認。
- **windowed lane の desktop 依存**: 本 worktree は対話 desktop
  （Windows 11 x64）想定だが、desktop session が取れない場合は
  verdict を `environment_blocked` で記録して仕上げる — Pass
  偽装しない。
- **emit の unsupported event**: windowed で `Created`/`Ready`/
  `Resized`/`Moved`/`ScaleFactorChanged`/`Destroyed` の注入は
  drop+notice — headless fixture が `emit(Ready)` を使う慣習を
  windowed AUT fixture が真似ないよう、windowed 側 `windows`
  fixture は real Ready を bounded poll する。
- **`request_shutdown` の job 順序**: confirm job が `Op::Exit`
  適用後に走る順序は `drain`（ops loop → `drain_ui_jobs`）の
  既存構造に依存 — 構造変更なしで成立。
- **`Service`/`UiRuntime` が非 `Send`**: `app.shutdown` は UiJob に
  capture できない → `pending_shutdown` + `UiSide::deliver` 内
  処理（design 判断 5）。
