---
id: "TSPEC-034SmE0Mm6CdyxKuBCPNAW"
type: "test-spec"
status: "approved"
title: "Issue #360 テスト仕様 — post-mint exit 順序不変条件"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/360"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-360-heir-exit-post-mint-ordering.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-360-heir-exit-post-mint-ordering.md"
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-360-heir-exit-post-mint-ordering.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-360-heir-exit-post-mint-ordering.md"
  derived-by: []
  verifies:
    - "docs/tasks/issue-360-t1-heir-exit-post-mint-ordering.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #360 テスト仕様 — post-mint exit 順序不変条件

## Purpose

`--leave-heir-after-msgs` 系 E2E が「mint 成功後の mid-life 自発 exit」を決定的に配置することを保証する。fixture flag が握手内 message 数を指す場合に mid-mint death（`SpawnFailed{ExitObserved}`）が起こり得ることは product の defined contract であり、テスト側の順序不変条件として防ぐ。本 spec は leave-heir の値が handshake message count を超える限り有効。

## Feature

voluntary exit ordering after mint — live instance を残した child が自発 `exit(0)` し、heir が stdio pipe を保持するシナリオの決定的配置。

## Rules

- Rule: `--leave-heir-after-msgs` を mint 成功を期待するテストで使う場合、その値は `Instantiate` handshake の message 数（`Scan`+`CreateInstance` = 2）より大きく、leave を発火させる message は mint 復帰後にテストが送信する。
- Rule: leave 発火 message の結果 oracle は `{Ok(_), Err(HostError::Ipc)}` — reply 送出済みの exit 収束が demux に先行する残存 race は defined outcome とし、defined rejection は fail。
- Rule: 変更後も従来の検証不変条件（`ExitObserved` notification・`exit.code == Some(0)`・drain deadline/eof/unread/stderr flags・`SpawnPhase::Live` record・bounded teardown）を pin し続ける。

## Examples

- `heir_stderr_after_drain_stop_is_never_read`: `--leave-heir-after-msgs=3` + `Initialize` 送信 → mint 済み instance が `Failed`(`ExitObserved`) へ収束、drain 停止後の heir stderr は未読 `0` byte。
- `heir_stderr_inside_grace_lands_in_record`: 同上 + `--heir-stderr-delay-ms=100` → grace 内 emission が record に 4096 byte 保持される。
- `leave_heir_yields_exit_observed_and_bounded_drain`: 同上（heir stderr なし）→ drain が deadline で bounded 停止、`eof`/`unread` false。
- `destroy_detaches_reader_when_heir_holds_stdout`: 同上（`L1Instance::spawn` 直接経路）→ reader は EOF なしのまま、`destroy`+drop が bounded に detach。

## Guarantees

- mint 成功を期待する全サイトで child exit が mint window 内に起こり得ない（exit は mint 復帰後の送信にのみ発火）。
- `--crash-after-msgs=2` 等の意図的 mid-mint death サイト（reply 前 exit）は変更されず `SpawnFailed` 決定的のまま。

## Non-goals

- product code の mid-mint death 順序（`flip_live`/`converge_child` の競合解消）自体の検証 — 既存 unit 群がカバー。
- `--spawn-heir`・`crash-after`・`hang-after`・`stop-reading` 系サイトの再監査（変更なしを確認するのみ）。

## Risk

- 本保証を外すと `SpawnFailed{ExitObserved}` の scheduler 依存 flake が復活し、無関係 PR の `verify (ubuntu-latest)`/`ci-gate` を再度汚す。
