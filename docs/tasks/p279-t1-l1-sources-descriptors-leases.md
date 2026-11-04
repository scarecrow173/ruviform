---
id: "TASK-034RTukbgTWDcKGHBdKYYN"
type: "task"
status: "done"
title: "P279-T1 l1/sources.rs 4 Descriptor + AudioSourceLeases + slot state + mod tests"
created: "2026-11-04"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/279"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/sources.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-279-plugin-host-counter-source.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-279-plugin-host-counter-source.md"
  blocks:
    - "docs/tasks/p279-t2-create-destroy-registrar-wiring.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/121"
  defers: []
  deferred-by: []
---

# P279-T1 l1/sources.rs 4 Descriptor + AudioSourceLeases + slot state + mod tests

## Work

`crates/ruviform-plugin-host/src/l1/sources.rs`（新規・`pub(crate)`）に
Domain 宣言の 4 `MetricDescriptor` と登録結果 owner `AudioSourceLeases`
を実装する。`AudioCounterReaders` と `&dyn ObservationRegistrar` を受け
`MetricSource::new(descriptor, ObservationScope::Instance, Counter(reader))`
で逐次登録し、結果を `Registered`/`Refused(RegistrationError)`/`Unattached`
の slot 状態に記録する bundle を返す。`retire()` は全 `Registered` lease を
`lease.retire()` して `RetiredSource` ticket を `retired` Vec に退避する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-279-plugin-host-counter-source.md`
- `docs/designs/issue-279-plugin-host-counter-source-registration.md`
- `docs/specs/issue-279-plugin-host-counter-source-registration.md`

## Dependencies

- 先行: なし（設計の l1/sources.rs のみ新規・他ファイルに触れない）。
- `AudioCounterReaders` は `l1/mod.rs` で定義済み（4 `CounterReader`）。
- `ruviform-observability` の `MetricSource`/`MetricDescriptor`/
  `ObservationScope`/`ObservationRegistrar`/`RegistrationLease`/
  `RetiredSource`/`RegistrationError`（OBS-02・着地済み）。

## Affected Files

- `crates/ruviform-plugin-host/src/l1/sources.rs` — 新規。
- `crates/ruviform-plugin-host/src/l1/mod.rs` — `mod sources;` 宣言の 1 行
  （T-2 と共有するが T-1 のコンパイルに必要）。

## Done When

- [x] 4 `MetricDescriptor` が `kind=Counter`・`aggregation=Cumulative`・
  `unit=Count`・`scope=Instance`・`confidentiality=Internal`・
  `origin="ruviform-plugin-host::l1"`・`warmup=None` で供給される。
- [x] `passthrough_submits` のみ `SnapshotOnly`、他 3 本は
  `OnIncrease{min_delta:1, min_interval:1s, impact:High}`。
- [x] `register`/`unattached`/`retire`/`registered_count`/`identities`/
  `retired_lifecycles` が実装され、`Refused` は他 slot を巻き込まない。
- [x] mod tests が in-crate test registrar（正常/エラー挿入）で
  register/unattached/Refused/retire/ticket を決定的に検証する。

## Verification

- [x] `cargo check` でコンパイルが通る（T-2 の呼出し側までの独立検証は
  `mod sources` + mod tests の自己完結で可能）。
- [x] mod tests が pass（in-crate test registrar による登録呼出し回数・
  slot 状態・lifecycle 遷移の検証）。
- [x] `RegistrationLease`（`Send` のみ）が slot bundle 内で所有できること
  （Sync 非要求・facade 保持設計）。
