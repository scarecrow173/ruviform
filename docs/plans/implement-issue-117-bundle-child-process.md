---
id: "PLAN-034NlJVJxcfEkAxcclU9Zw"
type: "plan"
status: "completed"
title: "Issue #117 実施計画 — L1 bundle 子プロセス（1 子プロセス N インスタンス）"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-117-wire-v2-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-bundle-e2e-tests.md"
      - type: "file"
        path: "docs/impl/ir/issue-117-bundle-child-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-clap/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-wire-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-proxy-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-runner-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/real-adapter-l1-e2e-tests.md"
      - type: "file"
        path: "docs/designs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/specs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/specs/issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/designs/issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/specs/engine-plugin-host-integration.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-117-bundle-child-process.md"
  implemented-by:
    - "docs/tasks/l1-wire-v2-codec-cutover.md"
    - "docs/tasks/l1-child-instance-split.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
    - "docs/tasks/l1-wire-v2-docs-sweep.md"
    - "docs/tasks/l1-bundle-child-final-verification.md"
  depends-on:
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/plans/issue-76-followup-l1-internal-cleanup.md"
    - "docs/plans/implement-issue-119-l1-wire-version-policy.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/designs/issue-122-shared-platform-process-ipc-eval.md"
    - "AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/discovery/issue-117-bundle-child-process-discovery.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  derived-by:
    - "docs/tasks/l1-wire-v2-codec-cutover.md"
    - "docs/tasks/l1-child-instance-split.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
    - "docs/tasks/l1-wire-v2-docs-sweep.md"
    - "docs/tasks/l1-bundle-child-final-verification.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-117-wire-v2-codec-tests.md"
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "docs/test-specs/issue-117-bundle-e2e-tests.md"
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #117 実施計画 — L1 bundle 子プロセス（1 子プロセス N インスタンス） Implementation Plan

> status: completed — Refs #117。全 6 task 完了。approved design（`docs/designs/issue-117-bundle-child-process.md`）+ accepted ADR（`docs/adr/issue-117-bundle-child-failed-granularity.md`）を正本として、L1 を「1 instance = 1 子」から「1 子 N instance（bundle）」へ移行する 6-task DAG を実装した。T1 wire v2 原子 cutover（`4b5627c`+`6b3797a` — `WIRE_VERSION`=2・instance envelope・`CreateInstance`/`DestroyInstance`・`InstanceLimit` tag 9・registry・argv v2・両 harness）。T2 host 構造再編（`92f31fa`+`468d294` — `L1Child`+`InstShared` split・demux 判定表・converge split・`Destroying` gate）。T3 pool + 隔離面（`13e6cec`+`58bf8e5` — `OutOfProcessIsolation{Dedicated,Bundled}` default Bundled・`Route::OutOfProcess` struct variant + pool）。T4 bundle 検証（`cec29ff`+`4d73ea0` — `l1_proxy_e2e` 100 tests・demux 判定表/inert/doorbell 全行 pin・dedicated 非回帰・rt_ bundle）。T5 docs sweep（`10561d3`）。T6 final verification（Full Gate 12 コマンド全 exit 0・AC1–AC12 証跡・status 同期 — `docs/impl/ir/issue-117-bundle-child-implementation.md`）。GitHub merge は lead の authoritative `gh` のみ — 本計画は PR merged / issue closed の主張を含まない。

## Goal

L1 の spawn/lifecycle/wire 契約を wire v2（`WIRE_VERSION` 1→2 一括 bump + instance envelope + `CreateInstance`/`DestroyInstance` 動詞 + `HostError::InstanceLimit`）へ改訂し、host proxy を子共有化（`Inner.instances`・driver/reader demux・Failed 粒度 split・pool lifecycle）することで、`OutOfProcessIsolation::Bundled`（default）のもと 1 子プロセスに N 個の独立 `PluginHandle` を実現する。`Dedicated` は同一機構の capacity-1 経路として隔離 opt-in を残す。

## Non-Goals

