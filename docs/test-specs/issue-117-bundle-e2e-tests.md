---
id: "TSPEC-034NlQ9qbEN4prlkfCwvdl"
type: "test-spec"
status: "proposed"
title: "L1 bundle E2E tests（Issue #117 — pool lifecycle・隔離 surface・runner registry・dedicated 非回帰）"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
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
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "docs/test-specs/issue-117-wire-v2-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-117-bundle-child-process.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 bundle E2E tests（Issue #117 — pool lifecycle・隔離 surface・runner registry・dedicated 非回帰）

## Purpose

bundle 制御面の E2E oracle — `OutOfProcessIsolation` 公開面・pool 選択/reuse/lifecycle・runner registry・argv v2・handle minting/reuse・shutdown-on-empty・rollback・`Dedicated` 非回帰・RT 契約を pin する。`Failed` 粒度・demux 判定表は `issue-117-bundle-failed-granularity-tests`、codec 形式検証は `issue-117-wire-v2-codec-tests` が所有する。本 spec は「`Bundled`（default）のもと 1 子 N instance が外部から正しく観測される」ことを対象とする。

## Feature

`PluginHost` は `Route::OutOfProcess{spec, isolation, pool}` で、`Bundled`（default）が `live_count < WIRE_MAX_INSTANCES && !terminal && !shutdown_initiated` を満たす最初の子を pool から再利用・満杯時に新規 spawn し、`CreateInstance` で N instance を 1 子へ mint する。`Dedicated` は同 v2 機構の capacity-1 経路。runner は `instances` registry で per-instance dispatch・defined rejection（`InvalidHandle`/`InstanceLimit`/dup/`UnknownPluginId`）を行う。pool/子 lifecycle は shutdown-on-empty・rollback・last-ref teardown・eviction で完結する。

## Rules

- **隔離 surface**: `#[non_exhaustive] pub enum OutOfProcessIsolation{Dedicated, Bundled}` が公開され `new()` default が `Bundled`。`Dedicated` は同 v2 機構の capacity-1 経路（`L1Child`/demux/converge を通る・pool 非投入）
- **pool 選択・reuse**: `Bundled` の 2 回目 `Instantiate` が `live_count < 64 && !terminal && !shutdown_initiated` を満たす既存子へ `CreateInstance` で mint され、新 spawn しない。`i` は host が mint する lowest-free wire handle
- **per-instance 資源**: 各 instance は独立 `PluginHandle`/`LifecycleState`/`process` 経路・per-instance segment + `RtLeg`・per-instance notification channel を持つ（兄弟と資源を共有しない）
- **`(instance, seq)` 相関**: `Reply`/`BlockDone` が `(instance, seq)` で waiter/`block_in_flight` と相関し、instance 間で混線しない
- **runner registry**: runner は `instances: HashMap<u32, InstanceEntry>` で per-instance dispatch。`CreateInstance` 検証 chain: `plugin_id ∉ factories` → `Err(UnknownPluginId)`・`len ≥ 64` → `Err(InstanceLimit)`（capacity は dup より先 — 満杯 registry は全合法 handle を保持するため・`export/lib.rs` `create_instance` の順が正本）・`i` 既登録 → `Err(Ipc)`・shm attach/header/depth 不整合 → `Err(Ipc)`。未登録 `i` 宛 instance 動詞/`BlockIn` → `Reply{i,seq,Err(InvalidHandle)}` flagged（子は継続）。echo は verbatim。argv v2 = `<runner>` + spec.args のみ・`--ruviform-*` 受信は usage reject exit 2
- **host 側写像**: `Scan`-verified id で `Err(UnknownPluginId)` → host が `Err(Ipc)` へ fold。`Err(InstanceLimit)` `Instantiate`/`CreateInstance` → `Err(InstanceLimit)` 伝搬。`Destroy` → `DestroyInstance`（`Failed`+live child でも送信・terminal child なら local removal のみ・`Ok`）
- **shutdown-on-empty + eviction**: live=0 で `Shutdown`+grace+reap + pool evict。`converge_child` で terminal 子を pool から除去
- **rollback**: fresh-spawn `CreateInstance` 失敗 → 子ごと teardown+remove（兄弟なし）。shared-child `CreateInstance` 失敗 → instance entry rollback のみ（健康な兄弟は生存）
- **last-ref teardown**: `L1Instance` drop / route teardown で最後の参照が外れた子を最終 referrer が bounded teardown（`Shutdown`+grace or kill→reap→join）。live 兄弟を持つ子は殺さない
- **live_count semantics**: `live_count` は registered 全件（`Failed` 未 `Destroy` 含む）を数える — pool 選択 predicate が使う値
- **handle minting/reuse**: lowest-free wire handle を `Inner` 排他内で採番。destroy→create 同一 `i` 時、`Destroying` gate が新 doorbell を止め `mark_failed` で解放し、stale doorbell が新 ring へ届かない
- **argv v2**: spawn は `<runner>` + `spec.args` のみ・`--ruviform-shm-key`/`--ruviform-plugin-id` は `CreateInstance` payload（host は送らない・runner/worker は usage reject exit 2 維持）
- **RT 契約保存**: per-instance `RtLeg` 不変条件維持・`rt_` 命名テストが bundle 経路で `assert_no_alloc` を pass する

