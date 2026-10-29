---
id: "TASK-034OpsCV3XA7dtr4iHd2sg"
type: "task"
status: "done"
title: "T7. FailureCause 分類配線（first-observed・timeout-before-kill・通知伝搬）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/76"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/tasks/add-failure-cause-and-spawn-failed.md"
    - "docs/tasks/add-cause-to-instance-failed.md"
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
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T7. FailureCause 分類配線（first-observed・timeout-before-kill・通知伝搬）

## Work

`Inner` に minted instance ごとの `Option<FailureCause>` + 通知状態を保持し、first-observed 固定（同一 `inner` lock 内・上書き禁止 — reader/driver/reap/timeout の競合は lock 直列化順で決まる）で分類表の全観測点へ配線する:

- reader: `Hello`/EOF/decode（decode 不能は phase 不問で `Protocol`・非 `Hello` 先頭 frame は `Handshake`・handshake 中 EOF は `Transport`）
- demux: child-scope/instance-scope violation → `Protocol`
- driver: stdin write 失敗 → `Transport`・command waiter timeout → `Timeout`・watchdog kill は **kill 発行前に `Timeout` 記録**（後続 EOF で `Transport` に置き換わらない）
- `try_wait`/reap: 子終了観測 → `ExitObserved`（status は別の `ChildExit`）・reap/wait syscall error → `Internal`（最初の観測でも `FailureCause` が必ず定義される）
- spawn: OS spawn → `Spawn`・`Hello`/version・`Scan`/descriptor・`CreateInstance` 拒否 → `Handshake`・waiter 個別 timeout → `Timeout`

通知へ `cause` を伝搬（`ChildExit{None,None}` status でも fixed cause で送る・reap は status のみ補完し cause 不変）。T3 の暫定値 `Internal` を実分類に置換。

**変更面の注意**: `converge_child`/`converge_instance` の cause 引数 or `Inner` 記録、notify channel payload（`Receiver<ChildExit>` → cause 同送）、`Shared`/`InstShared` literal の field 追加 — 全て compile-caught。実サイト表:

| site | 数 | 対応 |
| --- | --- | --- |
| `converge_child(shared, …)` production call — `mod.rs:1081,1153,1207,1216,1273,1275,1368,1387,1399,1405,2223` | 11 | cause 記録（引数 or `Inner` 経由） |
| `converge_instance(shared, i)` production call — `mod.rs:1149` | 1 | 同上 |
| test/stamp 呼出 — `mod.rs:3316,3342,4261,4292,4317,4679,4736,4785,4791,5200,5289,5728` | ~12 | シグネチャ追従 |
| `try_recv_notification()` payload consumer — `composite.rs:1152,1154`・harness `l1_proxy_e2e.rs` の `wait_notification`（:82-85）+ 直接呼出（:177,259,428,983 他） | ~7 | payload が cause 同送に変わるため型追従が必要 — **harness file を本 task の changes に含める** |
| `Shared`/`InstShared` literal（`grep 'Shared {\|InstShared {'` = 14） | 14 | field 追加で compile-caught |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`（「原因の分類と収束」節 — 意味ベース分類表・first-observed・poison 非分類）

## Dependencies

- `docs/tasks/add-failure-cause-and-spawn-failed.md`（`FailureCause` 型）
- `docs/tasks/add-cause-to-instance-failed.md`（`InstanceFailed.cause` 型）

## Affected Files

- `crates/ruviform-plugin-host/src/l1/mod.rs`・`src/l1/video.rs`・`src/composite.rs`
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（notify payload 変更で `wait_notification`/`try_recv_notification` 呼出が型追従を要する — Verification がこの file の green を要求）

## Done When

- [x] 分類表の全観測点が `FailureCause` を記録し first-observed が lock 内で固定される
- [x] timeout が kill 発行前に記録される
- [x] `InstanceFailed.cause` が実分類を運び `ChildExit` status 语义が無退行
- [x] reap error が `Internal` として最初の観測でも定義される

## Verification

- [x] `mise run check` exits 0
- [x] `mise run lint` exits 0
- [x] `mise exec -- cargo test -p ruviform-plugin-host` exits 0（分類単体テスト — `converge_cause_is_first_observed_per_instance` 等 182 lib test）
- [x] `mise exec -- cargo test --test l1_proxy_e2e` exits 0（通知 payload・exactly-once 無退行 — 130 件）
