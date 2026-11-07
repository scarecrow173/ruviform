---
id: "PLAN-UO09oNkf6X28oaYsQ5DmNe"
type: "plan"
status: "proposed"
title: "Issue #386 Driver-neutral GUI Host + GPUI minimal fixture 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/386"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-386-driver-neutral-gui-host.md"
  implemented-by:
    - "../tasks/t1-driver-neutral-window-contract-issue-386.md"
    - "../tasks/t2-fake-driver-issue-386.md"
    - "../tasks/t3-gpui-driver-minimal-window-issue-386.md"
    - "../tasks/t4-gpui-lanes-and-evidence-issue-386.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  derived-by:
    - "../tasks/t1-driver-neutral-window-contract-issue-386.md"
    - "../tasks/t2-fake-driver-issue-386.md"
    - "../tasks/t3-gpui-driver-minimal-window-issue-386.md"
    - "../tasks/t4-gpui-lanes-and-evidence-issue-386.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-386-driver-neutral-gui-host.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #386 実装計画: Driver-neutral GUI Host + GPUI minimal fixture

## Scope split

| Task | 内容 | 依存 |
| --- | --- | --- |
| T1 | `ruviform-ui` 共通契約 `driver` module + `UiError::Unsupported` + `UiRuntime::dispatch_window_intent` | — |
| T2 | `ruviform-ui-test` の `FakeDriver`（`WindowDriver` 実装） | T1 |
| T3 | `ruviform-ui-gpui` の GPUI Driver + 最小 fixture + 接続試験 | T1（T2 は同一 meaning 検証で利用） |
| T4 | mise lane（check/lint/test-ui-gpui）+ crates/README + 移行表・証跡 | T1–T3 |

## File map

| 変更 | ファイル |
| --- | --- |
| T1 新設 | `crates/ruviform-ui/src/driver.rs` |
| T1 変更 | `crates/ruviform-ui/src/{lib,error,runtime}.rs` |
| T2 新設 | `crates/ruviform-ui-test/src/fake_driver.rs` + `tests/fake_driver.rs` |
| T3 新設 | `crates/ruviform-ui-gpui/{Cargo.toml,README.md,src/{lib,detect,driver,session,view}.rs,examples/minimal_window.rs,tests/driver_contract.rs}` |
| T4 変更 | `mise.toml`（3 lane 追加）、`crates/README.md`（索引追記）、`Cargo.toml`（members）、`Cargo.lock` |
| T4 新設 | `docs/impl/ir/issue-386-*.md`（実装記録 + GPUI 確認証跡） |

## リスクと対応

| リスク | 対応 |
| --- | --- |
| GPUI 0.2.2 の Windows build 失敗 | 実測済み（`cargo check` 4m36s pass — 本計画立案前に確認） |
| GPUI event loop への `WindowDriver` 同期適合 | `&mut App` を持つ `GpuiDriver` を handler callback 内で生成し、op を `cx.update` 経由で同期実行 |
| `Cargo.lock` の大幅増大 | 契約で予約済み（adapter 層の optional dep。union merge 時に整合） |
| workspace lane が gpui を build しない | `dep:gpui` を非既定 `gpui` feature ゲート。`check --workspace` は従来どおり GPUI なし |
| 他 worktree との共通ファイル競合 | 追加のみ（members 追記・lane 追加・README 追記）。union merge 前提 |

## 検証マトリクス

| 検証 | 入口 | 期待 |
| --- | --- | --- |
| headless 経路 | `mise run check` / `lint` / `test`（workspace） | GPUI なしで全 pass |
| GPUI typecheck | `mise run check-ui-gpui` | `ruviform-ui-gpui` が `gpui` feature で check |
| GPUI lint | `mise run lint-ui-gpui` | `-D warnings` pass |
| GPUI test | `mise run test-ui-gpui` | 接続試験（fake + gpui 同一 meaning）pass |
| 移行表 | design/discovery doc | 全移行対象が表に記載 |
| 実 OS window | 手動 fixture example | 本 Issue では BLOCKED/NotRun で記録 |

## ロールバック

- `ruviform-ui` の変更は追加型のみ。revert は `driver.rs` 削除 +
  `lib.rs`/`error.rs`/`runtime.rs` の追加行除去で復旧可。
- `ruviform-ui-gpui` crate 全体は新設。revert は members 行と
  crate 削除で復旧可。