- shared child I/O 多重化・shared reaper（Issue #120）、shared IPC backend（Issue #122）、video/NONRT shared-mem layout（Issue #118）、GUI 子への bundle 適用（#115 は dedicated 前提・ADR L76 どおり bundle 非対象）
- instance-scoped fault-injection 語彙の fixture 追加（design L388 — 子 scope 注入 + demux 単体検証で代替）
- `HostNotification::BundleFailed` 新 variant・pool key をまたぐ子共有・route teardown の keep-alive（全て design の deferred 行）
- wire v1 との共存・version 選択（#119 方針どおり単一最新版のみ・wholesale bump）
- 歴史 record（#76 spec/design・旧 plan/task/test-spec の当時記述）の書き換え — design L363 の grep 例外ポリシーどおり historical 文書は現行契約の正本ではない

## Scope and Split Boundary

単一 plan・6 task。分割軸は「コンパイル可能な最小整合単位」で、最終形は design の inventory（L336-364）に厳密対応:

- **wire v2 cutover は原子上限（T1）** — `Message` 4 payload kind への `instance` field 追加は enum の breaking change であり、v1/v2 中間のコンパイル可能状態が存在しない。consumer 6 file（wire.rs・export/lib.rs・l1/mod.rs・l1-test-worker.rs・l1_runner_e2e.rs・l1_proxy_e2e.rs）は同一 task 内で v2 へ揃える必要がある。さらに bootstrap 契約（argv `--ruviform-*` → `CreateInstance` payload）も host・runner・worker で同時に切り替わる必要があるため（片側だけ v2 化すると proxy E2E が不整合になる）、T1 は「全系統を最終 v2 形へ一発で持っていく」単位で、registry 化・argv 除去・`InvalidHandle` を含む
- **構造再編は cutover の後（T2）** — `Shared`/`Inner` → child 共有 + `InstShared` の split・`L1Child`・demux 判定表・converge split・`Destroying` gate・teardown は wire v2 上でしか意味を持たず、plugin-host crate 内で完結する独立単位
- **公開面 + pool は構造の後（T3）** — `Route::OutOfProcess` struct variant・`OutOfProcessIsolation`・pool 選択・destroy→`DestroyInstance` 写像は `L1Child`/demux 機構が前提
- **bundle 検証は機構完成後（T4）**、**docs/metadata sweep（T5）**、**final gate + closeout（T6）** の直鎖。T5 は T1-T3 の事実確定後なら T4 と並行可能（doc file のみ・code file と非交差）

## Files / Ownership

| task | owner crate | primary files | scope |
| --- | --- | --- | --- |
| T1 `l1-wire-v2-codec-cutover` | plugin-core + export + host(minimal) + harness | `wire.rs`, `boundary.rs`, `export/src/lib.rs`, `export/src/shm.rs`(doc), `l1-test-worker.rs`, `l1/mod.rs`, `composite.rs`(pin), `l1_runner_e2e.rs`, `l1_proxy_e2e.rs` | wire v2 codec + runner registry + worker registry + host 単一 instance v2 + MiniHost v2 + registry E2E |
| T2 `l1-child-instance-split` | plugin-host(l1) | `l1/mod.rs`, `l1/data.rs`(doc), `l1/shm.rs`(doc), `composite.rs`(call-site + `test_fixture` 移行) | `Shared`/`Inner`→`L1Child`+`InstShared` split、driver/reader demux、converge split、`Destroying` gate、install/uninstall、last-ref teardown |
| T3 `l1-route-pool-and-isolation-surface` | plugin-host(composite/lib) | `composite.rs`, `lib.rs` | `OutOfProcessIsolation` 公開 enum・`Bundled` default・`Route` struct variant + pool・instantiate/destroy 写像・pool lifecycle |
| T4 `l1-bundle-e2e-and-granularity-verification` | plugin-host(tests) | `l1_proxy_e2e.rs`, `l1/mod.rs`(unit test module) | bundle E2E・decision-table/inert/doorbell 検証・dedicated 非回帰・rt_ bundle |
| T5 `l1-wire-v2-docs-sweep` | docs/metadata | README×3・runner bin doc×3・adapter test stale comment×2・`export/shm.rs` doc・`export/Cargo.toml`・test-spec×4 verify/reconcile・親文書 addenda×5・literal sweep | design inventory L336-364 の docs 行全量 |
| T6 `l1-bundle-child-final-verification` | 全列 | workspace 全体・`docs/impl/ir/` | Full Gate・AC1-AC12 証拠・IR・status 同期 |

