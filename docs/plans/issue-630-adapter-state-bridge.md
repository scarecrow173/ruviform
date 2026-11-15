---
id: "PLAN-034Y14eOip1VTyZMiRueNr"
type: "plan"
status: "approved"
title: "Issue #630 adapter-state-bridge slice — 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-adapter-state-bridge.md"
  implements:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  implemented-by:
    - "docs/tasks/issue-630-adapter-state-bridge.md"
  depends-on:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
    - "docs/plans/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-adapter-state-bridge.md"
    - "docs/test-specs/issue-630-adapter-state-bridge.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-630-adapter-state-bridge.md"
  verified-by:
    - "docs/test-specs/issue-630-adapter-state-bridge.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 adapter-state-bridge slice — 実装計画

## Goal

両 adapter host bridge（VST3 `IBStream` / CLAP `clap.ext.state`）に
標準 ABI state 境界を実装し、中立 `save_state`/`restore_state`
を実 plugin state 呼び出しへ橋渡しする。

## Scope

単一実装 stream — 2 adapter + 1 fixture stub 修正 + doc/evidence。
`test_stateful` fixture・`mark_dirty`・Preset は後続 slice。

## Upstream Documents

- `docs/specs/issue-630-adapter-state-bridge.md`
- `docs/specs/issue-630-test-plugin-suite.md`
- `docs/specs/issue-630-l1-runner.md`

## Design Inputs

- `docs/designs/issue-630-adapter-state-bridge.md`（approved —
  blob framing・`BridgeStream` 契約・呼び出し列・error mapping・
  RT 境界・side finding を全て確定済み）

## File Map

- `crates/ruviform-plugin-vst3/src/host.rs` — `BridgeStream`
  （`IBStream` ComWrapper）新設、`Vst3Plugin::save_state`/
  `restore_state`、state 呼び出し列の free function、
  module doc 更新、`#[cfg(test)]` stateful COM double test、
  `unsupported_state_verbs_are_defaults` 書き換え
- `crates/ruviform-plugin-vst3/src/controller.rs` — fixture
  `GainController::setState`/`getState` を `kNotImplemented` に
  修正 + test pin（side finding — design 記載）
- `crates/ruviform-plugin-clap/src/host.rs` — import 追加、
  `ClapPlugin::save_state`/`restore_state`、module doc 更新、
  `unsupported_state_verbs_are_defaults` 書き換え
- `crates/ruviform-plugin-clap/Cargo.toml` — `host` feature に
  `"clack-extensions/state"` 追加
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/`
  `issue-630-adapter-state-bridge.*` + `evidence/rf630/` —
  本 slice の文書・証拠チェーン

## Implementation Sequence

- [x] VST3: `BridgeStream` + framing helper + state 呼び出し列
      free function + `Vst3Plugin` method + module doc
- [x] VST3: `#[cfg(test)]` stateful COM double（distributed +
      single-component 両形・malformed・seek/tell/partial）
      + fixture defined-decline test 書き換え
- [x] `controller.rs` stub 正直化 + test pin
- [x] CLAP: `Cargo.toml` feature + `PluginState` import +
      `ClapPlugin` method + module doc + test 書き換え
- [x] fmt/check/lint/test 両 crate + l1-real-plugin lanes +
      test-rt + prek
- [x] evidence + docs index 再生成 + review-protocol

## Dependencies and Risks

- 既存 L1 e2e（lease 外）が init 済み stateless fixture に
  `Err(Unsupported)` を pin — error mapping（`kNotImplemented`
  → `Unsupported`、extension 無し → `Unsupported`）が hard
  constraint。
- `clack-extensions/state` は additive feature — `Cargo.lock`
  差分が出たら理由を記録。
- VST3 COM call は全て `unsafe` — 既存慣行の
  `#[expect(unsafe_code)]` + SAFETY コメントに従う。

## Task Breakdown

`docs/tasks/issue-630-adapter-state-bridge.md` 参照。

## Verification

- `mise run fmt-check` / `check` / `lint` / `test`（両 crate）
- `mise run check-l1-real-plugin` / `lint-l1-real-plugin` /
  `test-l1-real-plugin`（既存 e2e 緑維持 — fixture は stateless）
- `mise run test-rt`（RT 不変条件）
- `mise exec -- prek run --all-files`
- `git diff` で禁止パスが空であること

## Open Questions / Gaps

- real-plugin state evidence は `test_stateful` fixture slice
  まで NotRun（in-process double の成功を real-plugin Pass と
  読み替えない）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatch worker が直接実装）
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol
  で代替 — blocker 無しを承認とみなす）

## Review Handoff

- error mapping が既存 L1 e2e の `Err(Unsupported)` pin と
  整合するか、`BridgeStream` の seek/write 意味論、single-
  component の二重呼び出し回避を重点レビュー。
