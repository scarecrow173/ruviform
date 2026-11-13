---
id: "IMPL-mK9jH2gF5dS8aQ1wE4rT7y"
type: "impl"
status: "completed"
title: "Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完 実装記録"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "evidence/rf306/ci-os-l1-report.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-e2e-cases.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/staged_lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-io-e2e-cases.md"
    - "docs/designs/issue-306-io-e2e-cases.md"
    - "docs/plans/issue-306-io-e2e-cases.md"
    - "docs/tasks/issue-306-io-e2e-cases.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-306-io-msrv-dag-rt-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-306-io-e2e-slice-4-feature-matrix-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-306-io-e2e-cases.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-306-io-e2e-cases.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完 実装記録

## 実施サマリ

Slice V+T を実施。対象 SHA `e93c438a` の Full CI run
`43428172820`（completed/success）の生ログを解析し、io 系
crate の OS-L1 実走証拠を `evidence/rf306/ci-os-l1-report.md`
に記録。`staged_lifecycle.rs` に 3 Case を追加し io-e2e を
77 → 80 tests に補完。production code・公開 seam・CI 定義は
未変更。

## Slice V 結果（要約・正本は evidence report）

- verify 3 OS（ubuntu/windows/macos）: `fmt-check` +
  `lint-full`（clippy --all-targets --profile ci -D warnings）
  + `test-full`（cargo test --workspace --profile ci）が
  success。io-e2e 77/0f・io-file（1 ignored=意図的）・
  io-archive 172・io-compression 126・io-structured 150・
  io-wav 38・io-symphonia 50・media-io 66、全て 0 failed。
- compression-verify: 5 feature 組合せで deflate 含め実走
  （ubuntu のみ）。io-structured-verify: 4 組合せ。
- msrv-verify: rust@1.89.0 で io 系 6 crate + deflate 補完。
- rt-verify: `rt_` filter で io binary は compile/link・
  filter 実行（io-e2e の一致は assert_ 誤検出のみ）。
- deflate は io-e2e 非消費のため verify lane では 0 —
  ubuntu lane のみ実走として分離記録。

## Slice T 結果

`staged_lifecycle.rs` 6 → 9 tests（io-e2e 計 80）:

- `post_seal_temp_rewrite_is_not_reverified_before_publish`:
  post-seal temp rewrite が publish で再検証されず改竄
  bytes が destination に publish されることを契約固定。
- `post_seal_temp_delete_fails_publish_not_published`:
  temp delete → NotPublished・destination 非作成・
  recovery cleanup が Absent で成功。
- `recovery_cleanup_failure_returns_handle_for_retry`:
  cleanup 失敗→handle 返却→retry 成功（TempState::Absent）。

## 検証（canonical = mise・Windows 11）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Passed |
| `mise run check ruviform-io-e2e` | 0 | Passed |
| `mise run lint ruviform-io-e2e` | 0 | Passed |
| `mise run test ruviform-io-e2e` | 0 | 80 tests / 0 failed / 0 ignored |

補助証拠: `cargo test -p ruviform-io-e2e --test
staged_lifecycle` 直接実行でも 9/9 Pass（iteration 用）。

## 観測・判定材料

- post-seal temp 改竄は publish で検出されない（seal 時点
  保証の境界）。io-file の再検証強化は設計判断として別
  slice — 本 slice は現行契約を test で固定。
- RT Fake Fault: io-e2e/io 系は non-RT（doc 明記）で rt_
  契約 test 不要。rt_ filter 非混入は lane 実測で確認。
- G07: `PlatformFileOps`/`MockFileOps` は pub(crate)/
  cfg(test) で io-e2e から到達不能。公開化判定は別 slice。

## NotRun / Unsupported / Deferred

実機 macOS/Linux・Network/Cloud FS・実 4 GiB・deflate の
Windows/macOS・consumer cross 集約・Windows L2/L3 実 FS
特殊条件は NotRun/Deferred（evidence report §7 が正本）。
Indeterminate/TempState::Unknown は Unsupported（公開 API
到達不能・mock 正本 canonical）。

## 変更ファイル

- `evidence/rf306/ci-os-l1-report.md`（新規）
- `crates/ruviform-io-e2e/tests/staged_lifecycle.rs`（+3 tests）
- `crates/ruviform-io-e2e/README.md`（検証節更新）
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  issue-306-io-e2e-cases.md`（新規 6 文書）
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  README.md`（managed index 再生成）
