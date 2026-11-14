---
id: "TASK-034XqAHZThxH8yUF9OOG3k"
type: "task"
status: "done"
title: "Issue #625 T1 — handshake 分類正規化・doc-chain・lane 検証"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/625"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-625-ci-e2e-handshake-race.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #625 T1 — handshake 分類正規化・doc-chain・lane 検証

## Objective

run 43540652567 の `verify (ubuntu-latest)` 失敗を修正し、`e2e_process::e2e_disable_env_kills_active_transport` の handshake refusal 分類が OS・timing に依らず `Handshake` で一意になることを、決定的 unit test と対象 lane のローカル検証で確認する。

## Checklist

- [x] 失敗 run(43540652567)の一次ログ確認 — `e2e_process.rs:325` で `expected handshake refusal, got Io(BrokenPipe)`、他 17 test pass、他必須 job success
- [x] root cause 確定 — `TestDriver::handshake` の hello write/read で「channel closed before welcome」同一条件が write 側 `Io(BrokenPipe)`/read 側 `Handshake(Eof)` に分かれる分類 race
- [x] `2613b238..origin/main` に既修正なし確認(5 commit は `crates/ruviform-ui-test/` 未触)
- [x] doc-chain(spec/design/plan/task/test-spec)作成
- [x] `driver.rs` 実装 — `hello_io` helper、hello write/read error 写像、`spawn_tcp` accept error child-exit 確認
- [x] driver 内 unit test で分類契約固定
- [x] `evidence/rf625/` root cause 分析・検証台帳
- [x] `mise run fmt-check` / `check` / `lint` / `test --test e2e_process` / `test` / `test-full` 実行(全て Pass — `evidence/rf625/verification.md`)
- [x] review-protocol レビュー(blocker なし — ACCEPT WITH NON-BLOCKING NOTES、P3 指摘 2 件は非 blocking)
- [x] `git merge origin/main`(Already up to date)→ docs index 再生成(`doc_maintenance.js index apply` 全 6 dir)→ IR 記録 → commit → push → PR(`Refs #625`)

## Work

<!-- Describe the implementation slice. -->

`crates/ruviform-ui-test/src/driver.rs` のみを変更する。`handshake()` の hello 交換で発生する transport io error を、channel-death 系(`BrokenPipe`/`ConnectionReset`/`ConnectionAborted`/`UnexpectedEof`、または child exit 確認済みの任意 kind)は `DriverError::Handshake("control channel closed before welcome: {e}")` に、それ以外は `DriverError::Io` に分類する helper を追加し、hello write/read の `?` 伝播をこの写像に置き換える。`spawn_tcp` accept loop の非 WouldBlock error でも `child.try_wait()` で exit を確認できれば既存 WouldBlock 経路と同じ `Handshake("app exited before TCP handshake")` を返す。driver 内 `#[cfg(test)]` unit test で fake reader/writer による error 経路を決定的に固定する。e2e assertion・child 側・`DriverError` variant 集合は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- spec: `docs/specs/issue-625-ci-e2e-handshake-race.md`
- design: `docs/designs/issue-625-ci-e2e-handshake-race-design.md`
- plan: `docs/plans/issue-625-ci-e2e-handshake-race-plan.md`
- test-spec: `docs/test-specs/issue-625-ci-e2e-handshake-race.md`
