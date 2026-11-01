---
id: "PLAN-034QedYKnRFb63F40E0Mb6"
type: "plan"
status: "approved"
title: "Implement Issue #184 M1 ruviform-graph crate: typed IR + validator + PoC"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/184"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
    - "docs/adr/typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/discovery/issue-184-bus-port-graph-ir-crate.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
    - "docs/adr/typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md"
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-184-m1-typed-graph-ir-representation-multi-bus-topology-validation-failure.md"
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/185"
  deferred-by: []
---

# Implement Issue #184 M1 ruviform-graph crate: typed IR + validator + PoC Implementation Plan

## Goal

design `DESIGN-034QdReVJaFxlQTjmr5oy5` どおりに新 crate `ruviform-graph`（typed multi-bus Port/Connection Graph IR + 非 RT Validator + GraphProject/StableHandleMap）を実装し、ex_001〜ex_006 PoC 受入テストと既存スイート不変共存を実測で証明する。

## Scope

1 本の実装ストリーム。新 crate 内部の型→検証→公開境界→PoC の依存は全て同一 critical path であり分割しない。M2/M4 領域（Compiler/Buffer/PDC/RT 実行、plugin 実接続）は含まない。

## Upstream Documents

- `docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md`（proposed・AC-1〜10）
- `docs/adr/typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md`（proposed・Option A + 構造決定 1-5）
- `docs/discovery/issue-184-bus-port-graph-ir-crate.md`（棚卸し正本）

## Design Inputs

- `docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md`（approved — 型モデル・Validator 規則 1-7・GraphProject 契約・Kahn topo tie-break・ex_001〜006 定義）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `Cargo.toml` (root) | `members` に `crates/ruviform-graph` 追記 | 構造層位置: domain foundations 後・engine 前 |
| `crates/ruviform-graph/Cargo.toml` | crate manifest（`lints`/`publish` workspace 継承、deps `audio-core`+`media-core` のみ） | 新規 |
| `crates/ruviform-graph/README.md` | 責務/層・許可依存/RT 契約（crate 規約） | 新規 |
| `crates/ruviform-graph/src/lib.rs` | crate doc + re-export | 新規 |
| `crates/ruviform-graph/src/ids.rs` | `NodeId`/`PortId`/`ConnectionId`/`GraphRevision`/`RuntimeNodeIndex`/`RuntimePortIndex`/`RuntimeConnectionIndex` | 新規 |
| `crates/ruviform-graph/src/port.rs` | `Direction`/`SignalDomain`/`PortRole`/`PortFormat`/`Multiplicity`/`PortCapability`/`Availability`/`Activation`/`PortDef`/`ChannelMap` | 新規 |
| `crates/ruviform-graph/src/graph.rs` | `NodeDef`/`NodeKind`/`NodeSpec`/`PortSpec`/`PortRef`/`EdgeDelay`/`ConnDef`/`Connection`/`GraphSpec`/`GraphBuilder`/`BuildError` | 新規 |
| `crates/ruviform-graph/src/validate.rs` | `ValidationError`/`ValidationErrors`/`Validator`/`ValidatedGraph`/`LatencyStatus` | 新規 |
| `crates/ruviform-graph/src/mapping.rs` | `StableHandleMap`（Kahn topo + 逆引き） | 新規 |
| `crates/ruviform-graph/src/project.rs` | `GraphProject`/`PublishedGraph`/`PrepareError` | 新規 |
| `crates/ruviform-graph/tests/poc_ex.rs` | ex_001〜ex_006 受入 | 新規 |
| `crates/README.md` | クレート索引 28→29 + 構造層節 | 更新 |
| `docs/impl/ir/…` 等の implementation record | 実装記録（graph の impl-record audit が要求する場合） | 後続 |
| `Cargo.lock` | 再生成 | `mise exec -- cargo generate-lockfile` |

## Implementation Sequence

