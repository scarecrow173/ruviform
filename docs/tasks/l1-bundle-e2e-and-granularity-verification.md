---
id: "TASK-034NlQnJQdVWzhM6WxhAM2"
type: "task"
status: "todo"
title: "L1 bundle E2E + Failed 粒度検証（decision table・inert・doorbell・fan-out・dedicated 非回帰）"
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
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/tasks/l1-child-instance-split.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
  blocks:
    - "docs/tasks/l1-bundle-child-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/plugin-host-composite-and-l1-e2e.md"
    - "docs/tasks/l1-proxy-control-plane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "docs/test-specs/issue-117-bundle-e2e-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
  defers: []
  deferred-by: []
---

# L1 bundle E2E + Failed 粒度検証（decision table・inert・doorbell・fan-out・dedicated 非回帰）

## Work

T2 機構 + T3 pool 上で bundle の外部観測契約を検証する。instance-scoped fault-injection 語彙は design L287 で deferred 済みのため、`converge_instance` は demux 合成 frame unit test + `kill_child` 子 scope E2E で代替する。

1. `l1_proxy_e2e.rs` — bundle E2E を追加（`PluginHost` + `Bundled` route 経由、または `#[path]` harness の `pub(crate)` dedicated helper 経由で multi-`CreateInstance`）:
   - 1 子 N instance（2+ `Instantiate` → 同一 `L1Child`・各々独立 `PluginHandle`・独立 lifecycle/process 経路・per-instance segment+`RtLeg`）
   - `(instance, seq)` 相関・no crosstalk（各 instance の `BlockIn`/`Reply` が混線しない）
   - `kill_child` bundle → 全 live instance `Failed` + per-instance `InstanceFailed{handle, 同一 exit}` N 通知（fan-out）
   - `Err(InstanceLimit)` `Instantiate`（pool 満杯 + spawn 不可）→ `Err(InstanceLimit)` 伝搬
   - shutdown-on-empty: 最後の `Destroy` → 子が exit（`Shutdown`+grace+reap + pool evict）
   - `Failed`+live child の `Destroy` → `DestroyInstance` 送信 → `Ok`
   - mid-`DestroyInstance` 死亡 → `InstanceFailed` 通知 + `Destroy` `Ok`
   - dedicated 非回帰: `isolation: Dedicated` で `Route::OutOfProcess` が 1 子 1 instance として既存挙動と同じ（全 `l1_proxy_e2e` suite が green）
   - `rt_` bundle テスト: bundle 経路で `assert_no_alloc` + doorbell/staging 経由の process leg が `Failed` でない（`rt_` prefix 命名）
   - runner bin（export runner）への real-path bundle: `composite_e2e` 系で `Bundled` route → real runner 子へ multi-`CreateInstance`（`runner_bin_serves_real_gain_plugin` 等の bundle 版）
2. `l1/mod.rs` `#[cfg(test)]` — demux 判定表の合成 frame unit test を T2 の土台から拡張（本 task では E2E 側の対応関係を pin）:
   - 子 scope 終端（2nd Hello・`instance=NONE` Reply/BlockDone・`i≥64`・帰属不能 seq）→ `converge_child`
   - attributable instance-scope（live i への未知 `(i,seq)` BlockDone）→ `converge_instance(i)`（`Inner.terminal` 非変更を assert）
   - inert（registered-but-not-live・撤去済み `i`）→ 静黙 drop（`Inner` 非変更・`Failed` なし・通知なし）
   - post-`Shutdown` frame → non-event
   - per-command `Err(HostError)` → defined rejection（`Failed` 化しない）
   - `Destroying` doorbell window: `Destroying` 中の新 submit → `mark_failed`・in-flight `BlockDone` → 正常処理・reply → entry 撤去・残 seq `mark_failed` 解放
   - mid-`CreateInstance` 死亡 → 通知なし `Err(Ipc)`・entry 撤去
3. `l1-test-worker.rs` — bundle E2E で必要なら子 scope knob の bundle 動作を確認（fault 語彙は追加せず、既存 knob が bundle registry 下で動くことだけ検証）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（T4 — bundle 検証）
- [../designs/issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md) L180-204（判定表）・L236-289（検証方針・harness seam・deferred fault-injection）
- [../adr/issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)（判定表・notification 粒度の正本）
- [../specs/issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md) AC2-AC5/AC7/AC8/AC9/AC11/AC12
- [issue-117-bundle-failed-granularity-tests.md](../test-specs/issue-117-bundle-failed-granularity-tests.md)・[issue-117-bundle-e2e-tests.md](../test-specs/issue-117-bundle-e2e-tests.md)（本 task の oracle）

## Dependencies

- 先行タスク: [l1-route-pool-and-isolation-surface.md](l1-route-pool-and-isolation-surface.md)（pool + `Bundled` route が前提）・[l1-child-instance-split.md](l1-child-instance-split.md)（demux/converge 機構が前提）
- 並行可: [l1-wire-v2-docs-sweep.md](l1-wire-v2-docs-sweep.md)（doc-only・本 task の test file と非交差）

## Affected Files

- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` — bundle E2E・dedicated 非回帰・rt_ bundle・real runner bundle
- `crates/ruviform-plugin-host/src/l1/mod.rs` — demux 判定表 unit test 拡張（合成 frame → demux → state assert）

## Done When

- [ ] `PluginHost` + `Bundled` route で 1 子 N instance が成立し、各 handle が独立 lifecycle/process を持つ
- [ ] `kill_child` bundle で全 live instance `Failed` + N `InstanceFailed`（同一 exit）が通知される
- [ ] `Err(InstanceLimit)` `Instantiate` が `Err(InstanceLimit)` として surface へ伝搬する
- [ ] 最後の `Destroy` で shutdown-on-empty（`Shutdown`+grace+reap+pool evict）が動く
- [ ] `Failed`+live child の `Destroy`・mid-`DestroyInstance` 死亡・inert/doorbell/attributable/defined rejection が判定表どおりに動く
- [ ] `Dedicated` route で既存 `l1_proxy_e2e` suite が非回帰で green
- [ ] `rt_` bundle テストが `assert_no_alloc` を使い RT 契約を pin する
- [ ] real runner 子への bundle `CreateInstance` が `composite_e2e` で pass する

## Verification

- [ ] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` — bundle + dedicated 全 pass
- [ ] `cargo test -p ruviform-plugin-host rt_` — `rt_` bundle テストが `assert_no_alloc` で pass
- [ ] `cargo test -p ruviform-plugin-export --test l1_runner_e2e` — MiniHost bundle E2E が pass
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass
- [ ] 判定表の全行（子 scope/instance scope/inert/non-event/defined rejection）が unit または E2E で pin されている
