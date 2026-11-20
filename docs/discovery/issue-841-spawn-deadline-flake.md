---
id: "DISC-rf841spawnDeadline"
type: "discovery"
status: "resolved"
title: "Issue #841 L1 spawn/handshake deadline flake（SpawnFailed{Timeout}）— 定量・分類ルール・対処方針"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/841"
    - "crates/ruviform-plugin-host/src/l1/mod.rs"
    - "crates/ruviform-plugin-host/src/l1/diag.rs"
    - "crates/ruviform-plugin-host/src/composite.rs"
    - "crates/ruviform-plugin-core/src/boundary.rs"
    - "crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs"
    - "crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs"
    - "docs/discovery/issue-768-staleness-oracle.md"
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
    - "evidence/rf841/reproduction.md"
    - "evidence/rf768/reproduction.md"
    - "evidence/rf763/verification.md"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-841-spawn-deadline-flake.md"
      - type: "dir"
        path: "evidence/rf841/"
    modified:
      - type: "file"
        path: "docs/test-specs/issue-630-test-fault.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/846"
    - "docs/discovery/issue-768-staleness-oracle.md"
    - "docs/discovery/issue-763-flaky-e2e.md"
    - "docs/test-specs/issue-630-test-fault.md"
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
    - "evidence/rf841/reproduction.md"
    - "evidence/rf841/verification.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #841 L1 spawn/handshake deadline flake — 定量・分類ルール・対処方針

## Summary

`command_timeout=500ms` を設定する hang 系 test（両 ABI
`l1_fault_test_plugin_e2e.rs`）で、test の意図が post-mint の wedged
command 収束であるにもかかわらず、同じ 500ms が pre-mint の spawn
hand-off / `Hello` / `Scan` / `CreateInstance` 各 wait にも効くため、
環境 stall が重なった spawn で `Instantiate` が
`Err(HostError::SpawnFailed{cause: Timeout})` を返して fail する
環境 flake。補助 loop（`evidence/rf841/`）で定量した結果:

- **孤立単発 spawn（cold, 1 spawn/process）は 0/100**（`cargo build`
  負荷併走下を含む）。
- **suite 内に置かれた spawn は 5-30%** — probe 位置を 2 番目→4 番目
  →suite 内と進めると単調上昇し、直列 suite（`--test-threads=1`）でも
  5/20 fail。並列性だけが原因ではない。
- **canonical lane（`mise run test-l1-real-plugin`）連続 6 run で
  2 fail（33%）** — 本環境では lane 単位でも高頻度に発火する
  （full-arm 計測の per-binary-run ~27% から両 ABI 独立なら
  1-(1-0.27)²≈47% と整合する範囲）。
- 全 fail が同一 signature — `hang_converges_on_timeout` の
  `instantiate`（`.unwrap()` @ vst3:193 / clap:148）で
  `SpawnFailed { cause: Timeout, diagnostics_id: 1 }`。

本 class は rf#768 の staleness-oracle（post-mint `LegDeltas` 計数
assertion）とは**構造的に別の失敗面**であり、分類ルールを以下に独立化
する。

## 構造的原因（実コード確認）

`OutOfProcessSpec::command_timeout`（`composite.rs:147-150` — public
field・default 10s）は単一値で以下の全ての wait に共有される:

| wait | 場所 | `SpawnPhase`（freeze 時） |
| --- | --- | --- |
| spawn hand-off（driver → `spawn_rx.recv_timeout`） | `l1/mod.rs:2094` | `Spawn` |
| `Hello` wait（`hello_rx.recv_timeout`） | `l1/mod.rs:2148` | `Handshake` |
| `Scan`（descriptor match） | `l1/mod.rs:2594`（`request` → `request_timed`） | `Handshake` |
| `CreateInstance` reply wait | `l1/mod.rs:2594` 同上 | `Handshake` |
| post-mint 全 command reply wait | `l1/mod.rs:2594` 同上 | `Live` |

- `L1Child::spawn` の RT contract 自体が「最大 `2 × command_timeout`
  （spawn hand-off + `Hello` wait）」と明記（`l1/mod.rs:2063-2064`）。
