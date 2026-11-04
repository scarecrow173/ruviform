---
id: "TASK-034RTukTevxIOK0EVaR8A3"
type: "task"
status: "done"
title: "P279-T2 create_instance/destroy registrar wiring + PluginHost注入面 + tests"
created: "2026-11-04"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/279"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-279-plugin-host-counter-source.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-279-plugin-host-counter-source.md"
    - "docs/tasks/p279-t1-l1-sources-descriptors-leases.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-279-plugin-host-counter-source-registration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/121"
  defers: []
  deferred-by: []
---

# P279-T2 create_instance/destroy registrar wiring + PluginHost注入面 + tests

## Work

`AudioSourceLeases`（T-1）を mint/destroy 経路に配線し、`PluginHost` に
registrar 注入面を作る。`L1Child::create_instance` に `Option<&dyn
ObservationRegistrar>` 引数を追加し、`Creating → Live` flip 成功時のみ
reader clone で登録（lock 解放後）。`InstanceParts`/`L1Instance` に
`source_leases` フィールドを足し、`L1Instance::destroy` の
`remove_instance` 完了後に `retire` を挿入する。`composite.rs` に
`observation_registrar` フィールド + `set_observation_registrar` を追加し、
`instantiate` 両 arm・`instantiate_bundled`・`create_on_fresh` へ配線する。
全 `create_instance` 呼出しと既存試験 harness を新シグネチャに合わせる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-279-plugin-host-counter-source.md`
- `docs/designs/issue-279-plugin-host-counter-source-registration.md`
- `docs/specs/issue-279-plugin-host-counter-source-registration.md`

## Dependencies

- T-1（`AudioSourceLeases`/`MetricSource` 呼出し）。
- `L1Child::create_instance` / `InstanceParts` / `L1Instance` /
  `destroy` / `remove_instance` / `drain_endpoint_channels` の既存経路。
- `PluginHost` の `diag` 注入規約（同じ引数経路で registrar を渡す）。
- `Supervisor::attempt` → `HostCommand::Instantiate` の既存経路
  （host 保持 registrar が自動的に効く）。

## Affected Files

- `crates/ruviform-plugin-host/src/l1/mod.rs` — `create_instance`・
  `InstanceParts`・`L1Instance`・`destroy`・試験 harness。
- `crates/ruviform-plugin-host/src/l1/data.rs` — `create_instance` 呼出し
  の引数追加のみ（RtLeg/writer 不変）。
- `crates/ruviform-plugin-host/src/composite.rs` — `PluginHost`
  フィールド/setter・`instantiate`/`instantiate_bundled`/`create_on_fresh`
  ・試験 harness。
- `crates/ruviform-plugin-host/src/lib.rs` — `set_observation_registrar`
  を公開 API doc に追記（必要時）。

## Done When

- [x] `create_instance` は `Live` flip 成功後のみ `AudioSourceLeases::
  register` を呼び、失敗/rollback 経路では呼ばない。
- [x] `InstanceParts`・`L1Instance`・`from_parts`・全呼出しが
  `source_leases` を保持し、`destroy`/`Drop` が `remove_instance` 後に
  `retire` する。
- [x] `PluginHost::set_observation_registrar(Option<Arc<dyn ObservationRegistrar>>)`
  が `instantiate` 両 arm・bundled/fresh 両経路に registrar を渡す。
- [x] registrar 未接続（`None`）で既存動作と同値（`Unattached` slot・
  `register` 非呼出）。
- [x] registrar エラー（stopped/capacity/conflict 等）で mint が成功継続
  し `Refused` slot が記録される。
- [x] RT path に registrar/lease handle が届かない（`RtLeg`/`RtLegCounters`/
  `process`/`InstShared::push_result` は不変）。
- [x] 同名 key・異 Instance は `SourceId`/`SourceGeneration` で区別され、
  re-mint で新 `SourceIdentity`。
- [x] 既存 `audio_counters_*`・`rt_`・endpoint/lifecycle/e2e 試験が無改変で
  pass し、新 harness 試験が mint→登録・error→degrad・destroy→retire・
  failed-create→未登録を検証する。

## Verification

- [x] `mise run check` / `lint` / `test` / `test-rt` が pass。
- [x] 新試験が test-spec の Examples（registrar 接続・未接続・Refused・
  destroy 順序・failed-create・2 mint 同名 key）を pin する。
- [x] `RegistrationLease` が `Shared`/`InstShared` に入らない（`Sync` 要求を
  満たさないことの compile-time 担保は build で確認）。