## Examples

- pool reuse: `Bundled` route で 2 回 `Instantiate` → 同一 `L1Child`（子 process id 同値）・2 つの独立 `PluginHandle`（別 handle・独立 lifecycle/process 経路）
- bundle E2E: 2 instance で `activate`→`process`→`deactivate`→`destroy` を交互に実行し、(instance, seq) 相関が混線しない・各 instance が独立 lifecycle を遷移する
- `kill_child` bundle: `Bundled` 子を kill → 全 live instance `Failed`・per-instance `InstanceFailed`（同値 `exit` N 通知列）
- `InstanceLimit`: 64 超 `CreateInstance`/`Instantiate` → `Err(InstanceLimit)` 伝搬
- `InvalidHandle`: 未登録 `i` 宛 `BlockIn`/`DestroyInstance` → `Reply{i,seq,Err(InvalidHandle)}` flagged・runner 継続
- dup `CreateInstance` → `Err(Ipc)`・`Scan`-verified id の `Err(UnknownPluginId)` → host `Err(Ipc)` fold
- shutdown-on-empty: 最後の `Destroy` → 子 exit（`Shutdown`+reap + pool evict）・route に子なし
- dedicated 非回帰: `isolation: Dedicated` で `Instantiate`→kill→`Failed`→`InstanceFailed` 1 通知 — 既存 singleton 契約と観測同値
- real runner bundle: `composite_e2e` で `Bundled` route → real runner 子へ multi-`CreateInstance`・segment attach・process leg 動作
- `rt_` bundle: bundle 経路の doorbell/staging process leg が `assert_no_alloc` を pass・`Failed` 化しない

## Guarantees

- `Bundled`（default）で 1 子 N instance・`Dedicated`（opt-in）で capacity-1 — 両方とも同 v2 機構を通る
- pool は `live_count < WIRE_MAX_INSTANCES && !terminal && !shutdown_initiated` の predicate で選択・reuse・eviction・shutdown-on-empty・rollback・last-ref teardown で完結する
- runner registry は per-instance dispatch・`CreateInstance` 検証 chain・`InvalidHandle`/`InstanceLimit`/dup/`UnknownPluginId` の defined rejection を行い、echo は verbatim
- `kill_child` は子単位 — bundle 時は影響各 live instance へ `InstanceFailed` が fan-out する
- `rt_` bundle テストが RT 契約（allocation-free・lock-free・nonblocking）を `assert_no_alloc` で pin する
- `Dedicated` 経路で既存 `l1_proxy_e2e` suite が非回帰で green に留まる

## Non-goals

- codec 形式検証・`InstanceLimit` wire tag・scope 判定表（`issue-117-wire-v2-codec-tests` が対象）
- demux 判定表・`Failed` 粒度・notification 意味論の詳細（`issue-117-bundle-failed-granularity-tests` が対象 — 本 spec はその結果の外部観測（kill → N 通知・`Failed`+live child の `Destroy`）のみ検証）
- `MiniHost` fixture の内部機構（`l1_runner_e2e` が所有 — 本 spec は runner E2E の bundle 行のみ）
- shared shm layout / video NONRT / shared reaper / shared IPC（Issue #118/#120/#122 — out of scope）
- `BundleFailed` 相関通知 variant・pool key 跨ぎ bundle・keep-alive（deferred）

## Risk

- pool 選択 predicate の deadlock or 選択ミス → `get_mut` 中に子が teardown され instance が孤立する or 全 instance が単一子へ集中して spawn コスト削減の意義が失われる
- shared-child `CreateInstance` 失敗時に子を殺す → 健康な兄弟 instance が連鎖 `Failed` 化（rollback 判定軸の誤り）
- last-ref teardown 未実装 → 孤児子プロセスが残留する or live 兄弟を持つ子を殺す
- `live_count` の semantics 誤り（`Failed` 未 `Destroy` entry を含めない）→ pool が満杯子へ集中して `InstanceLimit` に達する or 死に子へ mint を試みる
- argv v2 で `--ruviform-*` を受ける旧 worker/runner と新 host が不整合 → handshake が永遠に失敗する
- `Dedicated` で bundle 機構が漏洩 → opt-in 隔離の意味が失われる