- [ ] Step 1: `Cargo.toml` members 追記 + `crates/ruviform-graph/{Cargo.toml,README.md}` 骨格 + `src/lib.rs` crate doc。
- [ ] Step 2: `src/ids.rs` + `src/port.rs`（属性型・`Multiplicity` 検証・`PortFormat::domain()`）を実装。
- [ ] Step 3: `src/graph.rs`（`GraphBuilder` 採番・`GraphSpec` private 構築・`BuildError`）を実装。
- [ ] Step 4: `src/validate.rs`（規則 1-7 + SCC cycle 判定 + `ValidationErrors` 全件収集 + `ValidatedGraph`/`LatencyStatus`）を実装。
- [ ] Step 5: `src/mapping.rs`（Kahn+BTreeMap tie-break の `StableHandleMap`）+ `src/project.rs`（`GraphProject::prepare`）を実装。
- [ ] Step 6: `tests/poc_ex.rs` ex_001〜ex_006 を実装し `mise exec -- cargo test -p ruviform-graph --test poc_ex` で緑にする。
- [ ] Step 7: `crates/README.md` 索引更新 + `mise exec -- cargo generate-lockfile` + `mise exec -- cargo tree --locked -p ruviform-graph` で依存確認。
- [ ] Step 8: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` を実行し結果・exit code を記録。
- [ ] Step 9: `rg` で `ruviform-rt`/`RtContext`/`unsafe` 参照が graph crate に無いことを確認し記録。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | workspace lint（`pedantic`/`exhaustive_enums`）が `#[non_exhaustive]` 方針や builder API と衝突 | med | low | 既存 crate（`plugin-core` 等）の同パターンを写し lint 緑まで修正 |
| 2 | `AudioFormat`/`ChannelLayout` の API 面が設計想定と相違 | low | med | `cargo check` で早期検出・設計書どおり再利用で対処（`sample_format` field 名） |
| 3 | `Cargo.lock` 差分が既存 crate に波及 | low | low | `generate-lockfile` 後 `git diff Cargo.lock` で新規追加分のみを確認 |
| 4 | 破壊的変更なし（全 additive・既存 crate 不変）のため rollback は `git revert` で完結 | — | — | 不可逆 step なし |

## Task Breakdown

- T-1: Step 1-3（crate 骨格 + ID/port/graph 型 + builder）→ task doc 1 件
- T-2: Step 4-5（validator + mapping + project）→ task doc 1 件
- T-3: Step 6-9（PoC 受入 + 索引/lockfile + 全検証実行と記録）→ task doc 1 件

## Verification

- [ ] `mise exec -- cargo test -p ruviform-graph --test poc_ex` → exit 0（ex_001〜006 緑）
- [ ] `mise exec -- cargo tree --locked -p ruviform-graph --edges normal` → `audio-core`+`media-core` のみ・循環 0
- [ ] `mise run fmt-check` → exit 0
- [ ] `mise run check` → exit 0
- [ ] `mise run lint` → exit 0（M0 baseline の `ruviform-numeric` 2 error が main で修正済みか要確認; 修正済みなら緑、未修正なら新規 error が graph crate 由来でないことを区別して記録）
- [ ] `mise run test` → exit 0（既存スイート不変共存）
- [ ] `mise run test-rt` → exit 0（`rt_` フィルタ）
- [ ] `rg "ruviform_rt|RtContext|unsafe" crates/ruviform-graph/src` → 0 match

## Open Questions / Gaps

- `AudioFormat` の `PartialEq` 有無（layout/SR/sample 比較に必要）— 実装時に `cargo check` で確認。無ければ field 単位比較で実装。
- `ChannelLayout::channels()` の戻り型（`u16`/`usize`/`u8`）— `ChannelMap` の `u16` index と照合する際に実装で確定。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 worker・critical path が直列で分割の利益なし）。
- Independent work streams: なし（Step 1→9 直列）。
- User approval required before dispatch: yes（本 plan は inline で実行済みのため dispatch 不要）。

## Review Handoff

- 仮定: `AudioFormat`/`ChannelLayout` の API 面が design 記述どおり（`sample_rate`/`layout`/`sample_format`、`channels()` 存在）。実装時に検証。
- レビュー重点: Validator 規則 7（全グラフ SCC での cycle 判定）と `GraphProject` の revision 発行順（requested→validate→published）。
- 上流: spec（AC-1〜10）・ADR（Option A+決定1-5）・design（approved）。