- `real_test_fault_l1_hang_converges_on_timeout`（vst3 `:633-638`・
  clap `:588-593`）は post-mint `SaveState` reply wait を 500ms で
  収束させるために `command_timeout=500ms` を設定するが、その値が
  上表の pre-mint wait 全てに及ぶ。
- `real_test_spawn_l1_hang_before_hello_times_out`（vst3 `:1284-1289`・
  clap `:1365-1369`）も 500ms を設定するが、child が `Hello` 前に
  park するため `SpawnFailed{Timeout}` が**期待値** — この test では
  deadline 発火は fail として表面化しない（signature 上の非対称）。

## 分類ルール

spawn deadline 失敗（本 class）と staleness-oracle 失敗（rf#763/#768
class）は、証拠の形が構造的に異なる。run log / diagnostics record に
対して以下の判定表を適用する。

| # | 判定 | signature | 帰属 |
| --- | --- | --- | --- |
| C1 | **pre-mint spawn deadline 失敗（本 class / env flake）** | panic `Err(SpawnFailed { cause: Timeout, .. })` が `instantiate`/`instantiate_spawn_failed` 系 helper 由来（`Instantiate` が handle を mint する前の `Err`） | `diagnostics_id` → `host.diagnostics_query(id)` → `DiagnosticQuery::Found(record)` → `record.phase` で発火した deadline を帰属: `PreSpawn`/`Spawn`/`DrainInit`/`Handshake` は pre-mint deadline 系。`record.cause == Some(Timeout)` を確認 |
| C2 | **post-mint staleness-oracle 失敗（rf#763/#768 class）** | test 本体の assertion fail で、assert message が `LegDeltas` 計数（`passthrough`/`dropped` delta・`delivered`/`substituted` 計数・`matched calls` 分類）を含む。`Err` は test 途中の `Err(Ipc)`/`Err(Failed)` 等の post-mint 面 | handle mint 済み — `SpawnFailed` は出ない。rf#768 の matched-only 分類（`issue-768-staleness-oracle.md`）の対象 |
| C3 | **その他**（fixture/runner 欠落・panic・`os error 5` 等） | `fixture cdylib missing`・`os error`・assert 内容が C1/C2 に非該当 | 環境 set-up / 別 defect として分類（rf768 `reproduction.md`「環境前提の補足」先例） |

- **C1 の必要十分**: `Instantiate` が `Err(HostError::SpawnFailed)` を
  返すのは handle mint 前に限られる（`boundary.rs:460-468` —「pre-mint
  only」。mint 済み instance の失敗は `Ipc`/`Failed`/`InstanceLimit` で
  あり `SpawnFailed` には畳まれない）。すなわち log 上の
  `SpawnFailed` panic は定義上 pre-mint であり、post-mint staleness と
  混同し得ない。
- **C1 内部の phase 帰属の限界**: panic 出力は `cause` と
  `diagnostics_id` のみを含み、発火 wait（spawn hand-off か `Hello`
  か `Scan`/`CreateInstance` か）を log からは確定できない。帰属には
  `diagnostics_id` → `diagnostics_query` → `record.phase`
  （`SpawnPhase` = `PreSpawn`/`Spawn`/`DrainInit`/`Handshake`/`Live`、
  `l1/diag.rs:83-97`）を test 側で print する実装が必要（本 slice の
  lease 外 — 実装する場合は両 ABI 対称・後述 Residual）。
  `diagnostics_query` の test 使用先例は同 file
  `real_test_spawn_l1_*` 群（vst3 `:1199`・clap `:1277` 付近）。
- **rf#768 との非矛盾**: rf#768 の matched-only 分類は C2（post-mint
  計数 assertion）だけを対象とし、C1 は同 doc の「別 class」注記
  （`:140-143`・`:253-255`）どおりその適用外。本ルールはその注記を
  独立した判定表として formalize したものであり、matched-only 分類の
  意味・適用条件を変更しない。

### 観測済み signature 一覧（本環境 2026-11-21）

| signature | 発生 test | 分類 | 観測数 |
| --- | --- | --- | --- |
| `SpawnFailed { cause: Timeout, diagnostics_id: 1 }` @ `instantiate` unwrap | `real_test_fault_l1_hang_converges_on_timeout`（両 ABI） | C1 env flake | 補助 loop 21 件 + canonical lane 2 件 |
| その他（staleness 計数・warmup・他 assert） | — | C2/C3 | 0 件 |

