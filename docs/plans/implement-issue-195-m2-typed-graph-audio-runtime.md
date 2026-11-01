---
id: "PLAN-034QfqzG1cwHntPLBtSz9L"
type: "plan"
status: "approved"
title: "Implement Issue #195 M2 Typed Graph Audio Compiler・Prepared Plan・RT Runtime"
created: "2026-11-01"
updated: "2026-11-01"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/195"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
    - "docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
    - "docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md"
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/196"
  deferred-by: []
---

# Implement Issue #195 M2 Typed Graph Audio Compiler・Prepared Plan・RT Runtime Implementation Plan

## Goal

design `DESIGN-034QfjYdt3o2a4lijg4lwt` どおりに `ruviform-audio` へ `typed` module（非 RT Compiler・bounded slot arena・PDC・Stable Runtime Slot ownership の `GraphRuntime`/`RtRuntime`・activation/fault 計装）を追加し、G2-01〜G2-08 の受入を PoC・golden fixture・`mise` 検証の実測で証明する。

## Scope

1 本の実装ストリーム（compiler→runtime→tests は全て同一 critical path・直列）。M4 plugin 接続・M3 event payload・#185 probe・旧 API 削除は含まない。

## Upstream Documents

- `docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md`（approved・AC-1〜9）
- `docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md`（proposed→PoC 後 accepted・Option B）
- `docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md`（棚卸し正本）

## Design Inputs

- `docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md`（approved）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-audio/Cargo.toml` | `ruviform-graph` path 依存追加 | 更新 |
| `crates/ruviform-audio/src/lib.rs` | `pub mod typed` + re-export | 更新 |
| `crates/ruviform-audio/src/typed/mod.rs` | crate doc・再輸出 | 新規 |
| `crates/ruviform-audio/src/typed/buffer.rs` | `AlignedSamples`/`Slot`/`DelayLine`/`SlotArena` | 新規 |
| `crates/ruviform-audio/src/typed/io.rs` | `NodeIo`/`InputView`/`OutputView` | 新規 |
| `crates/ruviform-audio/src/typed/compile.rs` | `Compiler`/`CompiledPlan`/`CompileError`/`NodeFactory`/PDC | 新規 |
| `crates/ruviform-audio/src/typed/runtime.rs` | `GraphProcessor`/`RuntimeCommand`/`NodeEvent`/`GraphRuntime`/`RtRuntime`/`SharedStatus`/`RuntimeFault` | 新規 |
| `crates/ruviform-audio/tests/typed_m2.rs` | G2-01/02/03/06 の受入（schedule/buffer/PDC/frames/RT 監査） | 新規 |
| `crates/ruviform-audio/tests/ownership_poc.rs` | Plan-owned vs Stable Slot PoC（G2-04）+ fault 注入（G2-05） | 新規 |
| `crates/ruviform-audio/tests/golden_legacy.rs` | 旧 API golden fixture（G2-07） | 新規 |
| `docs/impl/ir/issue-195-m2-typed-graph-audio-runtime.md` | impl record・EX/TA traceability・検証証拠（G2-08/AC-9） | 新規 |
| `Cargo.lock` | 再生成 | `mise exec -- cargo generate-lockfile` |

## Implementation Sequence

- [ ] Step 1: `Cargo.toml` 依存 + `lib.rs`/`typed/mod.rs` 骨格 + `buffer.rs`（aligned slot/delay line）。
- [ ] Step 2: `io.rs`（NodeIo/view）+ `compile.rs`（schedule/slot/binding/PDC/定義済み失敗/lifetime 監査）。
- [ ] Step 3: `runtime.rs`（GraphProcessor・GraphRuntime/RtRuntime・snapshot+spsc+retire+generation+fault status）。
- [ ] Step 4: `tests/typed_m2.rs`（multi-bus DAG・deterministic・fan-out・拒否・PDC・64/128/256/可変 frames・zero/tail・`alloc_counter` RT 監査）。
- [ ] Step 5: `tests/ownership_poc.rs`（同一 MockDSP で Option A/B 比較 + G2-05 fault 注入）+ ADR accepted 更新。
- [ ] Step 6: `tests/golden_legacy.rs` + EX/TA traceability（impl record）。
- [ ] Step 7: `git fetch origin && git merge origin/main` → `cargo generate-lockfile` → `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`cargo test -p ruviform-audio --locked`・`cargo test --workspace --locked` を実行し証拠を impl record に記録。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `StableHandleMap`/IR API 面が設計想定と相違 | med | med | `cargo check` で早期検出・実 API に合わせて修正 |
| 2 | `RefCell` borrow・borrowed IO の lifetime で compile が難航 | med | med | NodeIo を per-access materialize 型に限定・複雑な同時 borrow を避ける |
| 3 | workspace `pedantic` lint が新コードに抵触 | med | low | 既存 crate の同パターンを写す |
| 4 | `ruviform-graph` 依存追加による `Cargo.lock` 差分 | low | low | `generate-lockfile` 後 `git diff Cargo.lock` で新規分のみ確認 |
| 5 | 両 ownership 案が不適格 | low | high | 測定データ付き Blocked 報告（妥協採用禁止） |

## Task Breakdown

- T-1: Step 1-2（buffer/io/compile）→ task doc 1 件
- T-2: Step 3（runtime/activation/fault）→ task doc 1 件
- T-3: Step 4-5（受入 test + ownership PoC + fault 注入）→ task doc 1 件
- T-4: Step 6-7（golden・traceability・merge/lockfile/全検証・impl record）→ task doc 1 件

## Verification

- [ ] `mise exec -- cargo test -p ruviform-audio --test typed_m2` → exit 0
- [ ] `mise exec -- cargo test -p ruviform-audio --test ownership_poc` → exit 0
- [ ] `mise exec -- cargo test -p ruviform-audio --test golden_legacy` → exit 0
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` → exit 0（全て記録）
- [ ] `mise exec -- cargo test -p ruviform-audio --locked` / `cargo test --workspace --locked` → exit 0
- [ ] `cargo tree --locked -p ruviform-audio --edges normal` → `ruviform-graph`/`rt`/`audio-core`/`media-core` のみ・循環 0
- [ ] `rg "unsafe" crates/ruviform-audio/src/typed` → 0 match

## Open Questions / Gaps

- `StableHandleMap` の runtime index 取得 API 名（`node_index`/`port_index`/`connection_index`）と `ValidatedGraph` の node/connection 列挙 API — 実装時に確定。
- `spsc::channel` の try_push/try_pop 正確な API 面 — `ruviform-rt` docs で確定。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（直列 critical path・単一 worker）。
- Independent work streams: なし。
- User approval required before dispatch: no（inline）。

## Review Handoff

- 仮定: `ruviform_rt::{slot, retire_channel, channel}`・`StableHandleMap` API が design 記述どおり。実装時に検証。
- レビュー重点: PDC edge delay 計算（merge での `max - own`）、stamp 検査の境界（caller-fed input の `seq+1`）、install command の generation 整合（部分 push 失敗 rollback）。
- 上流: spec（AC-1〜9）・ADR（Option A/B/C・PoC 判定基準）・design（approved）。
