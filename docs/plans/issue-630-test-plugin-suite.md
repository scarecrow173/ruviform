---
id: "PLAN-Ef1lMq6niXu1LcH2meLOrU"
type: "plan"
status: "approved"
title: "Issue #630 VST3 test_gain first slice 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "docs/tasks/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "docs/test-specs/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "docs/impl/ir/issue-630-test-plugin-suite.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-630-test-plugin-suite.md"
  implemented-by:
    - "docs/tasks/issue-630-test-plugin-suite.md"
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  derived-by:
    - "docs/tasks/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  references:
    - "evidence/rf630/verification.md"
  defers: []
  deferred-by: []
---

# Issue #630 VST3 test_gain first slice 実装計画

## Goal

新規 test-only crate `crates/ruviform-plugin-test-vst3` に VST3 `test_gain` プラグインを実装し、本番 host path（`Vst3Library::load` → `ErasedPlugin`）と in-process COM round-trip の両経路で golden PCM・sample-accurate automation・RT non-allocation を検証可能な状態で納品する。

## Scope

単一ストリーム。Rust 実装 + crate README + doc chain + evidence。`crates/ruviform-plugin-vst3` 本体・`crates/README.md`・`mise.toml`・`POLICIES.md`・`.github/workflows/` は変更しない。`scarecrow173/ruviform#628` runtime への依存 edge は持たない。

## Upstream Documents

- `docs/specs/issue-630-test-plugin-suite.md`（SPEC-034XC2yDsqDluVXBOAyKnC）
- `docs/designs/issue-630-test-plugin-suite.md`（DESIGN-034XC4o1eLquthCFAcOrS8）
- `docs/specs/issue-627-virtual-first-verification-contract.md`（環境種別・証拠規範）
- Issue #630: https://github.com/scarecrow173/ruviform/issues/630

## Design Inputs

- design doc「Suite 規約」「`test_gain` 意味論」「検証経路（2 系統・独立）」— 命名/CID/format 分割/#628 統合点/外部 validator の pin
- `crates/ruviform-plugin-vst3/src/{lib,factory,component,controller,gain,block,strings}.rs` — COM/RT/lifecycle の先例（変更しない・参照のみ）
- `crates/ruviform-plugin-core`（`PluginProcessor`/`Event`/`EventList`/`ParamInfo`/`Lifecycle`）・`crates/ruviform-audio-core`（`PlanarBufferView(Mut)`）— neutral 契約

## File Map

- `crates/ruviform-plugin-test-vst3/Cargo.toml` — cdylib+rlib、新規 crate 定義
- `crates/ruviform-plugin-test-vst3/README.md` — test-only 宣言
- `crates/ruviform-plugin-test-vst3/src/lib.rs` — entry points + `sdk_enum_i32`
- `crates/ruviform-plugin-test-vst3/src/{strings,gain,block,component,controller,factory}.rs` — plugin 実装（独立実装）
- `crates/ruviform-plugin-test-vst3/tests/host_path.rs` — 本番 host path 統合テスト
- `Cargo.toml`（root）— `members` へ 1 行追加のみ
- `Cargo.lock` — 依存解決の再生成
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-630-test-plugin-suite*.md` + `docs/impl/exp/*-exp.jsonl` companion — doc chain
- `evidence/rf630/verification.md` — 検証 ledger
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/README.md` — index 再生成のみ

## Implementation Sequence

- [ ] T1: crate skeleton（Cargo.toml・lib.rs entry points・root members 追加・Cargo.lock 再生成）
- [ ] T2: strings/gain/block（param info・sample-accurate processor・RT 境界 core + unit tests）
- [ ] T3: component/controller/factory（COM 実装 + in-process round-trip・rt_ テスト）
- [ ] T4: tests/host_path.rs（本番 host path golden 検証）
- [ ] T5: crate README + lib docs test-only 宣言の確認
- [ ] T6: `mise run fmt-check`/`check`/`lint`/`test`/`build`（`ruviform-plugin-test-vst3`）+ `mise run test-rt`（rt_ 収集確認）
- [ ] T7: test-spec → impl/ir + impl/exp（+jsonl）→ evidence/rf630/verification.md
- [ ] T8: docs index 再生成（`doc_maintenance.js index apply` / `sync_impl_index.js`）
- [ ] T9: review-protocol レビュー（blocker 解消）
- [ ] T10: `git fetch origin main && git merge origin/main` → 生成物再生成 → コミット → push → PR → worker_done

## Dependencies and Risks

- **依存**: `vst3 = "0.3.0"`（lockfile 既存）、`ruviform-plugin-core`/`ruviform-audio-core`（workspace path）、dev-dep `ruviform-plugin-vst3` feature `host`（adapter 本体は pristine — dev-dep edge のみ）
- **リスク**: cdylib が `deps/` に生成されない環境差 → `fixture_path()` 先例で解決・実測確認、失敗時は evidence に NotRun 記録
- **リスク**: `vst3` binding の unsafe/FFI 詳細差異 → adapter の SAFETY パターンを踏襲しつつ独立実装、clippy `-D warnings` で検出
- **リスク**: doc-driven-dev-graph router が repo 既存 broken relation で blocked → 先行 slice（#627）と同じく index 整合は doc-maintenance で個別担保し、blocked 状態を plan に記録

## Task Breakdown

- T1〜T5: 実装（File Map どおり、単一スト림）
- T6: canonical 検証
- T7〜T8: 記録・index
- T9: review-protocol
- T10: 提出

## Verification

- [ ] `mise run fmt-check` exit 0
- [ ] `mise run check ruviform-plugin-test-vst3` exit 0
- [ ] `mise run lint ruviform-plugin-test-vst3` exit 0（`-D warnings`）
- [ ] `mise run test ruviform-plugin-test-vst3` exit 0 — in-process COM round-trip・DSP golden・host path 全 pass
- [ ] `mise run build ruviform-plugin-test-vst3` exit 0 — cdylib 生成確認
- [ ] `mise run test-rt` で `rt_` テストが実際に収集・実行されたことをテスト名で確認
- [ ] `git status`/`git diff` が編集許可範囲に限定されること
- [ ] review-protocol verdict が ACCEPT / ACCEPT WITH NON-BLOCKING NOTES

## Open Questions / Gaps

- なし（blocker は Coordinator 確定の slice scope で解消済み）

## Self-Review

- [x] Every spec requirement maps to a task（spec Requirements → T2 DSP/RT、T3 COM、T4 host path、T5 test-only 宣言、T6 検証、T7 記録）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 crate・単一ストリーム）
- Independent work streams: none
- User approval required before dispatch: yes → review-protocol の blocker なしを承認として扱う（Worker 共通契約）

## Review Handoff

- レビュアーは特に以下を確認されたい: (a) sample-accurate 適用の off-by-one 定義（offset = 影響を受ける最初のフレーム）と golden test の対応、(b) 独立実装の境界（plugin 固有ロジックは独立、`vst3` binding 層のみ共通）、(c) `kNotImplemented` を返す State/MIDI/transport の正直さ、(d) `rt_` テストが process 経路全体をカバーしているか。