cross-crate 依存の形: T1 が 4 crate（core→host→export→tests）の atomic surface、T2/T3/T4 は plugin-host 内で直列、T5 は doc-only で T4 と並行可、T6 が収束点。

## Ordering Rationale

1. **wire v2 codec（plugin-core）が基盤** — 動詞・envelope・error・version は全下流の前提であり、かつ enum field 追加の breaking change として v1/v2 中間状態を持てない（wholesale bump 必須）
2. **全 surface の同時更新** — design L336-364 の inventory（wire/boundary/runner/worker/host l1/composite/MiniHost/proxy harness + doc surface）は同一 Issue・事実上同一 compile unit 内で揃える。T1 は「codec + 全 consumer の最終 v2 形」を 1 unit で落とす
3. **その後 host 構造再編（T2）と runner registry** — runner registry は T1 内に含まれる（v2 runner に singleton 版が存在しないため）。host 側の pool/`L1Child`/demux/converge split/`Inner.instances` が T2
4. **pool + 公開隔離面（T3）→ bundle 検証（T4）→ docs sweep（T5）→ gate（T6）**

crate 別並行性: T1 内で「child 側半分（codec+export+MiniHost+worker）」と「host 側半分（l1/mod.rs+proxy harness）」は merge 前 branch 上では並行作業可能だが、コンパイル可能 commit は両者の union でのみ成立する。T4（host tests）と T5（docs）は完全に独立ファイル集合で並行可能。

## Task DAG

```
T1 l1-wire-v2-codec-cutover          (atomic foundation — AC1/AC6/partial AC7)
  └─ T2 l1-child-instance-split      (host l1 — AC3/AC4/AC5/AC11/AC12 mechanism)
       └─ T3 l1-route-pool-and-isolation-surface  (composite+lib — AC2/AC9/AC11 surface)
            ├─ T4 l1-bundle-e2e-and-granularity-verification  (AC2-AC5/AC7/AC8/AC9/AC11/AC12)
            └─ T5 l1-wire-v2-docs-sweep            (docs/metadata — T3 以後・T4 と並行可)
                 └─ T6 l1-bundle-child-final-verification     (AC10 + 全 AC 収束)
```

| task | one-line scope |
| --- | --- |
| T1 `l1-wire-v2-codec-cutover` | `WIRE_VERSION`→2・instance envelope・`CreateInstance`/`DestroyInstance`・`InstanceLimit` tag 9 を codec 化し、runner/worker registry・argv v2・host 単一 instance v2・両 harness・MiniHost bundle runner E2E まで原子 cutover する |
| T2 `l1-child-instance-split` | `Shared`/`Inner` を child 共有（`L1Child`）+ `Inner.instances`→`InstShared` へ split し、driver multi-drain・reader demux 判定表・converge split・`Destroying` gate・install/uninstall・last-ref teardown を実装する |
| T3 `l1-route-pool-and-isolation-surface` | `OutOfProcessIsolation{Dedicated,Bundled}`（`#[non_exhaustive]`・default Bundled）・`Route::OutOfProcess` struct variant + pool・instantiate/destroy 写像・pool lifecycle・`lib.rs` 再公開を実装する |
| T4 `l1-bundle-e2e-and-granularity-verification` | bundle E2E（N instance 1 子・crash fan-out・kill_child・`InstanceLimit`・shutdown-on-empty・dedicated 非回帰・rt_）と demux 判定表/inert/doorbell の合成 frame 検証を行う |
| T5 `l1-wire-v2-docs-sweep` | design L336-364 の doc inventory 全量（README・bin doc・Cargo.toml comment・shm doc・adapter test stale comment・test-spec×4 の実装 verify/reconcile・親文書 addenda×5・literal sweep）を更新する |
| T6 `l1-bundle-child-final-verification` | Full Gate（fmt-check/check/lint/test/test-rt/bench-smoke/doc-audit/cargo doc）・AC1-12 証拠収集・IR・status 同期で closeout する |

