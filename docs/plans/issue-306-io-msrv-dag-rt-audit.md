---
id: "PLAN-eShTb4vnZSc0gaiPtChnIU"
type: "plan"
status: "approved"
title: "Issue #306 IO E2E slice 3 実装計画"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/designs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/plans/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/tasks/issue-306-a1-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-msrv-dag-rt-audit.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf306/report.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-io-msrv-dag-rt-audit.md"
    - "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-306-io-msrv-dag-rt-audit.md"
    - "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E slice 3 実装計画 Implementation Plan

## Goal

Issue #306 受入条件の「MSRV1.89/Feature/License/DAG/RT 監査」
行を、headless で誠実に実行可能な範囲で閉じる。IO 系 6
crate の MSRV 1.89 lane を `mise.toml` に追加して実走し、
依存 DAG/RT 境界を `cargo metadata`/`cargo tree` で機械確認
して証跡化する。

## Scope

単一実装ストリーム・1 plan・1 task。変更は `mise.toml`・
`evidence/rf306/report.md`・`crates/ruviform-io-e2e/README.md`・
`docs/development-environment.md`・本 doc チェーンに限定。
crate の公開 API・実装・feature・依存は変更しない。

## Upstream Documents

- [docs/specs/issue-306-io-msrv-dag-rt-audit.md](../specs/issue-306-io-msrv-dag-rt-audit.md)（approved）
- Issue #306 本文（受入正本）
- [docs/designs/issue-306-io-msrv-dag-rt-audit.md](../designs/issue-306-io-msrv-dag-rt-audit.md)（approved）
- [docs/test-specs/issue-306-io-msrv-dag-rt-audit.md](../test-specs/issue-306-io-msrv-dag-rt-audit.md)（検証正本）
- 前 slice: [spec](../specs/issue-306-io-composition-e2e.md) /
  [design](../designs/issue-306-io-composition-e2e.md) /
  [plan](../plans/issue-306-io-composition-e2e.md) /
  [test-spec](../test-specs/issue-306-io-composition-e2e.md)、
  [slice 2 spec](../specs/issue-306-io-e2e-fault-fuzz.md) /
  [design](../designs/issue-306-io-e2e-fault-fuzz.md) /
  [plan](../plans/issue-306-io-e2e-fault-fuzz.md) /
  [test-spec](../test-specs/issue-306-io-e2e-fault-fuzz.md)

## File Map

- `mise.toml` — `test-ui-test-msrv` 直後に `[tasks.test-io-msrv]`
  追加（2 コマンド `run` 配列 + `CARGO_TARGET_DIR =
  "target/msrv-io"`）。
- `evidence/rf306/report.md` — 末尾に slice 3 節を追記
  （MSRV lane 実測・DAG/RT audit コマンドと判定・License
  状態・NotRun/Unsupported）。
- `crates/ruviform-io-e2e/README.md` — 検証節に `test-io-msrv`
  lane を追記。
- `docs/development-environment.md` — opt-in task 一覧に
  `test-io-msrv` を追記。
- `docs/{specs,designs,test-specs,plans,tasks,impl/ir}/
  issue-306-*msrv*` — 本 doc チェーン。

## Implementation Sequence

- [x] spec → design → test-spec → plan → task の doc chain 作成
- [x] `mise.toml` に `test-io-msrv` 追加
- [x] `mise run test-io-msrv` 実走（rust@1.89.0）・結果記録
- [x] `cargo metadata`/`cargo tree` audit 実行・判定記録
- [ ] `evidence/rf306/report.md` 追記・README/環境 doc 更新
- [ ] `mise run fmt-check`/`check`/`lint`/`test` で非回帰検証
- [ ] managed docs index 再生成（doc-maintenance apply）
- [ ] `git fetch origin main && git merge origin/main` で最新取込
- [ ] push・PR 作成（日本語・`Refs #306`・close しない）

## Dependencies and Risks

- 依存: rust@1.89.0 toolchain は mise で解決済み
  （`mise exec rust@1.89.0 -- rustc --version` = 1.89.0）。
- Risk: MSRV toolchain での新規 compile error — io 系 crate は
  1.89 での build 実績がなく初回のため、失敗時は原因を
  診断して修正する（MSRV 引上げ・`rust-version` 変更は
  本 slice の権限外。Issue #306 範囲で対処不能なら報告）。
- Risk: feature unify の想定と実際の差異 — `cargo test` 出力の
  per-crate test 数で gzip/zstd・全 format が有効だったことを
  確認する。

## Task Breakdown

| Task | 内容 | 依存 |
| --- | --- | --- |
| A1 | `test-io-msrv` 追加・実走 + DAG/RT audit + evidence/docs 更新・検証 | — |

## Verification

- `mise run test-io-msrv` — rust@1.89.0 で exit 0
- `mise run fmt-check` / `check` / `lint` / `test`（workspace）
  — 非回帰 pass
- DAG/RT audit 3 判定が `evidence/rf306/report.md` に
  コマンド・実測・判定つきで記録済み
- `doc-maintenance` apply 後の `doc-status` audit で新規
  blocking finding なし

## Open Questions / Gaps

- なし（slice 範囲は spec で確定済み。実 OS・consumer・4 GiB・
  Issue close は NotRun/Unsupported として記録のみ）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker 内で直接実装）
- Independent work streams: none
- User approval required before dispatch: no（Orca dispatch 済み・
  review-protocol を承認として扱う）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- MSRV lane が `test-ofx-msrv`/`test-ui-test-msrv` の先例
  （`mise exec rust@1.89.0`・`CARGO_TARGET_DIR` 分離）に
  一致しているか。
- feature unify（io-e2e dev-deps）と deflate 補完の記述が
  実際の `cargo test` 挙動と一致しているか。
- audit 証跡が実コマンド・実出力・判定を含み、NotRun が
  実測範囲と一致しているか。
