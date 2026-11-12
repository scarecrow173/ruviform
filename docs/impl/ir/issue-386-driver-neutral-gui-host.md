---
id: "IMPL-Nq2aV7kR4tYc9XwL3ZsH8D"
type: "impl"
status: "in-progress"
title: "Issue #386 Backend-neutral GUI Host 契約・GPUI Driver・Window/Dock 移行 — 第 2 PR 実装記録"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "../../specs/issue-386-driver-neutral-gui-host.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/src/lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/lifecycle_fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lifecycle_fixture.rs"
      - type: "file"
        path: "docs/tasks/t5-shared-lifecycle-fixture-issue-386.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "docs/designs/issue-386-driver-neutral-gui-host.md"
      - type: "file"
        path: "docs/test-specs/issue-386-driver-neutral-gui-host.md"
      - type: "file"
        path: "docs/plans/implement-issue-386-driver-neutral-gui-host.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../plans/implement-issue-386-driver-neutral-gui-host.md"
  implemented-by: []
  depends-on:
    - "../../tasks/t4-gpui-lanes-and-evidence-issue-386.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../designs/issue-386-driver-neutral-gui-host.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../test-specs/issue-386-driver-neutral-gui-host.md"
  derived-by: []
  verifies:
    - "../../test-specs/issue-386-driver-neutral-gui-host.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #386 実装記録（第 2 PR）

## 1. 第 1 PR（#394）までの到達点

`ruviform-ui::driver`（中立契約）・`ruviform-ui-test::FakeDriver`・
`ruviform-ui-gpui::GpuiDriver`（`gpui` feature）・mise lane（`check-ui-gpui`/
`lint-ui-gpui`/`test-ui-gpui`）が main に入っている。Issue #386 は OPEN のまま。

## 2. 第 2 PR で実装したもの

- **共通 lifecycle fixture**（`ruviform-ui-test::lifecycle`）:
  `run_lifecycle` が中立 `WindowDriver` と Dock（`DockWorkspace`）だけで
  2 Window・2-phase move の commit と abort・detach/reattach・focus・
  late `post_event`・close の veto/defer/approve・unanswered close の既定・
  id 非再利用・tear-down を実行し、正規化 trace を `expected_trace()` と比較する。
- **Fake 接続**（`crates/ruviform-ui-test/tests/lifecycle_fixture.rs`）:
  `FakeDriver` が同じ trace に一致する（workspace `test`）。
- **GPUI 接続**（`crates/ruviform-ui-gpui/src/lifecycle_fixture.rs`、
  `cfg(all(test, feature = "gpui-test"))`）: `TestAppContext` の 1 turn ごとの
  settle で `GpuiDriver` が同じ trace に一致する（`test-ui-gpui`）。
  settle の seam `GpuiDriver::settle_for_test` は `cfg(all(test, gpui-test))`
  かつ `pub(crate)` で、公開 signature に gpui 型を出さない。
- **gpui test platform の制約への対応**: test platform の window は OS handle を
  持たず `window_handle()` が panic する。`capture_raw_parent` を
  `cfg(test)` + `gpui-test` ビルドで None を返す seam とした。本番・非 test ビルドの挙動は不変。
  判断理由は surface 親 HWND を lifecycle fixture が使わないこと。
- **移行表**: `docs/designs/issue-386-driver-neutral-gui-host.md` の
  「旧 API → 新 API 移行表」を Host/Window/Dock/Binding の 3 セクション
  （型・実装ファイル・移行手順・状態）に拡張した。

## 3. 依存追加の記録

- `ruviform-ui-gpui` の `[dev-dependencies]` に `ruviform-ui-test`（path）を追加。
  テスト専用で本番グラフには入らない。`ruviform-ui-test` は gpui に依存しない。
  Cargo.lock には `ruviform-ui-test` の 1 行追加のみ。新規外部 crate は無し。

## 4. DoD（Issue #386 本文）対応