並行 group: `{T1}` → `{T2}` → `{T3}` → `{T4 ∥ T5}` → `{T6}`。T1 内の child-side substream（core codec + export runner + worker + MiniHost）と host-side substream（l1/mod.rs + proxy harness）は merge 前のみ並行可。

## Test Oracles

| test-spec | role |
| --- | --- |
| `issue-117-wire-v2-codec-tests.md`（新規） | v2 増分 oracle: envelope `[seq][instance][rest]`・新動詞 payload bounds・scope 判定表・`InstanceLimit` tag 9 両方向 pin・`WIRE_INSTANCE_NONE`/`WIRE_MAX_INSTANCES` |
| `issue-117-bundle-failed-granularity-tests.md`（新規） | ADR 判定表 oracle: 子 scope vs instance scope vs inert vs non-event・fan-out・`{None,None}`・`Destroying` doorbell window・mid-Create/Destroy 死亡意味論 |
| `issue-117-bundle-e2e-tests.md`（新規） | bundle 制御面 oracle: pool 選択・隔離 surface・`InvalidHandle`/`InstanceLimit`/dup/UnknownPluginId→Ipc・teardown/rollback・live_count・handle reuse |
| `plugin-l1-wire-codec-tests.md`（v2 更新適用済み） | 既存 codec oracle の v2 版（`WIRE_VERSION`=2・動詞表・non-goal 行）— T5 で実装着地と verify/reconcile |
| `plugin-l1-proxy-contract-tests.md`（v2 更新適用済み） | per-child proxy 形状・handshake +CreateInstance・argv v2・Failed 粒度 pointer — T5 で verify/reconcile |
| `plugin-l1-runner-e2e-tests.md`（v2 更新適用済み） | argv v2・registry dispatch・`CreateInstance` 検証・`InvalidHandle` flagged reply — T5 で verify/reconcile |
| `real-adapter-l1-e2e-tests.md`（v2 更新適用済み） | argv/handshake v2 追従・bundle route notification note — T5 で verify/reconcile |

## Verification Matrix（spec AC1–AC12 → task/test-spec）

| AC | task | 検証 |
| --- | --- | --- |
| AC1 v2 codec round-trip/異常拒否 | T1 | wire unit test + `issue-117-wire-v2-codec-tests` + 既存 codec spec 更新行 |
| AC2 bundle 制御面（pool reuse・独立 handle） | T3/T4 | PluginHost 経由 E2E（`issue-117-bundle-e2e-tests`） |
| AC3 `(instance,seq)` 相関・no crosstalk | T2/T4 | demux unit + bundle E2E |
| AC4 Failed 粒度（判定表・inert・post-Shutdown） | T2/T4 | `issue-117-bundle-failed-granularity-tests` |
| AC5 InstanceFailed fan-out 通知 | T2/T3/T4 | kill bundle → N 通知 E2E + per-instance `{None,None}` |
| AC6 runner registry + defined rejection | T1 | MiniHost multi-instance E2E（`l1_runner_e2e` + `issue-117-bundle-e2e-tests`） |
| AC7 dedicated 非回帰 | T1/T4 | 既存 suite green + `Dedicated` route E2E |
| AC8 RT 契約保存 | T4 | `rt_` bundle leg テスト（`assert_no_alloc`） |
| AC9 上限/handle reuse/live_count | T1/T3/T4 | `InstanceLimit` Instantiate・Destroy→Create 同値 reuse・stale doorbell |
| AC10 全 Gate + doc audit | T6 | `mise run` 系列 + `doc_status.js audit` |
| AC11 pool/child lifecycle（teardown・rollback・eviction） | T2/T3/T4 | shutdown-on-empty・fresh-spawn vs shared-child failure・last-ref teardown |
| AC12 in-flight lifecycle crash（mid-Create/Destroy 死亡） | T2/T4 | `Err(Ipc)` no-notify / notify+`Ok` 分岐 |

