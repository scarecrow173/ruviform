---
id: "TSPEC-034XTPhKNKddA4TmQhwjZJ"
type: "test-spec"
status: "approved"
title: "Issue #631 Real-window native-slot dock lane のテスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-631-real-window-dock.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-real-window-dock.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Real-window native-slot dock lane のテスト仕様

## Purpose

<!-- Why this test spec exists: the intent it preserves and when it may be retired. -->

## Feature

<!-- The behavior under specification, named like a Gherkin Feature. -->

## Rules

- <!-- Rule: an invariant or contract the feature must satisfy -->

## Examples

- <!-- Example: a concrete scenario that pins a rule down, optionally in Given/When/Then form -->

## Guarantees

- <!-- What a correct implementation must guarantee -->

## Non-goals

- <!-- Behavior or coverage this spec deliberately does not verify -->

## Risk

- <!-- What is lost or breaks if these guarantees are dropped -->

## Feature: RealOsGuiTest native-slot ops

### Rule: slot ops は canonical SharedSurface を app thread で操作する

- **Example**: `declare_native_slot` → `bind_native_slot` →
  `lease_native_slot` の連続呼出で、各 op が `UiJob` 経由で
  `driver.surface()` に到達し、返り値が `ruviform-ui` 型のみである
  こと。
- **Example**: 存在しない slot への lease/validate が
  `ScenarioError::Driver`（`native slot` prefix）で失敗し、
  panic しないこと。

### Rule: NativePluginDock capability は backend 連動

- **Example**: `supported_capabilities()` が
  `surface::BACKEND_AVAILABLE && !cfg!(feature = "gpui-test")` の
  ときのみ `NativePluginDock` を含む。`gpui-test` feature 下では
  含まない。

## Feature: real-dock lane verdicts

### Rule: 各 step は実走できたものだけ pass

- **Example**: `real-native-slot` で declare/bind/settle/lease/
  validate/container_token/stale-forged 拒否/release が順に記録され、
  途中失敗時はそれ以降が NotRun になる。
- **Example**: forged payload の redeem が拒否され、その拒否が
  detail に記録される（fake success なし）。

### Rule: editor attach は Unsupported を維持

- **Example**: `test_gain` の `createView` null →
  `HostError::Unsupported` → verdict Unsupported。Pass/Fail に
  しない。
- **Example**: plugin artifact 欠落時は依存 case が NotRun になり、
  silent skip や pass にならない。

### Rule: headless は fail-closed

- **Example**: window 生成失敗時に全 case が NotRun +
  environment_blocked detail を記録し、exit code は 0（Fail のみ非 0）。

### Rule: capture は対象を偽らない

- **Example**: `real-capture` の detail が「window/slot 領域の画素」
  であり plugin editor 画素を主張しない。

## Feature: JSONL schema 不変

- **Example**: `plugin-dock-scenario`（headless）の出力行形式が
  `jsonl.rs` 抽出後も不変であること（既存 test/lane で確認）。
- **Example**: real lane の case 行が driver/capabilities/required_
  capabilities/verdict/environment/evidence/details/timeout/cleanup
  を含む。