`diagnostics_id: 1` が常に 1 なのは、各 test が独立 `PluginHost`
（= fresh `DiagRegistry`、id は host 毎に mint、`composite.rs:477`）を
持ち、その host 内で唯一の record となるため。

## 定量結果（`evidence/rf841/reproduction.md` 詳細）

| arm（probe spawn 位置） | LOAD | fail/run | 注 |
| --- | --- | --- | --- |
| cold（process 内 1 回目） | 0 | 0/60 | |
| cold | 1（sibling `cargo build`） | 0/40 | 単独 spawn は負荷下でも落ちない |
| warm（2 回目、直列 warmup 後） | 0 | 2/40 | 両 ABI 同一 iter で相関 fail |
| warm | 1 | 2/40 | 同上 |
| warm3（4 回目） | 0 | 3/40 | |
| serial（suite 直列・8 番目前後） | 0 | 5/20 | 並列性なしでも発火 |
| full（suite 並列・canonical 相当形状） | 0 | 6/20 | ~30% |
| full | 1 | 5/20 | LOAD 差は本 N では非有意 |
| canonical lane 連続 run | — | 2/6 | run1 vst3・run4 clap、同 signature |

読み取り: 500ms deadline を超える stall 事象は単独 spawn では稀
（0/100）だが、先行して child 生成・kill/reap・DLL load/unload が
連続する suite 内条件（≒ spawn 直前に別 process/thread が OS 資源を
動かしている状態）では大幅に発火しやすい。fail が iter 単位で両 ABI
に相関する点は machine 全体の stall 事象を示唆。`cargo build` 併走
（LOAD=1）の寄与は suite 内条件に比べて小さい。

## Decision（対処方針）

評価対象の選択肢と、定量データ・構造的原因・先例との整合からの推奨:

| 選択肢 | 評価 | 判定 |
| --- | --- | --- |
| A. `spec.command_timeout` 引き上げ（例 500ms→2s） | hang oracle の post-mint 収束は遅くなるが test 自体は緩まない（断言系不変）。ただし pre-mint 側も同値で伸びるため stall 閾値が上がるだけで flake の構造的原因（単一 timeout 共有）は残る。500ms→2s で stall 必要量は 4 倍だが非ゼロ | **単独では不採用**（構造的回避にならない）。他対策との併用なら選択肢になり得る |
| B. 初回 instantiate 前の warm-up spawn | 定量で warm（2 番目 spawn）でも ~5%・suite 内では 25-30% と、warmth はほぼ効かない — 先行 spawn 済み条件こそ脆弱 | **不採用**（データで反証） |
| C. `SpawnFailed{Timeout}` 時の bounded re-instantiate（test-file-only） | `instantiate_spawn_failed` helper + `diagnostics_query` + `SpawnPhase` で C1 signature を識別し、bounded 回数だけ再 instantiate する型。pre-mint 環境許容のみで assertion/gate は不変。rf#766 導入の `ARM_RESEND_CALLS`/`MUTATION_RESEND_CALLS`（bounded resend-until-landed）と同型の「pre-contract 環境 retry」先例が repo 内に存在。**`docs/specs/issue-238-l1-proxy-e2e-flake.md:63` の「flaky 許容を付けない」との整合**: 同先例が禁じるのは assertion 弱化・`#[ignore]`・gate 緩和であり、C1 retry は (i) 検証対象 contract（post-mint hang 収束）を変更しない、(ii) mint 前の環境失敗だけを対象とし contract 未到達を contract 到達へ変換する、(iii) bound 超過は正直に fail、の 3 点で「許容」ではなく「環境分離」と弁明可能。ただし retry が contract 外側に限定されることを signature（`cause==Timeout` かつ `phase` が pre-mint）で明示的に検査する実装が前提 | **推奨（test 側当面策）**。実装は両 ABI 対称が必要・clap 側は rf#842 lease 占有中のため follow-up slice |
| D. canonical lane の失敗許容閾値（N 回中 M 回許容） | Issue Non-goal「canonical lane を無条件に緩める」に該当。閾値を定義しても「無条件でない」とは言いにくく、定量（lane 単位 ~33%）は許容で済む頻度を超える | **不採用** |
| E. production 側 deadline 分離（spawn/handshake 用 timeout を `command_timeout` と分ける） | 構造的原因の根治 — `command_timeout` が spawn hand-off・`Hello`・`Scan`・`CreateInstance`・post-mint command reply を単一値で共有していることが flake の源。pre-mint deadline を別 field（例 `spawn_timeout`）または固定 10s 相当に分離すれば、test が post-mint 用に 500ms を設定しても spawn 経路の環境耐性を損なわない。全 caller の flake を消す根治であり、test 個別の workaround ではない | **推奨（根治）— `ruviform#846` として起票済み**。Issue #841 Non-goal により本 slice では実装しない |

