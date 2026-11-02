---
id: "TSPEC-Jm8jpNK8kjkyBaizXBqwQo"
type: "test-spec"
status: "approved"
title: "Issue #238 テスト仕様 — l1_proxy_e2e 同期機構の不変条件"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/238"
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
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/plugin-l1-out-of-process.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/impl/ir/issue-238-l1-proxy-e2e-flake-implementation.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #238 テスト仕様 — l1_proxy_e2e 同期機構の不変条件

## Scope

`l1_proxy_e2e.rs` の待機機構のみを対象とし、プロトコル coverage（対象テスト 74 本のアサーション集合）は `plugin-l1-proxy-contract-tests.md` がカバーする既存不変条件のまま維持する。本書は wait 機構自身が守る不変条件を定義する。

## Invariants

- INV-1（barrier ordering）: `execute(A); execute(B)` が `Ok` を返した時点で、barrier 以前に `process` が submit した全 seq の completion element は result channel に push 済み。次の `process` drain で確定的に観測される（成功=`matched` copy、失敗/遅延=`passthrough`、RESULT_FAILED=reject 観測）。
- INV-2（bounded wait）: 全ての待機は明示的 deadline（`NOTIFY_TIMEOUT` / `command_timeout` / 新規 `REPLY_LANDING_TIMEOUT`）で区切られ、超過時は counter/lifecycle/notification を含む diagnostic を添えて panic する。
- INV-3（structural bounds）: hung child 配下の `dropped_submits`・`results_dropped` 期待値は submit channel cap（=depth）と ring 深度の構造的性質から導かれ、wall-clock には依存しない。
- INV-4（drain completion）: heir 3 テストの `heir_stderr` 検証は `try_recv_notification` 内の `freeze()` が `drain_cv` で `DrainState::Done` を有界 wait する事実に同期する（固定 700 ms を置かない）。
- INV-5（negative observation）: `assert_no_notification` の 500 ms 観測窓は「notification が来ない」ことを主張するための必須窓であり有界。削除しない。

## Cases（wait 機構の回帰）

| Case | 検証 |
| --- | --- |
| C1: 配送 paced calls | `process_roundtrip`/`process_underrun`/`bundled_child_hosts_independent_instances`/`interleave_disjoint_blocks`/`mixed_routes_full_series`/`runner_bin_serves_real_gain`/`runner_bin_serves_bundled`/`dedicated_route_isolates` は call 毎 barrier で `matched`/`delivered`/`processed` アサートを満たす |
| C2: counter deterministic | `audio_counters_track_rejected_blocks`: `dropped_submits==0`・`results_failed_observed ∈ [10,18]`・`results_dropped==0`。`audio_counters_*_hung/*_detached_endpoint`: `dropped_submits == CALLS−3` が構造的上界から成立 |
| C3: reject 観測 | `rejected_block_replies_resolve_pass_through`: `processed >= 1` が barrier 配送で保証され、call≥2 の reject bypass アサートが同一 |
| C4: heir drain | 3 テストの `heir_stderr` assert は notification pop 直後に drain 完成済み |
| C5: deadlock 非発生 | barrier は live child（Processing）でのみ呼び、`--hang`/`--crash`/`spawn` 失敗系では呼ばない |

## Evidence

実装後、IR（`docs/impl/ir/issue-238-l1-proxy-e2e-flake-implementation.md`）に反復実行記録（`--test-threads` 1/既定/16 × 反復回数、exit status、fail 一覧）を添付する。
