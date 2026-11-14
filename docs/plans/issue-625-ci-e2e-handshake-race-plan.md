---
id: "PLAN-034XqAH9JQDs0Ry1tvZOjh"
type: "plan"
status: "approved"
title: "Issue #625 実施計画 — handshake error 分類の正規化・決定的検証・lane 再検証"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/625"
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
      - type: "file"
        path: "docs/tasks/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "docs/impl/ir/issue-625-ci-e2e-handshake-race.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
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
  implements:
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  implemented-by:
    - "docs/tasks/issue-625-ci-e2e-handshake-race.md"
  depends-on:
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
    - "docs/designs/issue-625-ci-e2e-handshake-race-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-625-ci-e2e-handshake-race-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
  references:
    - "https://github.com/scarecrow173/ruviform/actions/runs/43540652567"
  defers: []
  deferred-by: []
---

# Issue #625 実施計画 — handshake error 分類の正規化・決定的検証・lane 再検証 Implementation Plan

## Goal

run 43540652567 の `verify (ubuntu-latest)` 失敗(`e2e_process::e2e_disable_env_kills_active_transport` が `Io(BrokenPipe)` で panic)の root cause — hello 交換中の transport error が OS・timing 依存で `Io`/`Handshake` に分かれる分類 race — を、分類を所有する `TestDriver::handshake` で正規化して解消する。

## Scope

単一実装ストリーム。`crates/ruviform-ui-test/src/driver.rs` の `handshake()` 分類正規化 + `spawn_tcp` accept error 経路の同一方針適用 + driver 内 unit test、doc-chain 6 件、`evidence/rf625/` 証跡。`dummy_app`/agent/transport、`DriverError` variant 集合、他 crate、workflow、Issue #625 管理 state は変更しない。

## Upstream Documents

- spec: `docs/specs/issue-625-ci-e2e-handshake-race.md`
- design: `docs/designs/issue-625-ci-e2e-handshake-race-design.md`
- test-spec: `docs/test-specs/issue-625-ci-e2e-handshake-race.md`

## Design Inputs

- `docs/designs/issue-625-ci-e2e-handshake-race-design.md`(approved)

## File Map

- `crates/ruviform-ui-test/src/driver.rs` - `handshake()` の io error 正規化 helper(`hello_io`)追加、hello write/read の error 写像、`spawn_tcp` accept error の child-exit 確認、分類契約の unit test
- `evidence/rf625/` - root cause 分析・検証台帳・NotRun 表(新規)
- `docs/{specs,designs,plans,tasks,test-specs}/issue-625-ci-e2e-handshake-race*.md`、`docs/impl/ir/issue-625-ci-e2e-handshake-race.md` - doc-chain(新規)
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md` - 管理 index 再生成

## Implementation Sequence

- [x] 失敗 run(43540652567)の一次ログを取得し panic 箇所・error 値・他 test 結果を確認
- [x] `2613b238..origin/main` に既修正が無いことを確認(5 commit は `crates/ruviform-ui-test/` 未触)
- [x] doc-chain(spec/design/plan/task/test-spec)作成
- [x] `driver.rs` に `hello_io` helper + `handshake()` write/read error 写像 + `spawn_tcp` accept error child-exit 確認を実装
- [x] driver 内 unit test で分類契約を固定(fake writer/reader の error 経路強制)
- [x] `evidence/rf625/` に root cause 分析・検証台帳を記録
- [ ] `git fetch origin main && git merge origin/main` → 触った docs dir の管理 index を `doc-maintenance` の index apply で再生成
- [x] `mise run fmt-check` / `check ruviform-ui-test` / `lint ruviform-ui-test` / `test ruviform-ui-test --test e2e_process` / `test ruviform-ui-test` / `test-full` を実行し Pass のみ記録(`evidence/rf625/verification.md`)
- [ ] review-protocol レビュー(blocker 解消)→ IR 記録 → commit → push → PR(`Refs #625`)

## Dependencies and Risks

- ubuntu 固有 EPIPE 経路は Windows ローカルで実機再現困難 → CI 一次ログの証拠 + 決定的 unit test で契約検証。実 CI での最終確認は merge 後の対象 SHA の `verify (ubuntu-latest)`(Coordinator が確認)。
- `try_wait` fallback は unmapped errno kind + child exit 確認済みに限定 — kind match で観測済み errno は確定的に覆われる。
- stdio `--stall before-serve` の handshake read hang は別の latent 問題 — 本タスクでは触れず evidence/PR に記録。

## Task Breakdown

- T1: `docs/tasks/issue-625-ci-e2e-handshake-race.md`(本タスク)

## Verification

| 検証 | コマンド | 合格条件 |
| --- | --- | --- |
| fmt | `mise run fmt-check` | exit 0 |
| check | `mise run check ruviform-ui-test` | exit 0 |
| lint | `mise run lint ruviform-ui-test` | exit 0(`-D warnings`) |
| e2e binary | `mise run test ruviform-ui-test --test e2e_process` | exit 0(18 test) |
| crate test | `mise run test ruviform-ui-test` | exit 0 |
| full lane | `mise run test-full` | exit 0(CI 失敗 lane 相当) |
| index 整合 | `doc_maintenance.js index apply` | exit 0 |

## Open Questions / Gaps

- なし(受入条件は Issue #625 トリアージと失敗 job から確定済み)。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(単一ストリーム・小規模変更)
- Independent work streams: none
- User approval required before dispatch: yes → 本環境では review-protocol レビューを承認の代替とする
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: `hello_io` の kind 集合が channel-death に限定されているか、`try_wait` fallback が過剰分類を生まないか、message prefix の後方互換。