| DoD 項目 | 状態 | 根拠 |
| --- | --- | --- |
| common public interfaces が GPUI/JS/WebView/OS Handle なしで build できる | 達成 | `cargo tree -p ruviform-ui -e normal` に gpui/wry/tao/winit なし。`check-ui-driver-boundary` pass（dep-graph・signature・allowlist） |
| 旧 API → 新 API の型/実装ファイル/移行手順の表 | 達成（表）/ 未達（一部移行の実装） | design の移行表 H1–H13・D1–D3・B1–B4。`WebViewWindowConfig`・WebView の `post_event`/Ready/Action 写像は未実装（H1–H5） |
| Fake Driver と GPUI Driver で 2Window/Close veto/focus/Dock rollback の同一 meaning fixture 合格 | 達成（headless 同一 trace）/ 未達（実 OS） | R6。`fake_driver_matches_shared_lifecycle_trace`・`gpui_driver_matches_shared_lifecycle_trace` |
| GPUI への直依存は Driver + composition root 例外のみ | 達成 | gpui は `ruviform-ui-gpui` のみ。`ruviform-ui` の依存グラフに gpui なし |
| Contract は GPUI なしで型検査できる | 達成 | `cargo check -p ruviform-ui`（feature なし）・`cargo test -p ruviform-ui`・`check-ui-driver-boundary` |

## 5. 未達・NotRun（理由つき）

- **実 OS 検証（NotRun）**: 実 window 表示・DPI 遷移・IME・OS 起因の focus・
  実 close ボタン・GPU device lost・実 plugin Native Dock。理由: CI と
  `TestAppContext` では実 platform window がない。手動 entry は
  `example-ui-gpui-minimal`。
- **Late receipt の GPUI 側（部分）**: GPUI の受信は view inbox の件数で検証。
  view の描画までは検証していない（既存の `rvf_view_inbox_queued_receipt_keeps_head` が補完）。
- **Dock と GPUI の統合（未達）**: `DockWorkspace` は headless モデルとして
  共通 fixture で検証した。GPUI の実 window 上の Dock 2-phase move は未実施。
- **WebView 側の写像（未達・Phase 1）**: H1–H5・H9 の adapter 内写像は未実装。
  旧 `HostHandle`/`WindowHandle`/`run` は残置（削除しない）。
- **capability の粗さ（既知の不整合）**: `GpuiDriver` は `focus` を Supported と
  宣言するが、`focus(_, false)` は gpui 0.2.2 の制約で `UiError::Unsupported` を返す。
  fixture は「Ok か Unsupported のどちらか（Host error ではない）」を要求し、
  capability 宣言の精緻化は後続 Issue へ。
- **`render.rs` の clippy 警告（既存・範囲外）**: `clippy --features gpui-test`
  の `too_many_lines`（`rvf_view_native_slot_public_api_cannot_starve_pushes`）。
  既存のテストコードで、`lint-ui-gpui`（`gpui` feature）は対象外。

## 6. 検証結果（第 2 PR の作業ツリー・merge 前）

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | pass |
| `mise run check` | pass |
| `mise run lint` | pass（`cargo clippy --workspace --all-targets -- -D warnings`） |
| `mise run test` | pass（`lifecycle_fixture` 含む） |
| `mise run check-ui-driver-boundary` | pass（dep-graph / signature / allowlist） |
| `mise run test-ui-driver-contract` | pass（5 tests） |
| `mise run test-ui-gpui` | pass（33 + lifecycle_fixture 1） |
| `mise run lint-ui-gpui` | pass |
| `mise run check-ui-gpui` | pass |
| `mise run check-ui-headless` | pass（`tsc --noEmit`） |
| `mise run test-ui-headless` | pass（67 tests） |
| `mise run test-ui-component-view` | pass（15 tests） |
| `mise run test-ui-test-msrv` | pass（MSRV 1.89、`lifecycle_fixture` 含む） |

最終検証は merge 後の HEAD で実施し、PR 本文に結果を記録する。
