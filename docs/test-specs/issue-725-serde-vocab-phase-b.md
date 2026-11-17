---
id: "TSPEC-034Z7dUHf47CjwtsI34B6k"
type: "test-spec"
status: "approved"
title: "Issue #725 serde 語彙 Phase B — test specification"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-725-serde-vocab-phase-b.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  blocks: []
  supersedes: []
  related:
    - "docs/test-specs/issue-725-serde-vocab-phase-a.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #725 serde 語彙 Phase B — test specification

## Purpose

`EnvironmentKind::RealAbiTestPlugin` の serde emit flip（Phase B:
canonical wire 綴りを schema 語彙 `test-plugin` に統一）を pin
する。Phase A の test-spec（`docs/test-specs/
issue-725-serde-vocab-phase-a.md`）が「移行期間中の挙動」として
定義した emit 側 rule を、解消後の状態（emit = `test-plugin`、
deserialize は新旧両綴りを恒久的に受容）に更新する。

## Feature: EnvironmentKind serde 語彙（Phase B 解消後）

### Rule TS-725B-1: emit は schema 語彙 `test-plugin` に統一される

**Example**: `serde_json::to_string(&EnvironmentKind::
RealAbiTestPlugin)` は `"\"test-plugin\""`、
`EnvironmentKind::RealAbiTestPlugin.as_str()` は
`"test-plugin"` を返す。`environment_kind_wire_spellings_match_
case_schema_v1` が `RealAbiTestPlugin` → `"test-plugin"` を
含む全 variant の `as_str()` 綴りを schema 語彙へ literal pin
する。`environment_kind_serde_matches_as_str` が serde emit ==
`as_str()` を全 variant で強制し続ける（flip が atomic に
行われたことの検証）。

### Rule TS-725B-2: deserialize は新旧両綴りを恒久的に受容する

**Example**: `serde_json::from_str::<EnvironmentKind>("\"test-
plugin\"")` と `serde_json::from_str::<EnvironmentKind>(
"\"real-abi-test-plugin\"")` の両方が `Ok(EnvironmentKind::
RealAbiTestPlugin)` を返す（canonical = `test-plugin`、旧綴りは
`alias` で受容継続）。`RunMetadata::from_json` で
`environment_kind` に旧綴り `real-abi-test-plugin` を含む文書
（historical evidence・flip 前に emit された文書）も
`RealAbiTestPlugin` として受容される。

### Rule TS-725B-3: 未知語彙は引き続き拒否する（fail-closed）

**Example**: `"test_plugin"`・`"TestPlugin"`・`"plugin-test"`・
`"testplugin"` 等の受容ペア以外の綴りは deserialize エラー。
`environment_kind_deserialize_accepts_both_vocab_spellings` の
拒否 pin と `json_rejects_unknown_vocabulary_and_malformed_
input` の契約を維持。

### Rule TS-725B-4: round-trip は新綴りで正規化される

**Example**: `"test-plugin"` 綴りの `RunMetadata` 文書を
`from_json` で読み `to_json` で再 emit すると
`environment_kind` は `"test-plugin"` になる。旧綴り
`real-abi-test-plugin` の文書を読んだ場合も再 emit は
`"test-plugin"` に正規化される（canonical emit は新綴り）。
旧綴り文書同士の round-trip（`json_string_round_trip_
preserves_all_fields`）は維持。

### Rule TS-725B-5: plugin-dock-test の emit 経路が schema 語彙を出力する

**Example**: `plugin_dock.rs` の `fixture_run_reports_honest_
verdicts` が、実 cdylib 解決環境で `environment["environment_
kind"] == "test-plugin"`（:168）および各 session case の
environment `environment_kind == Some("test-plugin")`（:211）を
assert する。cdylib 未解決環境では :137-149 の fail-closed
degrade 経路のみ通り、assert の実効実行は NotRun として記録
される（fake pass なし）。

## Guarantees

- serde emit（`rename` 由来）と `as_str()` が `test-plugin` で
  一致し、`environment_kind_serde_matches_as_str` が全 variant
  で成立し続ける。
- deserialize の受容集合は `{test-plugin, real-abi-test-plugin}`
  の 2 綴りで固定 — それ以外の語彙は拒否（fail-closed）。
- `EnvironmentKind` 型名・variant 集合・`#[non_exhaustive]`・
  他 enum 語彙・`as_str()` の `const` 性は不変。
- `ci/consumer-verify` の schema 語彙（`test-plugin` 正本）と
  emit が一致 — lane record 検証で未知語彙拒否される状態が
  解消される。
- historical evidence（旧綴りを含む JSONL）は書き換えず、
  deserialize alias により読み戻し可能なまま残る。

## Non-goals

- `EnvironmentKind` 型名の rename・`RealAbiTestPlugin` variant
  名の変更・他 enum 語彙の統一。
- consumer repo（audio-lens・synvelis）の変更・pin bump
  （影響評価で不要判定済み）。
- `ci/consumer-verify/**`・`.github/**` の変更（schema 正本は
  既に `test-plugin`）。
- historical evidence JSONL の書き換え・opt-in lane の実走・
  新規 evidence JSONL の再採取。
- `plugin_dock.rs` の fail-closed degrade 設計（:137-149）の
  構造変更 — 綴り assert の期待値更新のみ。

## Risk

- `rename`/`alias` の向きを誤る（canonical と alias の逆転）→
  emit が `real-abi-test-plugin` のまま残り gap 未解消、または
  旧綴りが拒否され historical evidence が読めない。TS-725B-1/2
  の pin で検出。
- `as_str()` flip の漏れ → serde emit と `as_str()` が乖離し
  `environment_kind_serde_matches_as_str` が失敗。atomic flip
  の徹底で防止。
- plugin-dock-test assert の更新漏れ → cdylib 解決環境で
  `fixture_run_reports_honest_verdicts` が失敗。TS-725B-5 で
  検出（degrade 環境では検出不可 — 実効範囲を記録）。
- 「解消済み」の誤記（flip 前・degrade のみでの記載）→
  #629 design Risks 表の警告パターン。status 追記は flip 実装・
  検証後に限定し、degrade のみの場合は実効範囲を限定して記録。

## 検証コマンド対応

| Rule | 検証 |
| --- | --- |
| TS-725B-1/2/3/4 | `mise run test ruviform-audio-test` — 更新済み emit pin 3 テスト + `environment_kind_serde_matches_as_str` + `json_rejects_unknown_vocabulary_and_malformed_input` + `json_string_round_trip_preserves_all_fields` |
| TS-725B-5 | `mise run test ruviform-plugin-dock-test` — `fixture_run_reports_honest_verdicts`（cdylib 解決可否で実効/degrade を記録） |
| 非回帰 | `mise run check|lint|test ruviform-audio-test`・`check|lint|test ruviform-plugin-dock-test`・`mise run test-ci-consumer-verify`（schema 側）・`mise run fmt-check`・`mise exec -- prek run --all-files` |
| NotRun | `*-full`/`*-msrv`/3OS 横断・opt-in lane（`test-ui-plugin-dock*`）実走 — daily CI 委譲・運用判断 |