**決定**: 根治は **E**（production deadline 分離 — `ruviform#846`
として起票済み）。test 側の当面策として **C**（`SpawnFailed{Timeout}`・
pre-mint phase 限定の bounded re-instantiate、両 ABI 対称）を推奨
する。B はデータで反証、D は Non-goal、A は単独不採用（E 未実装
期間の緩和としては C+A 併用の余地を残すが、C 単独で当面は十分と
判断 — retry 1 回の追加コストは数百 ms で assertion 面は不変）。

根拠の要約:

- 定量: 孤立 spawn 0/100 vs suite 内 5-30%・canonical 2/6 — 環境 stall
  由来であり、test の assertion 契約（hang 収束）自体に defect はない。
- 構造: post-mint oracle 用の 500ms が pre-mint wait に共有されている
  ことが直接の原因。E はこの共有を解く、C は共有のまま pre-mint
  環境失敗だけを分離する。
- 先例: rf#766 の bounded resend（同じく「環境 drop を contract 外で
  吸収する」型）と整合。#238 の「flaky 許容を付けない」は
  assertion/gate 不変を前提にすれば C と矛盾しない。

## Residual

- **C の実装（test-file-only・両 ABI 対称）**: ruviform#849 で実装
  済み。両 ABI `l1_fault_test_plugin_e2e.rs` の共有 `instantiate`
  helper が `Err(HostError::SpawnFailed)` を捕捉し、
  `host.diagnostics_query(diagnostics_id)` → `DiagnosticQuery::Found`
  record で `cause == Timeout` かつ `record.phase` ∈ {`PreSpawn`,
  `Spawn`, `DrainInit`, `Handshake`} かつ `record.cause ==
  Some(Timeout)` の signature を満たす場合のみ bounded（最大 3
  retry = 4 attempt）re-instantiate する。signature 不一致（`Live`
  phase・非 `Timeout` cause・`Evicted`/`NotFound`）は retry せず即
  fail、bound 超過も正直に fail — assertion・gate・`#[ignore]` は
  不変。`instantiate_spawn_failed`（失敗期待 helper）は厳格のまま
  維持。**coverage boundary（実装時に判明）**: `Scan`・
  `CreateInstance` の wait は `L1Child::spawn` が `Hello` 成功直後に
  `set_phase(Live)` してから走る（`l1/mod.rs:2184`）ため、それらの
  timeout は `phase == Live` の record を残し本 signature では
  retry されない — retry が効く pre-mint wait は spawn hand-off
  （`Spawn`）と `Hello`（`Handshake`）系に限定される。signature は
  本 doc 判定表どおりの保守境界であり、post-mint staleness 誤
  リトライは構造的に不可のまま。
- **E は `ruviform#846` として起票済み**: spawn/handshake deadline と
  `command_timeout` の分離（`OutOfProcessSpec`/`SpawnSpec` への別
  field、または `l1/mod.rs:2094/2148` wait の固定化）。`request_timed`
  （`l1/mod.rs:2606`）に `render_timeout` 先例（video `Render` のみ
  別 timeout を取る）があるため設計の置き場所は既存。
- **phase 帰属 print**: ruviform#849 で実装済み — `SpawnFailed`
  attempt ごとに attempt index・`diagnostics_id`・`record.phase`・
  `record.cause` を stderr へ直接 print（libtest の per-test capture
  を bypass し、retry が recover しても lane log に証跡が残る）。
  panic message にも `record.phase` を含む。
- docs index（`docs/**/README.md`）は Coordinator が統合時に整備。
