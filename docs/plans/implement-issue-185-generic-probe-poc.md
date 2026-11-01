---
id: "PLAN-034QibwCtM7qkntuVsjcKR"
type: "plan"
status: "approved"
title: "Implement Issue #185 Generic Probe PoC: typed::probe contract, chunk-pool RT capture, broker/collector"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/185"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
    - "docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/discovery/issue-185-generic-probe-observation-inventory.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
    - "docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
    - "docs/discovery/issue-185-generic-probe-observation-inventory.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-185-generic-probe-tests.md"
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  deferred-by: []
---

# Implement Issue #185 Generic Probe PoC: typed::probe contract, chunk-pool RT capture, broker/collector Implementation Plan

## Goal

design `DESIGN-034QhWjbVG7vgYciUpfE2b` どおりに `ruviform-audio::typed::probe`（capability 列挙・`ProbeBroker`/`ProbeDeck`/`RtProbe`/`ProbeCollector`・record 型群）とそれを支える compile/runtime の additive 拡張を実装し、spec AC-1〜AC-10 を mock graph PoC テストと `mise` 検証の実測で証明する。

## Scope

1 本の実装ストリーム（binding 集約 → runtime 拡張 → probe 契約 → テスト/証拠は全て同一 critical path・直列）。TimedEvent/Control payload、plugin 内部 stage、cross-session ID、Recording Writer 契約、旧 pull 型 graph への probe は含まない。

## Upstream Documents

- `docs/specs/issue-185-generic-probe-rt-capture.md`（approved・AC-1〜AC-10+AC-5b）
- `docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md`（accepted）
- `docs/discovery/issue-185-generic-probe-observation-inventory.md`（棚卸し正本）

## Design Inputs

- `docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md`（approved）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-audio-analysis/src/level.rs` | `ChannelLevel` channel 単位算出の抽出（`LevelMeter` 挙動不変） | 更新 |
| `crates/ruviform-audio/Cargo.toml` | `ruviform-audio-analysis`・`ruviform-observability` path 依存追加 | 更新 |
| `crates/ruviform-audio/src/typed/compile.rs` | `outputs`→`(PortId, slot)`・`effective_slot`・`latency_accum`/`own_latency` | 更新 |
| `crates/ruviform-audio/src/typed/io.rs` | `input()` が `effective_slot` を読む | 更新 |
| `crates/ruviform-audio/src/typed/runtime.rs` | `frame_cursor`・probe command drain・4 tap 点・`published_plan`・`RtState.probes` | 更新 |
| `crates/ruviform-audio/src/typed/probe.rs` | 契約型群・resolver・`ProbeBroker`・`ProbeDeck`/`RtProbe`・`ProbeCollector` | 新規 |
| `crates/ruviform-audio/src/typed/mod.rs` | `pub mod probe` + re-export | 更新 |
| `crates/ruviform-audio/tests/common/mod.rs` | `ProbePoint` 宣言 port 付き fixture helper | 更新 |
| `crates/ruviform-audio/tests/typed_probe.rs` | AC-1〜AC-9 受入（capability/mode/metrics/capture/metadata/fault） | 新規 |
| `crates/ruviform-audio/tests/rt_probe_alloc.rs` または `rt_` prefix テスト | `assert_no_alloc` 窓の alloc 0（全 mode） | 新規 |
| `docs/impl/ir/issue-185-generic-probe.md` | impl record・検証証拠・未検証範囲（AC-9/AC-10） | 新規 |
| `Cargo.lock` | 再生成 | `mise exec -- cargo generate-lockfile` |

## Implementation Sequence

- [ ] Step 1: `level.rs` の channel 単位 `measure` 抽出（`LevelMeter` はそれを呼ぶ形に揃える）+ `ruviform-audio/Cargo.toml` 依存追加。
- [ ] Step 2: `compile.rs` binding 規則集約 — `ScheduledNode::outputs` を `Vec<(PortId, usize)>` 化、`InputBinding::effective_slot` を compile 確定、`ScheduledNode::{latency_accum, own_latency}` を `apply_pdc` 結果から正本化。`io.rs`/`materialize`/`collect_boundaries`/既存テストの読み替え。
- [ ] Step 3: `runtime.rs` — `RtState.frame_cursor`・`published_plan` retain/accessor・probe command channel 2 系統（gen-scoped は activation・lifetime は毎 callback）・`ProbeDeck` の 4 tap 点・`free`/`chunks_out`/`telemetry` 配線。
- [ ] Step 4: `typed/probe.rs` — 契約型群（`#[non_exhaustive]` enum・`RecordMeta`・`ProbeError`）・capability resolver（物理解決先行）・`ProbeBroker`・`RtProbe`/`ProbeDeck`・`ProbeCollector`（`drain`/`recycle`/`status_mirror`）。
- [ ] Step 5: `tests/common` fixture の `ProbePoint` 宣言 port 対応 + `tests/typed_probe.rs`（AC-1〜AC-9 注入系を含む）+ `rt_probe_*` alloc 0 テスト。
- [ ] Step 6: ON/OFF 負荷差・alloc 0・実時間性能の計測記録 → impl record 作成 → `git fetch origin && git merge origin/main` → lockfile 再生成 → `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` 実行し証拠を記録。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `RefCell` borrow・lifetime で tap 実装が難航 | med | med | `try_borrow` + 短寿命 borrow のみ・`st` RefMut と `&arena` の disjoint を維持 |
| 2 | `chunks_out` 構造的不満杯保証の不変条件が実装で破れる | low | high | `#[debug_assert]` + 単体テストで「working slot 外の重複 push なし」を担保 |
| 3 | workspace pedantic lint 抵触 | med | low | 既存 typed コードのパターンを写す |
| 4 | 新規依存（analysis/observability）の `Cargo.lock` 差分 | low | low | `generate-lockfile` 後 `git diff Cargo.lock` で新規分のみ確認 |
| 5 | rebind/guard の競合が仕様どおりに観測できない | med | med | `bound_generation` guard + `Unbound`/`Unobserved` status の注入テストで網羅 |

