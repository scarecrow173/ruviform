---
id: "ADR-034Z7ygFmhM17agQb1FhO7"
type: "adr"
status: "accepted"
title: "Issue #725 EnvironmentKind RealAbiTestPlugin serde canonical 綴りを test-plugin に統一"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/725"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "evidence/rf725/consumer-impact-assessment.md"
  changes:
    added:
      - type: "file"
        path: "docs/adr/issue-725-serde-vocab-phase-b.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  implemented-by:
    - "docs/plans/issue-725-serde-vocab-phase-b.md"
  depends-on:
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Issue #725 EnvironmentKind RealAbiTestPlugin serde canonical 綴りを test-plugin に統一

## Context and Problem Statement

`EnvironmentKind::RealAbiTestPlugin` の serde emit 側は
`real-abi-test-plugin` を出力するが、Case Schema v1 /
consumer-verify 契約語彙の正本は `test-plugin`
（`ci/consumer-verify/evidence.mjs` `ENVIRONMENT_KINDS`）。
Phase A（PR#736）で deserialize は新旧両綴りを受容する状態に
なったが、emit 側の不一致が残存し、plugin-dock-test の evidence
JSONL が lane record 検証で未知語彙として拒否される状態が
継続していた。移行方針自体は Phase A design
（`docs/designs/issue-725-serde-vocab-phase-a.md`、approved）で
確定済みであり、本 ADR はその最終形 — canonical wire 綴りを
`test-plugin` に統一する決定 — を記録する。

## Considered Options

* `#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]`
  + `as_str()` → `"test-plugin"`（canonical = schema 語彙、
  旧綴りは deserialize alias で恒久的に受容）
* `#[serde(rename = "real-abi-test-plugin", alias = "test-plugin")]`
  のまま据え置き（Phase A 状態の継続 — schema 語彙との不一致が
  残る）
* custom Serialize/Deserialize impl で両綴りを対称に扱う
* `EnvironmentKind` 型名・variant 名自体の rename

## Decision Outcome

Chosen option: "`#[serde(rename = "test-plugin", alias =
"real-abi-test-plugin")]` + `as_str()` → `"test-plugin"`",
because schema 正本（`ci/consumer-verify`・issue-627/294 契約
文書）が `test-plugin` で一貫しており、flip は schema への収束
である。旧綴り `real-abi-test-plugin` を deserialize alias と
して恒久的に受容することで、historical evidence（過去 run の
SHA 付き記録）と flip 前に emit された JSON 文書の読み戻しを
維持する。`environment_kind_serde_matches_as_str` が serde emit
== `as_str()` を強制するため、serde `rename` と `as_str()` の
flip は同一 slice で atomic に行い、`ruviform-plugin-dock-test`
の literal assert も同時に更新する。型名 `RealAbiTestPlugin`
自体は rename しない（Issue の「触らない」範囲 — serde 語彙のみ
が対象）。

## Implementation Plan

* Affected paths: `crates/ruviform-audio-test/src/metadata.rs`、
  `crates/ruviform-audio-test/tests/metadata.rs`、
  `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`、
  `crates/ruviform-plugin-dock-test/Cargo.toml`（コメント）、
  `docs/{designs,specs,test-specs}/issue-629-*.md`（status 追記
  のみ）、`evidence/rf725/`、docs chain。
* Constraints: `unsafe` なし・依存追加なし・`#[non_exhaustive]`
  と variant 集合は不変・historical evidence は書き換えない・
  docs index は Coordinator が統合時に再生成・`ci/consumer-
  verify/**`・`.github/**`・consumer repo は変更しない。

## Verification

* [x] `mise run check|lint|test ruviform-audio-test` — pass
* [x] `mise run check|lint|test ruviform-plugin-dock-test` — pass
  （cdylib 解決可否による degrade 経路の実効範囲は evidence に記録）
* [x] `mise run test-ci-consumer-verify` — schema 側非回帰 pass
* [x] `mise run fmt-check`・`mise exec -- prek run --all-files` —
  pass
* [ ] `*-full`/`*-msrv`/3OS 横断・opt-in lane 実走 — NotRun
  （daily CI 委譲・運用判断）
