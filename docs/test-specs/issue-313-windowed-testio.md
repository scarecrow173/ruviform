---
id: "TSPEC-34WIND0WEDTESTI0S2TS"
type: "test-spec"
status: "approved"
title: "Issue #313 — windowed `TestIo` 検証契約（slice S2）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/windowed_uitest.rs"
      - type: "file"
        path: "evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-313-windowed-testio.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windowed-testio.md"
    - "docs/designs/issue-313-windowed-testio-design.md"
    - "docs/test-specs/issue-312-test-gui-host-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-313-real-os-input-driver.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-313-windowed-testio.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — windowed `TestIo` 検証契約

## 検証対象の受入条件 → 検証方法

| 受入条件 | 検証 |
| --- | --- |
| neutral 語彙統一 + 既存 headless test 群 pass | `cargo test -p ruviform-ui-test`（canonical: `mise run test`）— `headless_contract`/`e2e_process`/`lifecycle_fixture`/`driver_contract`/`fake_driver` が全 pass |
| closed op set が `TestIo` 経由で駆動（windowed） | `tests/windowed_uitest.rs` が `TestDriver::spawn_stdio` で `windowed_test_app` を spawn し `windows`/`receipts`/`notices`/`clock.advance`/`fault`/`screenshot`/`request_close`/`shutdown` を実走 |
| op が実イベントと同一経路で配送 | windowed lane の journal 検証: `emit(Action)`/`emit_ipc_body`/`request_close`/`destroy` 後の receipt が `In "message"`/`Out "post_event"`/`"close_requested"`/`"destroyed"` を含み、interceptor 経由の記録であること |
| dispatch/journal/metadata/fault（headless） | 新規/移行 test: `emit(Tick)`・`emit_ipc_body`（armed → suppressed receipt+notice / valid → `In "message"` + Action 到達 / malformed → `In "message"` + `Out "post_event"` + `Event::error` 到達）・`windows()` meta（title/ready/close_pending/ipc_down）・`window_crash` → `Destroyed` + notice |
| capability honesty | `test.hello` capabilities: windowed は screenshot claim が capture backend 存在 OS のみ・`input_injection`/`dom_access` denied。screenshot op は未対応で `unavailable`（fake data なし） |
| 実 window open/focus/capture | windowed lane: `--windows 1` で実 window open → `windows()` meta `ready=true` 到達（bounded wait）→ `capture_window` で非空 RGBA 取得（capture backend 存在時） |
| graceful cleanup / EOF cleanup | windowed lane: `test.shutdown graceful` → `CleanupReport.windows_closed` が実 window 数分・`exited`・AUT process 終了コード 0。`--fail-init` → `Failed` handshake |
| environment 起因の正直記録 | desktop session 不可・capture 不可 → verdict `environment_blocked`/`unsupported`/`not_run` で記録、Pass 偽装なし |
| S1 回帰なし | `real_window.rs`/`os_input.rs`/scenario contract 未変更 — `test-ui-scenario-contract`/`test-ui-real-scenario`（headless 可能分）pass |

## 検証手順

```sh
mise run fmt-check check lint test test-rt \
    check-ui-gpui lint-ui-gpui test-ui-gpui \
    test-ui-scenario-contract check-ui-driver-boundary
# 実 GUI manual lane（対話 desktop 必須）
RVF_WINDOWED_UITEST=1 mise run test-ui-windowed-uitest
```

## verdict JSON（`rf313-windowed-uitest/v1`）

`evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json`:

- `schema` `rf313-windowed-uitest/v1`、`issue` 313、`slice` `S2`、
  `lane` `windows-gpui-uitest`（または実走 OS）、
  `driver` `gpui-windowed`、`backend` `gpui`。
- `verdict`: `pass`/`fail`/`not_run`/`unsupported`/
  `environment_blocked`。
- `environment`: OS/arch/toolchain/capture backend/desktop session
  有無を実測値で記録。
- `ops`: op ごとの結果表（`hello`/`windows`/`fixture.prepare`/
  `receipts`/`notices`/`clock.advance`/`fault.set`/`screenshot`/
  `request_close`/`shutdown` に `pass`/`unsupported`/`not_run` +
  観測値の要約）。
- `failure`: error/reasons/details を `ErrorBody`/`UiError` から
  保持。

## 判定

- `pass`: handshake `Ready`・全 claim 済 op が成功・
  journal/metadata/fault/cleanup の観測が契約通り・AUT exit code 0。
- `environment_blocked`: desktop session/GUI loop が起動できない
  （`windowed_test_app` が exit 3 で死亡等）。
- `unsupported`: screenshot 未対応 backend など capability 未
  claim の op — op 自体が `unavailable` を返すことを確認して
  `unsupported` と記録（fail ではない）。
- `not_run`: 環境都合で lane 未実施。
- `fail`: protocol error・観測不一致・異常終了。
