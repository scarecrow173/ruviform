---
id: "TASK-034JO3XBRTT20ZNbIBDPRM"
type: "task"
status: "todo"
title: "MSRV 1.89 実測検証と Issue #52 完了確認"
created: "2026-10-21"
updated: "2026-10-21"
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
    - "../plans/implement-issue-52-msrv-raise.md"
  implemented-by: []
  depends-on:
    - "apply-msrv-1-89-and-sync-docs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-52-msrv-raise.md"
    - "apply-msrv-1-89-and-sync-docs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/52"
    - "https://github.com/scarecrow173/ruviform/issues/53"
  defers: []
  deferred-by: []
---

# MSRV 1.89 実測検証と Issue #52 完了確認

## Work

ADR の検証基準どおり、Rust 1.89 toolchain で workspace 全ターゲットを実測し、既存の mise/prek 検証が退行しないことを確認した上で、ドキュメントチェーンを完了状態にして Issue #52 を閉じる。

1. `rustup toolchain install 1.89`（未インストールなら）
2. `cargo +1.89 check --workspace --all-targets` — let-chains を使う bench/test ターゲットを含めて成立確認
3. `cargo +1.89 check -p ruviform-math --lib` — 従来成立していた `--lib` 範囲の退行なし確認
4. `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench-smoke` — pin toolchain（1.99）側の既存検証全パス
5. `prek run --all-files`
6. `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — 本変更起因 findings 0 を確認（既存の非ブロッキング警告は分離して報告）
7. 上流文書の verification チェックボックスを実測結果で更新し、graph route で `implementation-verified` を確認

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-52-msrv-raise.md](../plans/implement-issue-52-msrv-raise.md) — 実施計画
- [apply-msrv-1-89-and-sync-docs.md](apply-msrv-1-89-and-sync-docs.md) — 先行タスク

## Dependencies

- 先行タスク: [apply-msrv-1-89-and-sync-docs.md](apply-msrv-1-89-and-sync-docs.md)

## Affected Files

- `docs/designs/msrv-raise-design.md`（Verification Notes チェックボックス）
- `docs/plans/implement-issue-52-msrv-raise.md`（Verification チェックボックス）
- `docs/specs/msrv-raise-for-fearless-simd.md`（受入条件の結果記録）
- `docs/tasks/README.md`（ステータス同期）

## Implementation Notes

- `cargo +1.89` が未インストールの場合は `rustup toolchain install 1.89` で取得する。mise pin とは別 toolchain で実行することで MSRV 宣言値の実測を担保する
- all-targets が失敗した場合は ADR の対策に従い値を再検討する（spec/ADR/plan を修正してからやり直し）

## Done Criteria

- [ ] `cargo +1.89 check --workspace --all-targets` 成功
- [ ] `cargo +1.89 check -p ruviform-math --lib` 成功
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke` 全パス
- [ ] `prek run --all-files` パス
- [ ] ドキュメント監査で本変更起因 findings 0
- [ ] 上流文書の検証記録が実測結果で更新済み

## Verification

- [ ] 上記 Done Criteria 全項目

## Completion Notes

<!-- Record outcome, deviations, and evidence after implementation. -->