## Risks / Readiness

| risk | 対処 |
| --- | --- |
| demux 判定表の行漏れ（誤った子 scope 収束 or 静黙 inert の取り違え） | design L304-320 表を single source とし、判定ロジックを reader 内 1 関数に集約 + 合成 frame unit test で全行 pin |
| per-instance converge が「兄弟を殺す」誤爆 | `converge_instance(i)` は `instances` map entry のみを対象・`Shared` の子全体 state（terminal/eof）を変更しないことを invariant 化 |
| `Destroying` gate の doorbell window で in-flight seq が残留 | `DestroyInstance` 送信 serialize → entry `Destroying` → in-flight 完了を待機 → reply で撤去・残 seq は `mark_failed` 解放の順序を実装し、kill 中 doorbell 挙動を E2E pin |
| pool 選択で dead-lock（`get_mut` 中に child teardown） | selection predicate（`live_count<64 && !terminal && !shutdown_initiated`）を `Inner` 排他内で評価 + fresh-spawn 失敗時は child teardown 後に pool remove |
| `CreateInstance` 失敗時の子健全性判断 | fresh-spawn 失敗→子ごと teardown（兄弟なし）・shared-child 失敗→instance entry rollback のみ（design L249 どおり判断軸を分離） |
| argv v2 で adapter runner（clap/vst3）が旧 flag を受けたまま | T1 で bin doc・README・`export/Cargo.toml` comment を揃え、T5 で literal sweep（`--ruviform-` grep） |
| `instance=0` 以外の mint 未実装のまま `Dedicated` が capacity-1 を逸脱 | T1 では `instance:0` 固定（子=1 instance）。T2 で lowest-free mint を導入し、T3 で pool 経由の N>1 に拡張 |
| 既存 flake（`l1_proxy_e2e` 負荷依存）が bundle 列で再発 | 既知 flake は孤立 rerun で切り分け。bundle 検証は deterministic な demux 合成 + kill 注入を中心に |

## Rollback Strategy

- **commit 単位**: T1 が 1 つの「v2 cutover commit」を形成（task 内の action は論理分割だが buildable な commit 境界は union）。各 task の最後は `mise run check`/`lint`/`test`/`test-rt` green を必須とし、red なら同 task 内で完結する revert（cutover 前 = `d71bdd0` 基点への `git revert`）を選択
- **構造段階**: T2/T3 の bundle 機構は `Dedicated` 経路の非回帰で回帰境界を守る — bundle に欠陥が残る場合、公開面で `Bundled` default を `Dedicated` へ戻す 1 行 revert が最終手段（ただし design 上 default=Bundled は確定契約であり、rollback は欠陥封じ込め用の temporary 手段としてのみ使用）
- **pool 無効化**: `Route::OutOfProcess` の pool field は capacity-1（Dedicated）としても動作する構造のため、pool 選択ロジック単体を revert しても wire/runner 層は維持可能

## Implementation Sequence

- [ ] T1 `l1-wire-v2-codec-cutover` — codec + 全 consumer v2 化（原子）
- [ ] T2 `l1-child-instance-split` — host l1 構造再編
- [ ] T3 `l1-route-pool-and-isolation-surface` — pool + 公開隔離面
- [ ] T4 `l1-bundle-e2e-and-granularity-verification` — bundle E2E + 判定表検証（T5 と並行可）
- [ ] T5 `l1-wire-v2-docs-sweep` — doc/metadata inventory 全量
- [ ] T6 `l1-bundle-child-final-verification` — Full Gate + closeout

## Dependencies and Risks

