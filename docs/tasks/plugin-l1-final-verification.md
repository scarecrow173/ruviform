---
id: "TASK-034LNYv07MQpWKTmfQ3vX0"
type: "task"
status: "todo"
title: "Issue #76 プラグイン L1 分離 final verification"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/tasks/plugin-host-composite-and-l1-e2e.md"
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
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #76 プラグイン L1 分離 final verification

## Work

**Full Gate（1 回のみ・plan 最終段・action 18）**: `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke`・`mise exec -- prek run --all-files`・doc audit — 全 log を `docs/impl/exp/data/` に保存。`test-rt` は実一致件数で証明（0 件スキップを成功と偽らない — 0 件一致は skip-equivalent で前向き証拠にならない・`rt_l1_*` が実選択されることを記録）。L1 は feature gate ではなく常時コードのため `cargo test --workspace` が E2E 統合テスト（`tests/`・`CARGO_BIN_EXE_*`）を含めて実行する — 別 lane は不要。Windows ローカル + Rust 実証のみ・mac/Linux/CI は実施時のみ記録（CI `verify` ジョブの 3 OS matrix に委譲 — 未実行の 3 OS 実証を主張しない）

## Verification

- [ ] `mise run fmt-check` — exit 0（`issue-76-final-gate-fmt-check-*.log`）
- [ ] `mise run check` — exit 0（`issue-76-final-gate-check-*.log`）
- [ ] `mise run lint` — exit 0（`issue-76-final-gate-lint-*.log`）
- [ ] `mise run test` — exit 0（全 test binaries・passed 件数を記録 — L1 E2E 統合テストを含む・`issue-76-final-gate-test-*.log`）
- [ ] `mise run test-rt` — exit 0 かつ **実一致件数 >0**（`rt_l1_process_path_no_alloc`/`rt_l1_result_leg_try_ops` が実選択 — `issue-76-final-gate-test-rt-*.log`）
- [ ] `mise run bench-smoke` — exit 0（`issue-76-final-gate-bench-smoke-*.log`）
- [ ] `mise exec -- prek run --all-files` — exit 0（`issue-76-final-gate-prek-*.log`）
- [ ] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` — exit 0・blocking 0・findings []（`issue-76-final-gate-doc-audit-*.log`）
- **expected**: 全コマンド実実行・log 保存済み（`docs/impl/exp/data/`）。full gate は plan 最終段で 1 回のみ実施 — 実施時のみチェックを付け log パスを `changes.generated` へ追記する
