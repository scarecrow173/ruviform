---
id: "PLAN-YhNVDi1L10HncvCUA5RufU"
type: "plan"
status: "approved"
title: "Issue #181 Multi-Window Host — implementation plan"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/window.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/multi_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/examples/demo_multi.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/event.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/runtime.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/service.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/host.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-ui-webview/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "crates/ruviform-ui-webview/Cargo.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/designs/issue-181-multi-window.md"
  implemented-by:
    - "../tasks/issue-181-01-window-contract-types.md"
    - "../tasks/issue-181-02-runtime-routing.md"
    - "../tasks/issue-181-03-fake-host-contract-tests.md"
    - "../tasks/issue-181-04-webview-run-multi.md"
    - "../tasks/issue-181-05-demo-multi-and-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/adr/issue-181-multi-window-contract-placement-close-decision-ipc-routing.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-181-multi-window.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 181
---

# Issue #181 Multi-Window Host — implementation plan

## Scope Split

design（`docs/designs/issue-181-multi-window.md`）を以下の 5 タスクに分割する。全変更は additive — 単一 Window 経路（`host::run`・`UiRuntime::handle` 系）の既存セマンティクスは不変。

| Task | Scope | Depends |
| --- | --- | --- |
| T1 `issue-181-01-window-contract-types` | `ruviform-ui::window` 契約型一式 + `HostEvent::{Moved,Destroyed}` + lib exports | — |
| T2 `issue-181-02-runtime-routing` | `UiRuntime::handle_window`/`drain_jobs_routed`/`cancel_window_jobs`・Job owner・`ServiceContext::origin` + 単体テスト | T1 |
| T3 `issue-181-03-fake-host-contract-tests` | `tests/multi_window.rs` — FakeHost + 受入テスト（分離/close 決定/dead dest/id 衝突/job routing） | T2 |
| T4 `issue-181-04-webview-run-multi` | `SessionConfig` + `run_multi`（`webview` feature）+ lib exports | T1 |
| T5 `issue-181-05-demo-multi-and-verification` | `demo_multi` example・README・全検証・IR | T3, T4 |

## File Map

### T1 — window contract types

- add `crates/ruviform-ui/src/window.rs`
- modify `crates/ruviform-ui/src/event.rs`（`Moved`/`Destroyed` variant、`CloseRequested` doc 更新）
- modify `crates/ruviform-ui/src/lib.rs`（`pub mod window` + re-export）

### T2 — runtime routing

- modify `crates/ruviform-ui/src/runtime.rs`（`handle_window`/`drain_jobs_routed`/`cancel_window_jobs`・`JobRecord.owner`・`dispatch_for`）
- modify `crates/ruviform-ui/src/service.rs`（`ServiceContext::origin`）
- modify `crates/ruviform-ui/src/lib.rs`（必要なら re-export 追加 — `RoutedEvent` 等は T1 で導出済み）
- modify `crates/ruviform-ui/tests/runtime.rs`（multi-window routing のユニットテスト追加 — fake 不要な handle_window 直接検証）

### T3 — fake-host contract tests

- add `crates/ruviform-ui/tests/multi_window.rs`

### T4 — webview run_multi

- modify `crates/ruviform-ui-webview/src/host.rs`（`SessionConfig`/`run_multi`/Window レジストリ/close 決定機構/shared `WebContext`/`build_webview` の共有 WebContext 対応）
- modify `crates/ruviform-ui-webview/src/lib.rs`（`run_multi`/`SessionConfig` re-export）

### T5 — demo + verification

- add `crates/ruviform-ui-webview/examples/demo_multi.rs`
- modify `crates/ruviform-ui-webview/Cargo.toml`（`[[example]]` required-features）
- modify `crates/ruviform-ui/README.md`・`crates/ruviform-ui-webview/README.md`・`crates/README.md`・`AGENTS.md`（multi-window 契約の転記）
- docs 索引の再生成（doc-maintenance）

## Dependency Graph

```text
T1 ──→ T2 ──→ T3 ──→ T5
 └────→ T4 ────────→ T5
```

T1 は全タスクの型基盤。T2/T4 は独立（ruviform-ui runtime vs webview adapter）。T5 は全実装後の統合検証。

## Risk Register

- **close 決定の既定値誤用**: 未回答 `CloseRequested` は `Approve`（Window 単位）。誤ってループ終了に繋げない — fake-host テストで検証。
- **単一 Window baseline 回帰**: `handle`/`run` を手を入れない構造で担保 + 既存テスト非回帰で検証。
- **tao/wry の動的 Window 生成**: `EventLoopWindowTarget` 経由 `build` は `webview` feature 配下の型検証まで。実機差異（WebKitGTK/WKWebView）は `unverified` 記録。
- **共有 WebContext 寿命**: `run_multi` ローカル所有で全 WebView より長命 — drop 順序は構造で担保。
- **Job owner ≠ 寿命**: owner は routing のみ。Window 破棄後 event の silent drop は PoC 規約として明記（可観測 drop counter は deferred）。

## Rollback

各タスクは additive のため `git revert` で独立に戻せる。単一 Window 経路に変更が無いため baseline への影響はない。`Cargo.lock` は依存追加がない限り変化しない（変化時のみ再生成）。

## Verification Matrix

| AC | 検証 | 実施 |
| --- | --- | --- |
| AC-1 discovery | 文書存在 | 済（本チェーン） |
| AC-2 ADR+design | 文書存在・wire 不変の明示 | 済（本チェーン） |
| AC-3 契約型+runtime | `mise run check`/`lint` + ユニットテスト | T1/T2 |
| AC-4 fake-host テスト | `cargo test -p ruviform-ui --test multi_window` | T3 |
| AC-5 run_multi | `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` | T4 |
| AC-6 demo_multi | example 追加・型検証・実機結果の正直な記録 | T5 |
| AC-7 非回帰 | `mise run fmt-check`/`check`/`lint`/`test` green | T5 |
| AC-8 generic + consumer 分類 | 設計審査 + IR 記録 | T5 |
