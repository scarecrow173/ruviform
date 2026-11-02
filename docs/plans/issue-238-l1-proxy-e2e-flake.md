---
id: "PLAN-cNIlZseMNfbfI0D3uYpFLK"
type: "plan"
status: "approved"
title: "Issue #238 実施計画 — l1_proxy_e2e 壁時計 sleep 同期を有界 barrier/poll 待機へ置換"
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
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
  implemented-by:
    - "docs/tasks/issue-238-l1-proxy-e2e-flake.md"
  depends-on:
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
    - "docs/designs/plugin-l1-out-of-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-238-l1-proxy-e2e-sync.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #238 実施計画 — l1_proxy_e2e 壁時計 sleep 同期を有界 barrier/poll 待機へ置換

## Goal

`l1_proxy_e2e.rs` の全 sleep 同期サイトを監査し、データ面 barrier（2 連続 `SaveState` round-trip）と有界 counter/lifecycle poll へ置き換える。assertion・対象不変条件は変更しない。

## 中核機構 — pipeline barrier（2 round-trip command）

順序証明（`src/l1/mod.rs` `driver_main`・`drain_submits`、`src/l1/data.rs` `RtLeg::process`、`src/bin/l1-test-worker.rs`・`crates/ruviform-plugin-export/src/lib.rs` `serve` の直列 message loop より）:

- driver は 1 iteration あたり `recv` した `DriverCmd::Command` を stdin へ書いた **後に** `drain_submits` で全 queued submit を `BlockIn` として stdin へ書く。stdin は FIFO。
- child（fixture worker・real runner いずれも単一スレッド直列 loop）は stdin 順に dispatch し stdout へ書く。reader は stdout 順に処理するため、`Reply{cmd}` の waiter 完了より先に直前の `BlockDone`/flagged-`Reply` を result channel へ push 済み。
- よって `execute(A); execute(B)`: A の iteration drain がそれ以前に push された submit を A の後ろに流し、B はその BlockIn 群の後ろに並ぶ → **`execute(B)` が返った時点で、barrier 以前に submit された全 seq の completion element は result channel に push 済み**。次の `process` drain で確定的に pop される。
- `SaveState` は fixture worker（`Ok(State{[0x11,0x22,0x33,0x44]})`）と real runner `GainPlugin`（`Ok(State{RFG1+f32})`）の双方で `Ok` を返すことが確認済み。barrier は reply 内容を使わない（round-trip 完了のみが意味を持つ）。
- 有界性: `execute` は `command_timeout` で区切られ、返答不能なら `Err(Ipc)` + `Failed` 収束 — テストは barrier helper 内で counter/state を含む diagnostic panic に変換する。
- bundled（1 child に複数 instance）では任意 handle への barrier が同じ stdin を覆うため両 pipeline を flush する。Dedicated では handle ごとに barrier が必要。

## Sleep サイト監査表（全 23 サイト分類）

| site | sleep | 目的 | 処置 |
| --- | --- | --- | --- |
| wait helpers ×5 (93,101,2182,2207,2227) | 5 ms poll | deadline poll 自体 | 変更なし（既に正しい形） |
| `process_roundtrip` (742) | 5 ms | lookahead 内配送 | call 毎 barrier（`L1Instance` 版） |
| `process_underrun` (840,842) | 20+5 ms | starved pipeline 再充填 | barrier で flush → call 毎 barrier |
| `bundled_child_hosts_independent_instances` (1005) | 5 ms | 共有 child 上の配送 | call 毎 barrier（a で両者を覆う） |
| `rejected_block_replies` (1857) | 5 ms | seq 0/1 配送 + reject 観測 | call 毎 barrier → `processed` 決定的 |
| `interleave_disjoint_blocks` (2285) | 5 ms | 両 instance 配送 | call 毎 barrier（a のみ — 共有 stdin） |
| `mixed_routes_full_series` (2370) | 5 ms | l1 配送 | call 毎 barrier |
| `runner_bin_serves_real_gain` (2834) | 5 ms | runner 配送 | call 毎 barrier |
| `dedicated_route_isolates` (3227) | 5 ms | b 配送 | call 毎 barrier（b — 専有 child） |
| `runner_bin_serves_bundled` (3309,3336) | 5 ms ×2 | a/b 配送 + param persist | call 毎 barrier（a で共有 stdin を覆う） |
| `audio_counters_in_place_leg` (3435) | 5 ms | driver doorbell 配慮 | **除去** — submit channel cap = depth = 3 ≥ in-flight pushes(3) で構造的に overflow 不可。gate drop は leg 内で完結し `CALLS−3` は timing 非依存 |
| `audio_counters_rejected_blocks` (3508,3526,3528) | 10/50/5 ms | reply 着陸 + slot 解放 | call 毎 barrier → `dropped_submits==0`・`rfo∈[10,18]` が構造化 |
| `audio_counters_detached_endpoint` (3586) | 5 ms | 同上 | **除去** — 同じ構造的上界 |
| heir 3 tests (3829,3869,3909) | 700 ms | drain 500 ms deadline 経過待ち | **除去** — `freeze()` が `drain_cv` で `DrainState::Done` を有界 wait（`mark_drain_bounded` は converge 済） |

## Non-changes（監査済・変更しない）

- `assert_no_notification` の 500 ms grace: 負の assertion は本質的に固定観測窓を要し、既に `NO_NOTIFY_GRACE` 有界。
- `process_hung_child` `<5s`、`command_timeout` `<8s` の elapsed 上限: 時間上限 assert であり同期ではない。
- `spawn_failed_attempt_record_captures_stderr` の `Instant::now()`: timeout window 観測、同期ではない。

## File Map

- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` のみ変更:
  - top-level `pipeline_barrier(&mut L1Instance)` helper（`wire::Command::SaveState` ×2、`Err` は counter dump panic）
  - `composite_e2e::pipeline_barrier(&mut PluginHost, PluginHandle)` helper（`HostCommand::SaveState` ×2、同上）
  - 上表の各 call site

## 直列化・並列度判断

`--test-threads` 無指定のまま（現状維持）。barrier は wait 機構で flake を閉じ、並列性低下は不要 — 各 test が固有 child/process を持つため資源競合はない。採用判断は本 plan で記録（spec AC5 の反復実行で裏付ける）。

## Risk Register

- barrier は `command_timeout` に依存 — hung child では使えない（`--hang` 系は barrier 不可 → 構造的上界 or freeze wait に留める）。
- fixture worker の `SaveState` が instance live を要求: Processing 中は live — 該当範囲のみ使用。
- drain_submits が iteration 中に Command の後で全 drain する前提は `driver_main` の実装検証済み（`mod.rs:1570-1700`）。

## Verification

- `mise run fmt-check` `check` `lint` `test`
- `mise exec -- cargo test -p ruviform-plugin-host --test l1_proxy_e2e` を `--test-threads=1`・既定・`--test-threads=16` で複数 iteration
- IR に反復実行証跡（回数・exit・fail 有無）を記録

## Rollback

`git checkout -- crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` で復旧可能（docs は独立 artifact）。
