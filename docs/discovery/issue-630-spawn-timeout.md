---
id: "DISC-rf630spawnTimeout"
type: "discovery"
status: "resolved"
title: "Issue #630 L1 `SpawnFailed{Timeout}` env flake — #846/#849 統合後の残存調査と signature-miss 穴の閉塞"
created: "2026-11-21"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "crates/ruviform-plugin-host/src/l1/mod.rs"
    - "crates/ruviform-plugin-host/src/l1/diag.rs"
    - "crates/ruviform-plugin-host/src/composite.rs"
    - "crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs"
    - "crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs"
    - "docs/discovery/issue-841-spawn-deadline-flake.md"
    - "evidence/rf630/spawn-timeout-verification.md"
    - "evidence/rf630/spawn-timeout-signature-sweep.md"
    - "evidence/rf630/spawn-timeout-plugin-test-sweep.md"
    - "evidence/rf630/spawn-timeout-adapter-sweep.md"
    - "evidence/rf841/verification.md"
    - "evidence/rf849/verification.md"
    - "evidence/rf631/verification.md"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-630-spawn-timeout.md"
      - type: "file"
        path: "evidence/rf630/spawn-timeout-verification.md"
      - type: "dir"
        path: "evidence/rf630/spawn-timeout-logs/"
      - type: "file"
        path: "evidence/rf630/spawn-timeout-signature-sweep.md"
      - type: "file"
        path: "evidence/rf630/spawn-timeout-plugin-test-sweep.md"
      - type: "file"
        path: "evidence/rf630/spawn-timeout-adapter-sweep.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_stateful_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_stateful_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session_l1.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/gui_request.rs"
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
  related: []
tags: ["l1", "spawn", "timeout", "flake", "reliability"]
---

# Issue #630 `SpawnFailed{Timeout}` env flake — 残存調査記録

## 調査の出発点

本 slice の問いは一つに絞られる。#846（spawn deadline の
`command_timeout` からの分離）と #849（test 側 bounded
re-instantiate）の両方が main に統合されたあとで、`l1_fault` E2E の
`SpawnFailed{Timeout}` env flake は構造的に残りうるか。記録済みの
flake 観測は全て統合前の時点にあり、統合後の再観測は evidence 上
存在しない。残るかどうかは仮説ではなく実コードで判定する。

## 発火経路の棚卸し — production 側に穴は残らない

`crates/ruviform-plugin-host/src/l1/mod.rs` を読み直すと、#846 後の
timeout 構造は次のとおりである。`SpawnSpec` は `spawn_timeout` と
`command_timeout` を別フィールドに持ち、spawn hand-off・Hello・
`Scan`/`CreateInstance` reply wait という mint 前の全 wait は
`spawn_timeout` 一本で bound される。post-mint の `request_timed`
だけが `command_timeout` を使う。Coordinator の原因候補だった
「spawn-Hello 待ちへの `command_timeout` の共有」は、統合済みの
構造では発火経路を持たない。

残りうる発火経路を enumerate すると、pre-mint の各 wait が環境
stall で期限超過しうる点は変わらないが、それらは全て正当な環境
期限（`spawn_timeout` bound）であり、契約違反ではない。rf#841 の
分類ルールに沿えば C1（環境 flake）であって C2（契約 verdict の
誤報）ではない。spawn 後の driver teardown も disconnect →
kill/reap で bound-by-construction であり、無限待ちの経路はない。

## signature-miss 穴 — 修正の対象

