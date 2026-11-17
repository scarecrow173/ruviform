---
id: "TSPEC-034YZZbBgKzfmFfSLl9ai4"
type: "test-spec"
status: "approved"
title: "Issue #725 serde 語彙 Phase A — test specification"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-725-serde-vocab-phase-a.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
  blocks: []
  supersedes: []
  derives-from:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
  verifies:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
---

# Issue #725 serde 語彙 Phase A — test specification

## Purpose

`EnvironmentKind::RealAbiTestPlugin` の serde 語彙 gap 解消
（Phase A: deserialize alias 受容・emit 不変）を pin する。
移行期間中の挙動 — deserialize は新旧両綴りを受容し、serialize/
`as_str()` は旧綴り `real-abi-test-plugin` を維持する — が
意図せず変化しないことを保証する。emit flip（第2 slice）で
この spec の emit 側 rule は更新される。

## Feature: EnvironmentKind serde 語彙（Phase A 移行期間）

### Rule TS-725-1: deserialize は新旧両綴りを受容する

**Example**: `serde_json::from_str::<EnvironmentKind>("\"test-plugin\"")`
は `Ok(EnvironmentKind::RealAbiTestPlugin)`、
`serde_json::from_str::<EnvironmentKind>("\"real-abi-test-plugin\"")`
も `Ok(EnvironmentKind::RealAbiTestPlugin)` を返す。
`RunMetadata::from_json` で `environment_kind` に `"test-plugin"`
を含む文書も `RealAbiTestPlugin` として受容される。

### Rule TS-725-2: serialize / `as_str()` emit は旧綴りを維持する

**Example**: `serde_json::to_string(&EnvironmentKind::RealAbiTestPlugin)`
は `"\"real-abi-test-plugin\""`、`as_str()` は
`"real-abi-test-plugin"` を返す（`rename_all` 由来の canonical
綴り不変 — `#[serde(alias)]` は serialize に影響しない）。
`environment_kind_serde_matches_as_str` が全 variant で
serde emit == `as_str()` を強制し続ける。

### Rule TS-725-3: 未知語彙は引き続き拒否する（fail-closed）

**Example**: `"test_plugin"`・`"TestPlugin"`・`"plugin-test"` 等の
schema 語彙・旧綴り以外の綴りは deserialize エラー。
`json_rejects_unknown_vocabulary_and_malformed_input` の契約を維持。

### Rule TS-725-4: round-trip 後方互換

**Example**: 旧綴り `real-abi-test-plugin` で emit された
`RunMetadata` JSON 文書は `from_json` で同一値に復元される
（既存 `json_string_round_trip_preserves_all_fields` を維持）。
`"test-plugin"` 綴りの文書を `from_json` で読んだ後 `to_json` で
再 emit すると旧綴り `real-abi-test-plugin` になる
（canonical emit は旧綴りのまま — 移行期間中の正規化方向を pin）。

## Guarantees

- Phase A の変更は deserialize 受容集合の追加のみであり、
  serialize/`as_str()` emit・型名・variant 集合・他 enum 語彙に
  影響しない。
- `ci/consumer-verify` の schema 語彙（`test-plugin` 正本）に対し、
  deserialize 側が契約語彙を受容できる — emit flip 前でも
  schema 語彙文書の読み込み経路が成立する。
- plugin-dock-test の emit 経路（`as_str()`）は不変のため、
  rf#631 占有クレートの literal assert は影響を受けない。

## Non-goals

- emit flip 後の `"test-plugin"` emit pin — 第2 slice の test-spec
  で扱う。
- `EnvironmentKind` 以外の enum 語彙・他クレートの serde 変更。
- consumer 側の pin bump・consumer repo の検証（影響評価で不要と
  判定済み）。

## Risk

- alias の付与漏れ・綴り誤り → `"test-plugin"` deserialize が
  拒否され schema 語彙文書を読めない。TS-725-1 の pin で検出。
- alias が serialize に波及する serde 属性の誤用 → emit が
  `test-plugin` に変わり plugin-dock-test が壊れる。TS-725-2 の
  pin と `environment_kind_serde_matches_as_str` で検出。
- 「gap 解消済み」の誤記 → emit 側は未解消のまま。docs/PR で
  Phase A 範囲を明示し、issue-629 docs は「deserialize alias
  適用・emit flip は第2 slice」と記す。

## 検証コマンド対応

| Rule | 検証 |
| --- | --- |
| TS-725-1/3/4 | `mise run test ruviform-audio-test` の新規 alias pin テスト + 既存 `json_rejects_unknown_vocabulary_and_malformed_input`・`json_string_round_trip_preserves_all_fields` |
| TS-725-2 | 既存 `environment_kind_serde_matches_as_str`・`environment_kind_wire_spellings_match_case_schema_v1`（emit 綴り不変の pin として維持） |
| 非回帰 | `mise run check`/`lint`/`fmt-check ruviform-audio-test`・`mise run test-ci-consumer-verify`（schema 側） |
