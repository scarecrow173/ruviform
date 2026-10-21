---
id: "TSPEC-034JRQpjZtjsWuJOB1WYN0"
type: "test-spec"
status: "draft"
title: "音声エンジン テスト仕様"
created: "2026-10-21"
updated: "2026-10-21"
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
    - "docs/designs/audio-engine.md"
  derived-by: []
  verifies:
    - "docs/designs/audio-engine.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 音声エンジン テスト仕様

## Purpose

Issue #25 音声エンジンの公開契約テスト仕様。pull 駆動・ミックス・スナップショット差替・RT 契約を検証。

## Feature

`Graph`/`Mixer`/`PlanHandle`/`Session::open_output` の振る舞い。

## Rules

- ユニットテストは `src/` 内 `#[cfg(test)] mod tests`（クレート慣例）
- モック `OutputStream`/`Device` でデバイス非依存に駆動
- RT テストは `rt_` 接頭辞（`mise run test-rt` が拾う）

## Examples

- <!-- Example: a concrete scenario that pins a rule down, optionally in Given/When/Then form -->

## Guarantees

- <!-- What a correct implementation must guarantee -->

## Non-goals

- <!-- Behavior or coverage this spec deliberately does not verify -->

## Risk

- <!-- What is lost or breaks if these guarantees are dropped -->