構造的に残った穴は production ではなく test 側 signature にあった。
#849 の `instantiate()` helper は retry 対象を `phase ∈ {PreSpawn,
Spawn, DrainInit, Handshake}` に限る。しかし `l1/mod.rs` は `Hello`
成功直後に `set_phase(Live)` を進めるため、`Scan`/`CreateInstance`
の reply wait が `spawn_timeout` で timeout したときの record は
`Live` phase を freeze する。この record は #849 signature を
miss し、retry されず即 fail する。rf#841 の Residual 節が
「保守境界」として記録していた未カバー領域がこれである。

`SpawnFailed` は `spawn_failed`/`create_rollback_error` でのみ生成
される — 定義上 pre-mint であり、`instantiate()` 内で返る
`Timeout` cause は全て `spawn_timeout` bound の環境 wait である。
phase 検査は `Live` を外すことで正当な pre-mint 環境失敗まで
排除していた、過剰に狭い proxy だった。

## 修正方式の境界判定

production 側の修正（`set_phase(Live)` の位置変更）は検討したが
棄却した。理由は二つある。一つは、create-stage 失敗の
`phase == Live` を `l1_proxy_e2e.rs` が pin しており、phase 帰属を
動かせばその pin を破壊する。もう一つは、同一 bundled route の
二度目の `instantiate()` が踏む pooled-child の `create_instance`
wait は `Live` phase のまま実行されるため、`set_phase` の位置を
動かしても `Live` record は残る — phase 帰属の変更では穴を
閉じられない。発火経路が正当な環境期限である以上、分離策は
契約上 retry のみであり、rf#841 Decision C・`oos_instantiate` 先例
と同じく test 側 bounded retry の拡張が境界となる。

## 採用した修正と検証

両 ABI の `instantiate()` retry signature の phase 許可集合に `Live`
を追加した。`cause == Timeout`・retained record `Found`・
`record.cause == Some(Timeout)` の三重検査は不変で、非 Timeout
cause や record 不在は signature-miss のまま残る。bound 超過は
panic で honest fail し、期待値経路の `instantiate_spawn_failed`
には retry を載せない。

検証の直接証拠は induce 補助実行が与えた。`CreateInstance` reply
wait を一時的に zero-timeout にする環境変数 patch で `Live` phase
`Timeout` record を強制発火させ、初回のみ発火の arm では retry
後に test が pass し、毎回発火の arm では 4 回の trace を残して
bound 超過 panic した。canonical lane の 6 連続 run では
`SpawnFailed` trace は 0/6 で、統合後の稀少化と整合する。詳細は
`evidence/rf630/spawn-timeout-verification.md` に記録した。

## 残件

- `ruviform-plugin-test-vst3`/`test-clap` の非 fault test file
  （`l1_real_test_plugin_e2e`・`l1_stateful_test_plugin_e2e` 等）の
  instantiate 経路は `SpawnFailed` で即 panic する — 同 signature
  の穴は残り、別 slice の対象となる。**（続報: 後述の plugin-test
  sweep slice で全件判定・閉塞済み）**。`ruviform-plugin-host/tests/`
  と `ruviform-plugin-export/tests/` は後続 slice
  （`evidence/rf630/spawn-timeout-signature-sweep.md`）で全件
  sweep 済み: 7 file の un-retried `Instantiate`/spawn/mint
  surface に同一 bounded retry を適用し、`plugin-export` 側は
  spawn-path 呼出なしを確認。
- `l1_scale_probe.rs` の `#[ignore]`-gated 手動計測 probe は
  sweep 対象外と判定した（契約 test の flake 対策の範囲外）—
  spawn/mint `.unwrap()` は残る。
- `l1_proxy_e2e` の timeout-convergence flake（rf#631 記録）は
  別 class であり本 slice では触れない。
- `SpawnPhase::Handshake` の doc が `Scan`/`CreateInstance` を mint
  window に含めると記述しているのに対し実装は `Hello` 直後に
  `Live` へ進める乖離は、lease 外 pin の都合で維持した —
  観測面上の既知の不一致として記録に留める。

## 続報 — plugin-test crate non-fault file の sweep（本 slice で閉塞）

上記残件の第一項が本 slice の対象だった。
`crates/ruviform-plugin-test-vst3/tests/`（5 file）と
`crates/ruviform-plugin-test-clap/tests/`（4 file）を grep と目視で
全件 sweep し、signature-miss 判定基準に照らした結果は次のとおり
（詳細は `evidence/rf630/spawn-timeout-plugin-test-sweep.md`）。