## Task Breakdown

- T-1: Step 1-2（level 抽出・依存追加・binding 集約）→ `p185-t1-level-extract-and-binding-centralization.md`
- T-2: Step 3（runtime 拡張・tap 点・channel 配線）→ `p185-t2-runtime-probe-deck-and-taps.md`
- T-3: Step 4（probe 契約・broker・deck/collector）→ `p185-t3-probe-contract-broker-collector.md`
- T-4: Step 5-6（受入テスト・rt alloc・計測記録・merge/lockfile/全検証・impl record）→ `p185-t4-probe-poc-tests-verification.md`

## Verification

- [ ] `mise exec -- cargo test -p ruviform-audio --test typed_probe` → exit 0
- [ ] `mise run test-rt` → `rt_probe_*` テストが alloc 0 で exit 0
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` → exit 0（全て記録）
- [ ] `cargo tree --locked -p ruviform-audio --edges normal` → 循環 0・下向きのみ
- [ ] probe 有無で boundary 出力 bit 一致・rebind 後に正しい slot を観測
- [ ] 非 RT 制御 API の全失敗面が `ProbeError` で返る

## Open Questions / Gaps

- `StableHandleMap`/`ValidatedGraph` の port 列挙・`PortSpec` の capability 読み出しの正確な API 面 — Step 4 冒頭で確定。
- `ruviform_rt::snapshot` reader 側で `published_plan` を retain する形（`Arc` clone）の正確な実装 — Step 3 冒頭で確定。
- telemetry capacity の具体的既定値（PoC は 512 を予定）と `pool_chunks` 既定値（PoC は 16 を予定）— Step 3-4 で確定・予算根拠を impl record に記録。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（直列 critical path・単一 worker）。
- Independent work streams: なし。
- User approval required before dispatch: no（inline）。

## Review Handoff

- 仮定: `ruviform_rt::{channel, retire_channel, snapshot}`・`ruviform_observability::counter`・`ruviform_audio_analysis::ChannelLevel` の API が design 記述どおり。実装時に検証。
- レビュー重点: probe command の drain 位置（gen-scoped は `activate_generation` 内・lifetime は event drain 前）、`bound_generation` guard、`stamp == seq` 鮮度監査、`effective_slot` の compile 確定、`chunks_out` 不満杯不変条件。