- 前提: `docs/designs/issue-117-bundle-child-process.md`（approved）・`docs/adr/issue-117-bundle-child-failed-granularity.md`（accepted）・`docs/specs/issue-117-bundle-child-process.md`・`docs/specs/issue-119-l1-wire-version-policy.md`（wholesale bump 方針）
- T1 の原子性は「enum field 追加 = 全 consumer 同時更新」が原因 — task 分割不能の構造的制約であり、計画上の deviation ではない（実装上の最小型）
- 検証列は全 task 共通: `mise run fmt-check` → `check` → `lint` → `test` → `test-rt`。T6 で `bench-smoke` + `cargo doc -p {core,host,export} --no-deps` + doc audit を追加。さらに opt-in lane `mise run test-l1-real-plugin` + `mise run lint-l1-real-plugin` — real clap/vst3 runner bin・adapter E2E は `required-features = "host"` で default gate の対象外のため、T5（lint lane）と T6（test lane）で個別に実行する
- `unsafe` 新規導入は設計上不要（shm view 既存機構を再利用）。`#[non_exhaustive]` は `OutOfProcessIsolation`・wire 新動詞 enum 関連で必須

## Task Breakdown

- `docs/tasks/l1-wire-v2-codec-cutover.md` — T1
- `docs/tasks/l1-child-instance-split.md` — T2
- `docs/tasks/l1-route-pool-and-isolation-surface.md` — T3
- `docs/tasks/l1-bundle-e2e-and-granularity-verification.md` — T4
- `docs/tasks/l1-wire-v2-docs-sweep.md` — T5
- `docs/tasks/l1-bundle-child-final-verification.md` — T6

## Verification

- [ ] `mise run fmt-check` が差分なしで終了する
- [ ] `mise run check`・`mise run lint`・`mise run test`・`mise run test-rt` が全て成功する
- [ ] `mise run bench-smoke`・`mise exec -- cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps` が warning 0 で完了する
- [ ] `mise run lint-l1-real-plugin`（T5）・`mise run test-l1-real-plugin`（T6）が pass — `required-features = "host"` で default gate の対象外の real-adapter lane
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す
- [ ] spec AC1–AC12 全行が検証 matrix の task/test-spec に対応付けられている
- [ ] `grep -rn "WIRE_VERSION" crates/` で `1` 残存が historical 注記以外に存在しない・`grep -rn '\-\-ruviform-' crates/` が `l1-test-worker` 等の「受信したら usage reject」行以外に残らない

## Open Questions / Gaps

- **T1 の原子性**: prompt の「codec → host∥runner → harness → E2E」段階は enum envelope の breaking change により中間コンパイル状態が存在せず、T1 に統合した。host minimal v2（単一 instance=0・monolithic `L1Instance`）は T2 で完全 split される前提の中間構造であり、argv→`CreateInstance` bootstrap の同時切替も atomic 性の一部（runner/worker の片側だけ v2 だと proxy E2E が不整合になる）。この統合は構造的制約によるもので、design からの deviation ではない
- **`Dedicated` route の pool field**: `Route::OutOfProcess{spec, isolation, pool}` で `Dedicated` は `pool` を保持するが instantiate 時に新規 child を spawn するか否かは selection predicate で制御 — design の「dedicated も同 v2 経路・pool 非投入」を踏襲（capacity-1 semantics via same machinery）
- **demux の instance-scoped fault 注入欠如**: design L388 の deferred どおり、`converge_instance` の正当性は「合成 frame → demux 関数」unit test + `kill_child` E2E で代替する。`MiniHost` 側の per-instance dishonest-child fixture は作らない

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 branch・6 task 直列+T4/T5 並行・wire atomic cutover のため外部委任不要）
- Independent work streams: T4（host tests）と T5（docs）は T3 完了後に並行可
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 主眼は T1 の原子 cutover 境界（argv bootstrap + registry を T1 に含める妥当性）と、T2 の `Shared`/`Inner`/`InstShared`/`L1Child` split が design L199-250 の責務写像と一致するか。demux 判定表（T2 実装 + T4 検証）は design L304-320 を逐語 oracle として使う。Failed 粒度の contract は ADR が正本 — spec/design の記述と齟齬がある場合 ADR が優先する。
