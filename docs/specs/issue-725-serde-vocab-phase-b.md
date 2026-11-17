---
id: "SPEC-034Z7dKIl2FnoV81lYbnhI"
type: "spec"
status: "approved"
title: "Issue #725 EnvironmentKind serde 語彙 gap 解消 Phase B — emit flip（test-plugin 統一）"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/725"
    - "docs/discovery/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
    - "evidence/rf725/consumer-impact-assessment.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-725-serde-vocab-phase-b.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/discovery/issue-725-serde-vocab-phase-b.md"
  implemented-by:
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
    - "docs/plans/issue-725-serde-vocab-phase-b.md"
  depends-on:
    - "docs/discovery/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #725 EnvironmentKind serde 語彙 gap 解消 Phase B — emit flip（test-plugin 統一）

## What Should Be Built

`EnvironmentKind::RealAbiTestPlugin` の serde emit 側を Case Schema
v1 / consumer-verify 契約語彙 `test-plugin` に統一する（Phase A
設計の「第2 slice」）: `#[serde(alias = "test-plugin")]` を
`#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]`
へ切替え、`as_str()` の emit 綴りを `"test-plugin"` に flip する。
旧綴り `real-abi-test-plugin` は deserialize alias として恒久的に
受容し続ける（historical evidence の読み戻しを維持）。あわせて
`ruviform-plugin-dock-test` の literal assert・コメント、
`ruviform-audio-test` の emit pin テスト、issue-629 docs の gap
status（解消済み）、docs chain、evidence を更新・記録する。

## Why Now

Phase A（PR#736）で deserialize 側の後方互換は着地したが、emit
側は `real-abi-test-plugin` のままで schema 正本
`ci/consumer-verify` の `test-plugin` と不一致が残存している。
旧直列要因の rf#631 catalog flip が PR#741 で merge・lease 解除
済みのため、`crates/ruviform-plugin-dock-test/` の literal
assert 更新を含む atomic flip が実行可能になった。本 slice の
merge で Issue #725 の全 AC が満たされる見込み。

## Users and Value

- `ruviform-plugin-dock-test` の evidence JSONL: emit される
  `environment_kind` が schema 語彙 `test-plugin` となり、
  consumer-verify の lane record 検証語彙と一致する。
- `RunMetadata`/`EnvironmentKind` の consumer・将来の証拠 lane:
  canonical wire 綴りが契約語彙に揃い、新旧綴りの混在が解消
  される（旧綴り文書は引き続き deserialize 可能）。

## Scope

### In Scope

- `crates/ruviform-audio-test/src/metadata.rs`:
  `RealAbiTestPlugin` の serde attr を
  `#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]`
  へ切替、`as_str()` を `"test-plugin"` に flip、enum doc・
  variant doc を Phase B 完了状態に更新。型名
  `RealAbiTestPlugin` は変更しない。
