---
id: "TSPEC-034Y3hpBGEGO5mhH5d4wsG"
type: "test-spec"
status: "approved"
title: "Issue #628 close-out — test specification"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-628-close-out.md"
    - "docs/designs/issue-628-close-out.md"
    - "docs/knowledge/docs-operations-and-agent-workflow.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-628-close-out.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-628-close-out.md"
    - "docs/designs/issue-628-close-out.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-628-close-out.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-628-close-out.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-628-close-out.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 close-out — test specification

## Purpose

Issue #628 close-out slice の 2 つのコード変更（RunMetadata JSON・
`StreamFault::Cancellation`）が守るべき契約を固定する。deterministic
backend の Pass は OS-L1 証拠であり、実 OS path の証明ではない。

## Feature: RunMetadata JSON serialization

### Rule JSON-1: round-trip fidelity
Given a fully populated `RunMetadata`（全 field 設定済み）、
`to_json` → `from_json` は `PartialEq` で一致する
`RunMetadata` を返す。`save_json` → `load_json` も同様。

### Rule JSON-2: Case Schema v1 wire vocabulary
Serialized JSON の enum 表現は kebab-case の Case Schema v1 語彙:
`environment_kind` は `EnvironmentKind::as_str()` の spelling
（`deterministic-backend` 等）と一致し、`level`/`requirement`/
`result` も kebab-case（`os-l1`/`required`/`not-run` 等）。
`as_str()` と serde spelling の drift を test で pin する。

### Rule JSON-3: non-RT boundary
JSON API は alloc/IO を伴う非 RT test-driver API であり、
`rt_` prefix テストの対象にしない。既存 `rt_` テストは非回帰。

## Feature: Cancellation fault

### Rule CAN-1: cancelled callback delivers nothing
Given `StreamFault::Cancellation { at_callback: N }` を script した
stream、index N の pump は data callback を発火せず、error callback
へ `DeviceError::Backend("cancelled")` を配送し、`PumpReport` は
`frames: 0`・`invalidated: true`・`errors_fired >= 1` を報告する。

### Rule CAN-2: stream terminates honestly
Cancellation 後の pump は `Err(DeviceError::StreamInvalidated)` を返し、
`is_*_open()` は false。dead stream の drop 後は再 open できる
（reconnect 経路は `Invalidate` と同じ lifecycle）。

### Rule CAN-3: both directions
output・input 両 direction で同じ semantics が成立する。
`inject_*_fault` による mid-stream 注入でも同じ効果を持つ。

### Rule CAN-4: no production contract change
`git diff` は `crates/ruviform-audio-test/**` と docs/evidence/
生成物のみ。production crate の src・`DeviceError` への新 variant
追加はない。

## Examples

- output stream に `Cancellation { at_callback: 1 }`:
  index 0 は通常配送、index 1 で data callback skip +
  `Backend("cancelled")` + stream 終了、index 2 の pump は
  `StreamInvalidated`。
- `RunMetadata::deterministic(...)` を `to_json`/`save_json` して
  `from_json`/`load_json` で復元し `assert_eq!`。

## Guarantees

- JSON wire format が Case Schema v1 語彙と一致し続けること
  （`as_str()` pin test）。
- cancellation が in-flight delivery の中断として観測可能であること
  （data callback 未発火 + `Backend("cancelled")` + 終了）。

## Non-goals

- MIDI 側（`backend/midi.rs`/`MockPort`）への cancellation 拡張
  （Deferred — 棚卸し表に記録）。
- 実 OS/実デバイス/実スレッド timing の証明。
- 新規 CI lane・workflow/mise 変更。

## Risk

- cancellation の解釈が consumer 期待とずれた場合、Issue close 判定で
  指摘される可能性 — design doc・棚卸し表に写像を明記して緩和。
