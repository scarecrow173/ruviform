---
id: "PLAN-034RTsBYMsfKWtQaNUpvnl"
type: "plan"
status: "approved"
title: "Implement Issue #279 OBS-04 plugin-host counter source registration"
created: "2026-11-04"
updated: "2026-11-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/279"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/test-specs/issue-279-plugin-host-counter-source-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-277-observability-core-source.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/designs/issue-279-plugin-host-counter-source-registration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-279-plugin-host-counter-source-tests.md"
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
  deferred-by: []
---

# Implement Issue #279 OBS-04 plugin-host counter source registration Implementation Plan

## Goal

design `DESIGN-034RTpdouoLLSINNczwa3c` どおりに `ruviform-plugin-host` の
L1 audio mint 経路へ OBS-02 `ObservationRegistrar` 注入・4 Counter の Domain
宣言登録・`RegistrationLease` 寿命配線を実装し、spec AC-1〜AC-7 を `mise`
検証と決定的試験で証明する。

## Scope

1 本の実装ストリーム（`l1/sources.rs` → `l1/mod.rs` 配線 → `composite.rs`
注入面 → 試験・README は同一 critical path）。OBS-07 Collector / OBS-03
tracing_export の実装、L0/video 経路の登録、新規クレート・外部依存追加、
`OutOfProcessSpec` へのフィールド追加は含まない。

## Upstream Documents

- `docs/specs/issue-279-plugin-host-counter-source-registration.md`（approved・AC-1〜AC-7）
- Issue #279（完了条件の正本）・親 Issue #275（統合設計正本）・Issue #121（4 Counter 意味表）

## Design Inputs

- `docs/designs/issue-279-plugin-host-counter-source-registration.md`（approved）

## File Map

- `crates/ruviform-plugin-host/src/l1/sources.rs` — 新規。4 `MetricDescriptor`
  （Domain 宣言）・`AudioSourceLeases`（slots + retired ticket）・
  `AudioSourceSlot`（Registered/Refused/Unattached）・mod tests
- `crates/ruviform-plugin-host/src/l1/mod.rs` — `pub(crate) mod sources`・
  `L1Child::create_instance` に registrar 引数追加・Live flip 後の登録・
  `InstanceParts`/`L1Instance` に `source_leases` フィールド・`destroy` の
  retire 挿入・試験 harness の更新
- `crates/ruviform-plugin-host/src/l1/data.rs` — `create_instance` 呼出しの
  更新のみ（RtLeg/writer は不変）
- `crates/ruviform-plugin-host/src/composite.rs` — `observation_registrar`
  フィールド + `set_observation_registrar`・`instantiate` 両 arm と
  `instantiate_bundled`/`create_on_fresh` への経路・harness 試験の更新
- `crates/ruviform-plugin-host/README.md` — OBS-04 登録経路の 1 節（必要時）
- `crates/ruviform-plugin-host/Cargo.toml` — 変更不要（`ruviform-observability`
  は既存依存）

## Implementation Sequence

- [ ] T-1: `l1/sources.rs`（Descriptor・Leases・slot 状態機械 + mod tests）
- [ ] T-2: `l1/mod.rs` 配線（create_instance 登録・InstanceParts/L1Instance・
  destroy retire・harness 更新）+ `composite.rs` 注入面と試験

## Dependencies and Risks

- `RegistrationLease` は `Send` のみ（`Sync` 非保証）→ `InstShared`/`Shared`
  （driver/reader thread 共有）に置かず facade `L1Instance` に保持する設計
  制約。コンパイルが担保。
- `remove_instance` 内の residual doorbell flush が `results_dropped` に
  最終 write し得る → retire はその後に置く（順序不変条件・test-spec が pin）。
- 同名 key・異 Instance は `scope=Instance` + `SourceId` 区別に委ね、
  `KeyConflict` を返す Collector 実装には `Refused` slot の typed degrad
  （誤 Series fallback なし）。
- 他並行 worker（#277 周辺の doc-graph blocker・Cargo.toml 編集の可能性）→
  最終検証前に `git merge origin/main` で統合する。

## Task Breakdown

- `docs/tasks/p279-t1-l1-sources-descriptors-leases.md`
- `docs/tasks/p279-t2-create-destroy-registrar-wiring.md`

## Verification

- [ ] `mise run fmt` / `mise run fmt-check` / `mise run check` / `mise run lint` /
  `mise run test` / `mise run test-rt` が全 pass
- [ ] 既存 `audio_counters_*`・`rt_`・lifecycle/endpoint/e2e 試験が無改変で pass
- [ ] `l1/sources.rs` mod tests: register 4 slot Active・per-slot Refused・
  unattached・retire→ticket 退避
- [ ] `create_instance`/`PluginHost` 試験: mint→登録・registrar None→未登録・
  error→degrad 継続・failed-create→未登録・destroy→retire
- [ ] `git merge origin/main` 後の最終検証（共有ファイル統合確認）

## Open Questions / Gaps

- なし（Issue 本文・親 #275・OBS-02 契約・4 Counter 意味表は設計で確定済み。
  warmup `Duration` 化・Collector 側 Removed 観測は Deferred に分離済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 worker の dispatch。task 分割は
  同一ファイル編集のため逐次実行）
- Independent work streams: なし（T-1 → T-2 は同一 critical path・同じ
  `l1/mod.rs` を編集）
- User approval required before dispatch: no（既承認 dispatch・Coordinator
  からの直接指示で実行中）
- If approved: discover available implementation and delegation capabilities
  in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph
  and verification matrix.

## Review Handoff

- Reviewer focus: (a) `create_instance` の Live flip 後登録と rollback 経路
  （登録が走らないこと）、(b) `destroy` の quiescence → retire 順序、
  (c) RT path（`process`/`RtLeg`/`RtLegCounters`）に一切の registrar/lease
  handle が届かないこと、(d) per-slot `Refused` で plugin 継続（fallback
  なし）の typed degradation。
- 前提: OBS-07 Collector 不在のため Source 登録の外部効果は test registrar
  で検証。`KeyConflict` 等を返す Collector の実動作は #289 着地後の
  OBS-06 gate で確認。