- `crates/ruviform-audio-test/tests/metadata.rs`: emit pin の
  期待値を `test-plugin` に更新（wire spellings・deserialize
  両綴り受容・`RunMetadata` re-emit の 3 テスト）。
  `environment_kind_serde_matches_as_str` は不変条件として
  変更しない。
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`:
  :168・:211 の literal assert と assert メッセージを
  `test-plugin` に更新。
- `crates/ruviform-plugin-dock-test/Cargo.toml`: :77-79 コメントの
  語彙言及を `test-plugin` に更新。
- issue-629 docs 5 ファイルへの「解消済み」status 追記
  （append-only — flip 実施・検証後に実施）。
- `evidence/rf725/` への Phase B 検証記録。
- docs chain（discovery/spec/design/plan/task/test-spec/impl）と
  `docs/impl/exp/` companion 記録。

### Out of Scope

- `ci/consumer-verify/**`・`.github/workflows/**` の変更
  （schema 正本は既に `test-plugin` — flip は schema への収束で
  contract 側の変更は不要。read-only 参照のみ）。
- consumer 側 pin bump 実作業（影響評価で不要と判定済み —
  必要なら別 Issue）。
- historical evidence（`evidence/rf389*/`・`evidence/rf631/` 等）
  の書き換え — 過去 run の SHA 付き記録として履歴のまま残す
  （旧綴りは deserialize alias で恒久的に受容）。
- `EnvironmentKind` 型名自体の rename・他 enum/語彙の統一・
  `RealAbiTestPlugin` 以外の variant 変更。
- `crates/ruviform-plugin-dock-test/src/scenario.rs`・
  `src/real_dock.rs` 等の lease 外ファイル（`scenario.rs`:280 は
  `as_str()` flip に自動追従、`real_dock.rs` は `"real-window"`
  比較のみで非影響）。
- stale merge 済み branch `scarecrow173/rf725-serde-vocab` の
  削除・再利用（破壊的 Git 操作 — ユーザー判断）。
- opt-in lane（`test-ui-plugin-dock*` 系）の実走と新規 evidence
  JSONL 再採取（CI 必須 gate でなく運用判断）。
- 依存の追加・変更（`serde` derive は既存で十分）。
- docs index（`docs/**/README.md`）の更新 — Coordinator が統合時に
  再生成。

## Requirements

### R1: emit flip の atomic 実装（AC3 emit 側）

`metadata.rs` で serde canonical 綴りを `test-plugin` に切替え
（`rename` + 旧綴り `alias` 受容継続）、`as_str()` を
`"test-plugin"` に flip する。enum doc・variant doc を Phase B
完了状態に更新する。

### R2: pin テストの更新（AC3 emit 側 + AC4）

`tests/metadata.rs` の emit 期待値を `test-plugin` に flip し、
schema 語彙一致の pin を全 variant で成立させる。deserialize
両綴り受容・未知語彙拒否・round-trip の pin は維持する。

### R3: plugin-dock-test の literal 更新（atomic 必須）

`tests/plugin_dock.rs` の綴り assert と `Cargo.toml` コメントを
`test-plugin` に更新する。cdylib 未解決環境では degrade 経路
のみ通るため、実効確認の可否を検証結果として正直に記録する。

### R4: issue-629 docs の gap status 更新（追記のみ）

5 ファイルの gap 記述へ、emit flip 実施・検証済みの旨を
append-only で追記する。flip が実装・検証されるまで「解消済み」
とは記載しない（#629 design Risks 表の誤記パターン回避）。

### R5: 正規検証（AC4/AC5）

`mise run check|lint|test ruviform-audio-test`・
`mise run check|lint|test ruviform-plugin-dock-test`・
`mise run test-ci-consumer-verify`（schema 側非回帰）・
`mise run fmt-check`・`mise exec -- prek run --all-files` を
実行し exit code を記録する。

## Acceptance Criteria

Issue #725 からの転記に slice 適用を加えたもの:

- [x] consumer（AudioLens / Synvelis の `ruviform-audio-test`
  消費箇所）への影響評価を実施し、結果を記録する（Phase A・
  `evidence/rf725/consumer-impact-assessment.md`）
- [x] serde 移行方針（rename+alias 等）を決定し、設計文書または
  evidence に記録する（Phase A design・approved）
- [ ] `real-abi-test-plugin` ↔ `test-plugin` の語彙不一致が解消
  され、新旧語彙の移行期間中の挙動が定義されている（本 slice で
  emit 側を解消 — deserialize は新旧両綴りを恒久的に受容、
  historical evidence は書き換えない）
- [ ] `mise run test` 系の正規検証が pass する
- [ ] 変更が証拠 lane（`ci/` 系の EnvironmentKind を参照する経路）
  で非回帰であることを確認する

## Deferred Design Concerns

- consumer 側 pin bump: 影響評価の結果不要と判定。再開条件:
  consumer が `metadata` module を消費し始めた場合。
- opt-in lane（`test-ui-plugin-dock*`）での実効 evidence 再採取:
  CI 必須 gate でなく運用判断。必要と判断した場合は Coordinator
  に確認する。

## Sources and Relations

- Issue: `scarecrow173/ruviform#725`・`#629`（発見元・gap 記録）
- Phase A chain: `docs/*/issue-725-serde-vocab-phase-a.md`・
  `docs/impl/exp/issue-725-serde-vocab-phase-a-exp.jsonl`
- 移行方針の正本: `docs/designs/issue-725-serde-vocab-phase-a.md`
  （approved — 移行期間中の挙動・atomic flip・第2 slice 手順）
- schema 正本（read-only）: `ci/consumer-verify/evidence.mjs`・
  `docs/designs|specs/issue-627-*`・`issue-294-*`
- gap 記録（追記のみ）: `docs/designs|specs|test-specs/
  issue-629-*.md` 5 ファイル
- 発見・前提確認: `docs/discovery/issue-725-serde-vocab-phase-b.md`
- consumer 影響評価: `evidence/rf725/consumer-impact-assessment.md`
- Phase B 検証記録: `evidence/rf725/`（検証実行後に追記）
