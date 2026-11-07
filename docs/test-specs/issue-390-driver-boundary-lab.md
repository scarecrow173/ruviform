---
id: "TSPEC-034TaI7Luee0T88mH1IYv2"
type: "test-spec"
status: "draft"
title: "Issue #390 Driver Boundary/Lab 検証"
created: "2026-11-07"
updated: "2026-11-07"
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
    - "docs/designs/issue-390-driver-contract-lab.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-390-driver-contract-lab.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #390 Driver Boundary/Lab 検証

## Purpose

Issue #390 driver boundary/lab の検証契約 — ruleset 適用・fixture
pass/fail・manifest 語彙・実行入口を pin する。検査限界（design
§限界）を超える主張を防ぐための oracle。

## Feature

Driver Boundary Guard と Driver Lab registry が、driver-neutral
共通契約の toolkit/GPUI 非依存を機械的に検査し、Fake driver の
semantic oracle を正直に分類する。

## Rules

- Rule: protected workspace member が forbidden dep root を
  direct/declared feature/dep.features/transitive/conditional の
  いずれの経路でも依存すれば `fail`。
- Rule: allowlisted adapter の grant 済み optional dep 辺は
  exempt。grant 外の optional 化・非 optional 宣言・
  viaFeatures 外 feature 起動・非 grant consumer 依存は `fail`。
- Rule: crate 外に露出する pub signature の forbidden path root
  漏出は `fail`（adapter も対象。private 利用は対象外）。
- Rule: UI-DVR-01..08 が cases.json の `requiredCases` で pin
  され、layer/verdict 語彙に従う。実行不能は
  `not_run`/`blocked`/`unsupported` で owner+rerun 必須。
- Rule: `pass` binding は実行機構を持つ（cargo-test/mise-task/
  manual+note）。Fake の pass を GPUI/real-os の pass にしない。

## Examples

- negative/direct-dep: `forbidden-dep-direct` + transitive。
- negative/transitive-dep: `common -> middle -> gpui` witness。
- negative/conditional-dep: optional 辺経由 `forbidden-dep-conditional`。
- negative/feature-leak: `forbidden-dep-feature`（grant 外 feature）。
- negative/not-optional: `forbidden-dep-not-optional`。
- negative/dev-dep: dev kind の `forbidden-dep-direct`。
- negative/signature-leak: `signature-leak`（pub fn/field/trait
  bound/impl。private impl は対象外）。
- negative/adapter-feature: `forbidden-dep-adapter-feature` +
  `adapter-consumer`。
- positive: allowlisted adapter 構成で verdict `pass`。
- live workspace: verdict `pass`（UI-DVR-01 boundary）。

## Guarantees

- `mise run check-ui-driver-boundary` が exit 0（live workspace）。
- 上記 fixture が tests/driver_boundary.rs で pass/fail を pin。
- `mise run test-ui-driver-contract` で manifest 整合 +
  UI-DVR-01/05/07/08 fake 実行 + evidence emission。

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
