---
id: "PLAN-034XEbS62sWmGaedjSv85f"
type: "plan"
status: "approved"
title: "Issue #631 Plugin-dock scenario skeleton 実装計画"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/tasks/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/test-specs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/impl/ir/issue-631-plugin-dock-scenario.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
  implemented-by:
    - "docs/tasks/issue-631-plugin-dock-scenario.md"
  depends-on:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/designs/issue-631-plugin-dock-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-631-plugin-dock-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/designs/issue-631-plugin-dock-scenario.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-631-plugin-dock-scenario.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Plugin-dock scenario skeleton 実装計画 Implementation Plan

## Goal

`test_gain` 実 ABI cdylib に対して plugin-dock scenario を実行し、load/discover/hash を `Pass`、editor attach/dock/pixel を `Unsupported`/`NotRun` として `ScenarioReport` + artifact digest evidence 付きで記録する test-only harness を納品する。

## Scope

単一実装 stream — 1 crate 新設 + 2 crate への additive 拡張 + mise task + evidence。分割不要。

## Upstream Documents

- `docs/specs/issue-631-plugin-dock-scenario.md`
- `docs/designs/issue-631-plugin-dock-scenario.md`
- `docs/specs/issue-627-virtual-first-verification-contract.md`（Case Schema v1 語彙）
- `docs/specs/issue-630-test-plugin-suite.md`（test_gain fixture）

## Design Inputs

- `docs/designs/issue-631-plugin-dock-scenario.md`（approved）

## File Map

- `crates/ruviform-plugin-dock-test/Cargo.toml` — 新規 test-only crate 定義（依存理由コメント付き）
- `crates/ruviform-plugin-dock-test/README.md` — test-only 宣言・lane 手順
- `crates/ruviform-plugin-dock-test/src/lib.rs` — crate docs + module export
- `crates/ruviform-plugin-dock-test/src/artifact.rs` — artifact 解決（env var + auto）・SHA-256・`PluginArtifactEvidence` 構築
- `crates/ruviform-plugin-dock-test/src/scenario.rs` — case 実行（view-mount/artifact/host-lifecycle/editor-attach/dock-pixels）と `ScenarioReport` 組立・JSONL 出力
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-scenario.rs` — lane 入口（`--out` で JSONL 出力）
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs` — 実 cdylib を使う統合テスト（verdict 正直性の assertion）
- `crates/ruviform-audio-test/src/metadata.rs` — `EnvironmentKind` variant + `as_str()`
- `crates/ruviform-audio-test/src/lib.rs` — metadata 語彙の re-export
- `crates/ruviform-ui-test/src/scenario.rs` — `ScenarioDriver::PluginHostProbe` + `pub mod plugin_dock`
- `crates/ruviform-ui-test/src/scenario/plugin_dock.rs` — scenario spec / ViewSpec / artifact evidence 型
- `crates/ruviform-ui-test/src/lib.rs` — re-export 追加
- `Cargo.toml` — `members` へ 1 行追加
- `Cargo.lock` — 再生成
- `mise.toml` — `[tasks.test-ui-plugin-dock]` 追加（additive のみ）
- `evidence/rf631/` — verification.md + plugin-dock-scenario.jsonl
- `docs/*/README.md` — index 再生成（generator のみ）

## Implementation Sequence

- [ ] `metadata.rs`/`lib.rs`（audio-test）: `EnvironmentKind` 拡張
- [ ] `scenario.rs`/`scenario/plugin_dock.rs`/`lib.rs`（ui-test）: driver variant + plugin-dock spec/evidence 経路
- [ ] `ruviform-plugin-dock-test` crate 一式 + workspace `members` 追加
- [ ] `mise.toml` task 追加
- [ ] `cargo build -p ruviform-plugin-test-vst3` → lane 実走 → `evidence/rf631/`
- [ ] docs index 再生成・impl IR 仕上げ

## Dependencies and Risks

- `test_gain` cdylib の存在が前提 — lane が `cargo build -p ruviform-plugin-test-vst3` を内包し、test 側は dev-dep で fixture package を build graph に含める。
- `InProcessEditorOps` の redeemer は `FakeNativeSlot::validate_lease` で lease を検証してから `ParentLease` を mint する（forged/stale lease を plugin に渡さない契約）。
- `ScenarioReport` 非 Serialize → JSONL は public field からの `serde_json::Value` 組立で出力層に閉じる。

## Task Breakdown

- T1（`docs/tasks/issue-631-plugin-dock-scenario.md`）: 全体を 1 task で実施（slice が小さい）。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check|lint|test ruviform-plugin-dock-test` / `ruviform-audio-test` / `ruviform-ui-test` / `ruviform-plugin-test-vst3`
- [ ] `mise run test-ui-scenario-contract` `-fake` `-gpui` / `test-ui-native-slot` / `test-ui-golden` / `test-ui-driver-contract` / `test-ui-gpui` / `check-ui-driver-boundary`
- [ ] `mise run test-ui-plugin-dock` → `evidence/rf631/plugin-dock-scenario.jsonl`
- [ ] `mise exec -- prek run --all-files`

## Open Questions / Gaps

- `ScenarioDriver::PluginHostProbe` の命名は実装時に確定（driver-neutral・証拠級を偽らない名前）。

## Self-Review

- spec の acceptance criteria を全て file map / sequence / verification に写像済み。
- 実 Window・実 editor は明示的に out of scope。

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none

## Review Handoff

- verdict 正直性（attach/dock/pixel を Pass にしない）と artifact evidence（digest+version）の記録が reviewer focus。
