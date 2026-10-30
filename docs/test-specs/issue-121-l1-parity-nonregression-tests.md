---
id: "TSPEC-034OpmNQsPNOBT99IHbrQz"
type: "test-spec"
status: "approved"
title: "L1 diagnostics parity・無退行（Issue #121 — video parity・supervisor pass-through・L0/wire/supervisor 契約維持）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/116"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-121-l1-stderr-drain-tests.md"
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/adr/issue-121-l1-diagnostics-policy.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-121-diagnostics.md"
    - "docs/designs/issue-121-l1-diagnostics.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 diagnostics parity・無退行（Issue #121 — video parity・supervisor pass-through・L0/wire/supervisor 契約維持）

## Purpose

pin audio/video parity と既存契約の無退行 — spec R-B6/R-B7/R-B8・AC6。video 子は audio と同じ stderr drain・cause 通知経路を通しつつ audio counter は適用しない；supervisor は watched `InstanceFailed` を従来どおり消費し Unsupervised は pass-through；L0・`Failed`→`Destroy`・wire バージョン・通知 exactly-once は変えない。

## Feature

video dedicated 子も piped stderr + drain + `FailureCause` 通知を持ち（dedicated per instance・shared memory 経路は #118 契約のまま）、bundled 子の fan-out で複数 instance が同一の子スコープ診断を共有し、supervisor は `cause` を payload 転送せず既存復旧を維持する。

## Rules

- **video parity（spec R-B6・AC6）**: video dedicated 子の `spawn_scanned` 配管・drain・deadline・flags・retention は audio と同一機構・同一契約。`create_video_instance` 失敗・render timeout→`Timeout`・protocol 違反→`Protocol`・crash→`ExitObserved` の通知 payload が同一语义。**audio counter は video route に載らない**（`RtLeg` 計数は audio のみ — video に counter query 面は作らない）。
- **bundled fan-out（spec R-B6）**: bundled route の失敗は全同居 instance に同じ `cause` の通知が fan-out する — `InstanceFailed` の exactly-once・`ChildExit` status 保持は instance ごとに独立に守られる。診断レコードは **child スコープ**（instance スコープではない — bundled 子の複数 instance は同一レコードを共有）。
- **supervisor 無退行（spec R-B8・AC6）**: `Supervisor::poll` が watched `InstanceFailed` を既存どおり消費し `Restarted`/`GaveUp` 機械的復旧を維持・`cause` を payload 転送しない・Unsupervised pass-through 不変。watched handle の cause を読むには `cause` 非依存の handle → 診断レコード query が別経路で提供される。
- **既存契約維持（spec R-B7）**: L0 経路・`Failed`→`Destroy`/`Failed` adopt・通知 exactly-once・`ChildExit{code,signal}` 语义・`Err(InvalidHandle)` stale・`Err(InstanceLimit)` unflattened・post-mint waiter の `Err(Ipc)` wakeup — いずれも無退行。
- **trigger ledger（spec R-F3・AC7）**: #120（per-child 3 スレッド目 — driver+reader+stderr drain）と #101（observability 最初の実需要）の再トリガ記録が design/ledger 文書に反映される。

## Examples

- `l1_video_e2e`: video 子 `--stderr-bytes` で stderr 捕捉・render timeout→`Timeout`・protocol violation→`Protocol` 通知・counter query 面は存在しない（非混入）
- bundled 上 N instance + `--crash-after-msgs` → 全 instance が同一 `cause` の `InstanceFailed` を受け取り・子スコープ診断レコードは 1 件
- `l1_supervisor_e2e` 全 suite 無退行 — watched `InstanceFailed{cause}` は `{handle, ..}` で消費・`Restarted`/`GaveUp`/`AttemptFailed` 语义不変・Unsupervised は `cause` を含めて pass-through
- `Err(InstanceLimit)`/`Err(Ipc)` の既存 pin test がそのまま pass（パススルー無退行）

## Guarantees

- audio/video は同一の stderr・cause・deadline 契約 — 片側だけの診断穴がない
- bundled fan-out は全 instance に通知を届け・`ChildExit` status と `cause` を instance ごとに保持
- supervisor の API・復旧语义・pass-through は additive `cause` field のみで语义不変
- wire version・codec・L0 経路は不変 — 全既存 test suite のグリーンが無退行証明

## Non-goals

- video への audio counter 適用・video 固有の診断 counter
- supervisor の cause 分岐・原因ベース復旧ポリシー
- bundled 子の per-instance 診断レコード（child スコープが契約）
- real-adapter（clap/vst3）E2E の diagnostics 拡張（`l1_real_adapter_e2e` は compile 互換のみ）

## Risk

- video spawn が `spawn_scanned` を共有している前提で audio 側だけ変更をテストすると video 側の parity が未検証のまま残る
- bundled fan-out で `cause` を instance ごとに正しく運ばないと AC6 の「全 instance に通知」が欠損
- `ChildExit` に `cause` を混ぜると exit-status fact と failure cause の分離が崩れ exactly-once 语义が損なわれる