- 穴あり — 5 file: 両 ABI の `l1_real_test_plugin_e2e.rs`・
  `l1_stateful_test_plugin_e2e.rs`、および vst3
  `os_endpoint_midi_e2e.rs` の `instantiate` helper が un-retried
  surface だった。vst3 側は `HostCommand::Instantiate` の
  `.unwrap()`、clap 側は `SpawnFailed` arm で diagnostics を印字
  して `panic!` する形で、`spawn_timeout` bound の環境 stall が
  test fail へ直結していた。#933 と同一の signature（`cause ==
  Timeout`・record `Found`・`phase ∈ {PreSpawn, Spawn, DrainInit,
  Handshake, Live}`・`record.cause == Some(Timeout)` の検査、
  bound `SPAWN_TIMEOUT_RETRIES = 3`、signature-miss 即 fail、
  bound 超過は panic で honest fail、attempt ごとの stderr
  trace）で閉塞した。clap 側は Issue #207 の diagnostics 全量
  印字を terminal failure 時に温存した。
- 穴なし — 2 file: 両 `host_path.rs` は in-process host 経路で
  `PluginHost`/`HostCommand::Instantiate` を持たず、`SpawnFailed`
  variant を返しえない。
- 対象外: 両 ABI の `l1_fault_test_plugin_e2e.rs`（#933/#935 の
  正本）、期待値経路
  `real_test_plugin_l1_missing_runner_is_spawn_failed`
  （`SpawnFailed{cause: Spawn}` を契約値として pin — retry 非搭載
  を維持）、parent-side `HostCommand::Scan`（descriptor replay で
  あり spawn surface ではない）。
- 検証: induce 補助（`CreateInstance` reply wait の zero-timeout
  一時 patch、検証後 revert 済み）で 4 file について once 相当の
  回復 pass と always 相当の bound 超過 panic を両側実測。
  `os_endpoint_midi_e2e` は loopback pair 未 provision の host では
  endpoint 列挙が先に `verdict: NotRun` を返す構造のため runtime
  未到達 — compile 検証（`check-l1-real-plugin`・
  `lint-l1-real-plugin` exit 0）と同一構造の実測済み helper との
  対称性で裏付ける。canonical lane `mise run test-l1-real-plugin`
  は全 leg `ok`。
- 新たな残件: `ruviform-plugin-vst3`/`ruviform-plugin-clap`
  adapter crate の `tests/l1_real_adapter_e2e.rs` 等は同型穴を
  持ち得るが本 slice の lease 外 — 後続 slice の対象候補として
  記録する。**（続報: 後述の adapter sweep slice で両 file とも
  判定・閉塞済み — signature 穴の残はゼロ）**。

## 続報 — adapter crate tests/ の sweep（本 slice で閉塞・signature 穴の残ゼロ）

前項の残件が本 slice の対象だった。
`crates/ruviform-plugin-vst3/tests/` と
`crates/ruviform-plugin-clap/tests/` を grep と目視で全件
sweep した（worktree HEAD `e145a2d7` の `git ls-files` 実測で
両 crate とも `l1_real_adapter_e2e.rs` 1 file のみ — 計 2
file）。signature-miss 判定基準に照らした結果は次のとおり
（詳細は `evidence/rf630/spawn-timeout-adapter-sweep.md`）。

- 穴あり — 2 file: 両 ABI の `l1_real_adapter_e2e.rs` の
  `instantiate` helper が un-retried surface だった。vst3 側は
  `HostCommand::Instantiate` の `.unwrap()` と
  `panic!("unexpected reply")`、clap 側は `SpawnFailed` arm で
  diagnostics を印字して `panic!` する形で、`spawn_timeout`
  bound の環境 stall が test fail へ直結していた。#933 と同一の
  signature（`cause == Timeout`・record `Found`・`phase ∈
  {PreSpawn, Spawn, DrainInit, Handshake, Live}`・
  `record.cause == Some(Timeout)` の検査、bound
  `SPAWN_TIMEOUT_RETRIES = 3`、signature-miss 即 fail、bound
  超過は panic で honest fail、attempt ごとの stderr trace）で
  閉塞した。clap 側は Issue #207 の diagnostics 全量印字を
  terminal failure 時に温存した。両 file とも composite route
  の helper loop が retry unit — public `PluginHost` API 経由
  のみで pub(crate) bundled spawn surface を持たないため
  `child_attempt` は不要。
- 穴なし・対象外: 期待値経路
  `real_adapter_l1_missing_runner_is_spawn_failed`（両 ABI —
  `SpawnFailed{cause: Spawn}` を契約値として pin、retry 非搭載
  を維持）、parent-side `HostCommand::Scan`（descriptor
  replay であり spawn surface ではない）、post-mint
  `command_timeout` 経路（`drive_to_processing`・state/gui
  verbs・`Destroy`・`kill_child`・notification wait・
  `process`）。
- lease 外の網羅性確認: `crates/*/tests/` 横断 grep で
  `HostCommand::Instantiate`/`register_out_of_process`/
  `SpawnFailed` を洗った結果、spawn surface を持つのは
  #933/#935/#936 と本 slice で対応済みの crate のみだった。
  `ruviform-audio-plugin-bridge/tests/`（4 file）は
  `Instantiate` を呼ぶが `register_factory`（L0 in-process
  route）経由であり `SpawnFailed` variant を返しえない —
  spawn-path 呼出なしとして「穴なし」を記録する（lease 外・
  無変更）。
- 検証: induce 補助（`CreateInstance` reply wait の
  zero-timeout 一時 patch、検証後 revert 済み）で両 file に
  ついて once 相当の回復 pass と always 相当の bound 超過
  panic を両側実測。canonical lane `mise run
  test-l1-real-plugin` は全 leg `ok`（adapter leg 3+3 を含
  む）。

これで `crates/*/tests/` 配下の signature 穴の残はゼロ —
`SpawnFailed{Timeout}` を返しうる全 spawn/mint surface に同一
の bounded retry が載った。残る既知の残件はいずれも本
signature とは別 class であり（`l1_scale_probe` の手動計測
probe・`l1_proxy_e2e` の timeout-convergence flake・
`SpawnPhase::Handshake` doc と実装の乖離・clap editor verb
3・external legs）、本 reliability slice 群の閉塞対象としては
ここで締める。
