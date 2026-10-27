---
id: "TASK-034NlQmTAKwSUzU49wWswM"
type: "task"
status: "todo"
title: "L1 route pool + 隔離選択面（OutOfProcessIsolation・Route struct variant・pool lifecycle）"
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
        path: "crates/ruviform-plugin-host/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/tasks/l1-wire-v2-codec-cutover.md"
    - "docs/tasks/l1-child-instance-split.md"
  blocks:
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
    - "docs/tasks/l1-wire-v2-docs-sweep.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/plugin-host-composite-and-l1-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-117-bundle-e2e-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
  defers: []
  deferred-by: []
---

# L1 route pool + 隔離選択面（OutOfProcessIsolation・Route struct variant・pool lifecycle）

## Work

T2 の `L1Child`/demux/teardown 機構を `composite` の `Route::OutOfProcess` へ配線し、`OutOfProcessIsolation` 公開面と pool lifecycle を実装する。`Bundled`（default）が `pool` 経由で 1 子 N instance を実現し、`Dedicated` は同一機構の capacity-1 経路。

1. `crates/ruviform-plugin-host/src/composite.rs` — `OutOfProcessSpec.isolation: OutOfProcessIsolation` field 追加（`new()` default `Bundled`）。`#[non_exhaustive] pub enum OutOfProcessIsolation{Dedicated, Bundled}` を定義（doc に Failed 粒度・liveness trade・`Dedicated`=capacity-1 opt-in を明記）。`Route::OutOfProcess` を struct variant `{spec, isolation, pool: Vec<Arc<L1Child>>}` へ変更
2. `instantiate` — `routes.get` → `get_mut` へ変更。`Bundled`: pool から `live_count < WIRE_MAX_INSTANCES && !terminal && !shutdown_initiated` を満たす最初の子を選び、無ければ `L1Child::spawn` で新規追加 → `Inner` 排他内で lowest-free handle mint → `CreateInstance` 送信 → reply で `L1Instance` 構築。`Dedicated`: 新規 `L1Child::spawn`（pool 非投入）→ `CreateInstance`。`Scan`-verified id で `Err(UnknownPluginId)` → `Err(Ipc)` fold。`InstanceLimit` reply → `Err(InstanceLimit)` 伝搬
3. `destroy` — `DestroyInstance{i}` 送信 → entry `Destroying` → reply で撤去。live=0 判定時に `Shutdown`+grace+reap（shutdown-on-empty）で pool から evict。`Failed` + live child の `Destroy` でも `DestroyInstance` を送信（entry が残っている限り）。terminal child → local removal のみ・`Ok`
4. pool lifecycle — `converge_child` で terminal 子を pool evict。fresh-spawn `CreateInstance` 失敗 → 子ごと teardown + pool remove（兄弟不在のため）。shared-child `CreateInstance` 失敗 → instance entry rollback のみ（健康な兄弟は生存）。`Inner` 排他内で selection predicate を評価して `get_mut` 中の deadlock を回避
5. `kill_child(handle)` — signature 不変。handle が属する子を kill → その子の全 live instance が `Failed`（bundle 時に N fan-out）。doc に「kill は子単位・bundle 時は同所 instance が連鎖」を明記
6. `lib.rs` — `pub use composite::{… OutOfProcessIsolation}` 再公開 + crate doc（L25-26 付近の singleton 記述）を bundle 契約へ更新
7. unit/integration test — `OutOfProcessIsolation` default=Bundled・`Route` struct variant・pool 選択 predicate・`kill_child` 子単位の振る舞いを unit test で pin

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（T3 — pool + 公開面）
- [../designs/issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md) L206-234（Route/isolation/pool lifecycle・instantiate/destroy 写像・shutdown-on-empty・rollback）
- [../adr/issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md) L50-78（`Bundled` default・`Dedicated` opt-in・liveness trade の根拠）
- [../specs/issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md) AC2/AC9/AC11 の surface 側
- [issue-117-bundle-e2e-tests.md](../test-specs/issue-117-bundle-e2e-tests.md)（pool/isolation oracle — T4 で E2E 接続）

## Dependencies

- 先行タスク: [l1-child-instance-split.md](l1-child-instance-split.md) — `L1Child`・live set・`Inner` 排他・mint が前提
- 後続タスク: [l1-bundle-e2e-and-granularity-verification.md](l1-bundle-e2e-and-granularity-verification.md)（T4 が bundle E2E で検証）・[l1-wire-v2-docs-sweep.md](l1-wire-v2-docs-sweep.md)（T5 が公開面 doc を反映）

## Affected Files

- `crates/ruviform-plugin-host/src/composite.rs` — `OutOfProcessIsolation`・`Route` struct variant + pool・instantiate/destroy/kill_child・pool lifecycle
- `crates/ruviform-plugin-host/src/lib.rs` — `OutOfProcessIsolation` 再公開 + crate doc

## Done When

- [ ] `#[non_exhaustive] pub enum OutOfProcessIsolation{Dedicated, Bundled}` が公開され `new()` default が `Bundled`
- [ ] `Route::OutOfProcess` が `{spec, isolation, pool}` struct variant で、`Bundled` が pool 経由で `live_count<64` の子を再利用・満杯時に新規 spawn する
- [ ] `Dedicated` が同 v2 機構の capacity-1 経路として動作する（pool 非投入・spawn 毎に新規 `L1Child`）
- [ ] `destroy` → `DestroyInstance` → live=0 で `Shutdown`+reap + pool evict（shutdown-on-empty）
- [ ] `Err(UnknownPluginId)`→`Err(Ipc)` fold・`Err(InstanceLimit)` 伝搬が動作する
- [ ] `kill_child` が子単位で全 live instance を `Failed` にし doc に明記される
- [ ] pool の fresh-spawn 失敗→子 remove・shared-child 失敗→entry rollback のみが実装される
- [ ] `lib.rs` が `OutOfProcessIsolation` を再公開し crate doc が bundle 契約を述べる

## Verification

- [ ] `cargo test -p ruviform-plugin-host --lib` — pool/isolation unit test が pass
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass
- [ ] `cargo doc -p ruviform-plugin-host --no-deps` が warning 0
