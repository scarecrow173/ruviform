---
id: "SPEC-034YZVNYlydMg2rp3mE6v7"
type: "spec"
status: "approved"
title: "Issue #725 EnvironmentKind serde 語彙 gap 解消 Phase A — deserialize alias 受容"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/725"
    - "docs/discovery/issue-725-serde-vocab-phase-a.md"
    - "evidence/rf725/consumer-impact-assessment.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-725-serde-vocab-phase-a.md"
  implements:
    - "docs/discovery/issue-725-serde-vocab-phase-a.md"
  implemented-by:
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
    - "docs/plans/issue-725-serde-vocab-phase-a.md"
  depends-on:
    - "docs/discovery/issue-725-serde-vocab-phase-a.md"
  blocks: []
---

# Issue #725 EnvironmentKind serde 語彙 gap 解消 Phase A — deserialize alias 受容

## What Should Be Built

`EnvironmentKind::RealAbiTestPlugin` の serde 語彙 gap
（emit 側 `real-abi-test-plugin` ↔ schema 正本 `test-plugin`）に対し、
Phase A として deserialize 側の後方互換 layer を実装する:
`RealAbiTestPlugin` variant に `#[serde(alias = "test-plugin")]` を
付与し、deserialize が新旧両綴りを受容するようにする。serialize
および `as_str()` の emit 綴りは `real-abi-test-plugin` のまま
維持する（emit flip は第2 slice へ分離）。あわせて consumer 影響
評価の記録・移行方針の設計文書化・issue-629 docs の gap status
追記を行う。

## Why Now

#629 provisioning slice で検出・記録された語彙 gap の残件。
Issue #725 の再開条件（consumer 影響評価 + 承認）の評価部分は
`evidence/rf725/consumer-impact-assessment.md` で完了 — 両
consumer（audio-lens・synvelis）は `metadata` module を未消費で
影響ゼロ。schema 正本 `ci/consumer-verify` は `test-plugin` を
契約語彙とし、emit 側 `real-abi-test-plugin` は lane record 検証で
未知語彙として拒否されるため、移行は避けられない。

## Users and Value

- `ruviform-audio-test` の `RunMetadata`/`EnvironmentKind` を使う
  consumer・将来の証拠 lane: schema 語彙 `test-plugin` の JSON 文書を
  deserialize できる後方互換を得る。
- ruviform repo: 語彙 gap の解消を 2 slice に分割し、rf#631 占有の
  `ruviform-plugin-dock-test` 更新を待たずに Phase A を先行着地
  できる。

## Scope

### In Scope

- `crates/ruviform-audio-test/src/metadata.rs`:
  `EnvironmentKind::RealAbiTestPlugin` への
  `#[serde(alias = "test-plugin")]` 付与と enum doc の Phase A
  状態への更新。`as_str()`・serde emit 綴りは不変。
- `crates/ruviform-audio-test/tests/metadata.rs`: alias 受容の
  pin テスト追加（`EnvironmentKind` 単体 + `RunMetadata` JSON
  文書の両方）。既存 emit 綴り pin テストは維持。
- `evidence/rf725/consumer-impact-assessment.md`: consumer 影響
  評価の記録（作成済み）。
- `docs/designs/issue-725-serde-vocab-phase-a.md`: serde 移行方針
  （Phase A = deserialize alias、emit flip = 第2 slice）の決定
  記録。移行期間中の挙動定義を含む。
- issue-629 docs 5 ファイルへの gap status 追記（append-only）。
- docs chain（discovery/spec/design/plan/task/test-spec/impl）と
  `docs/impl/exp/` の companion 記録。

### Out of Scope

- emit flip（`rename = "test-plugin"` + `as_str()` →
  `"test-plugin"`）— `crates/ruviform-plugin-dock-test/`（rf#631
  占有）の literal assert・emit 経路更新を必須とするため第2 slice
  へ分離。
- `crates/ruviform-plugin-dock-test/`・`crates/ruviform-plugin-
  test-vst3/`・`crates/ruviform-plugin-test-clap/`・
  `crates/ruviform-plugin-clap/`・`crates/ruviform-plugin-core/`・
  `ci/plugin-abi-validator/`・`ci/consumer-verify/**`・
  `.github/**` の変更（全て read-only 参照または他 Issue 占有）。
- consumer 側 pin bump 実作業（影響評価の結果、不要と判定 —
  必要になれば別 Issue）。
- historical evidence（`evidence/rf389*/`・`evidence/rf631/` 等）
  の書き換え — 過去 run の SHA 付き記録として履歴のまま残す。
- `EnvironmentKind` 型名自体の rename・他 enum/語彙の統一。
- docs index（`docs/**/README.md`）の更新 — Coordinator が統合時に
  再生成。

## Requirements

### R1: consumer 影響評価（AC1）

AudioLens / Synvelis の `ruviform-audio-test` 消費箇所を read-only
で照合し、結果を `evidence/rf725/consumer-impact-assessment.md`
に記録する。emit flip 後に consumer 側 pin bump が要るかの判断も
評価記録に残す。

### R2: serde 移行方針の決定・記録（AC2）

`docs/designs/issue-725-serde-vocab-phase-a.md` に記録する:
Phase A = deserialize 側 alias 受容、第2 slice = emit flip。
移行期間中の挙動を定義する: (a) deserialize は新旧両語彙を受容、
(b) serialize/`as_str()` emit は旧綴り `real-abi-test-plugin` を
維持、(c) historical evidence JSONL は過去 run 記録として書き換え
ない、(d) schema 語彙 `test-plugin` が最終正本。

### R3: deserialize alias 実装 + テスト pin（AC3 前半）

`RealAbiTestPlugin` に `#[serde(alias = "test-plugin")]` を付与。
`"test-plugin"` → `RealAbiTestPlugin`・`"real-abi-test-plugin"` →
`RealAbiTestPlugin` の deserialize 受容を `EnvironmentKind` 単体と
`RunMetadata` JSON 文書（`from_json` round-trip）の両方で pin
する。enum doc の「Known vocabulary gap」記述を Phase A の状態に
更新する。`as_str()` と serde emit 綴りは変更しない。

### R4: issue-629 docs の gap status 更新（追記のみ）

`docs/designs/issue-629-virtual-endpoint-lane.md`・
`docs/designs/issue-629-provisioning-runbook.md`・
`docs/specs/issue-629-virtual-endpoint-lane.md`・
`docs/specs/issue-629-provisioning-runbook.md`・
`docs/test-specs/issue-629-virtual-endpoint-lane.md` の gap 記述に
rf#725 Phase A の状態（deserialize alias 適用・emit flip は第2
slice）を追記する。元記録の書き換え・削除はしない。

### R5: 正規検証（AC4/AC5）

`mise run check ruviform-audio-test`・`mise run lint
ruviform-audio-test`・`mise run test ruviform-audio-test`・
`mise run fmt-check`・`mise run test-ci-consumer-verify`（schema
側非回帰）を実行し exit code を記録する。

## Acceptance Criteria

Issue #725 からの転記に slice 適用を加えたもの:

- [ ] consumer（AudioLens / Synvelis の `ruviform-audio-test`
  消費箇所）への影響評価を実施し、結果を記録する
- [ ] serde 移行方針（rename+alias 等）を決定し、設計文書または
  evidence に記録する
- [ ] `real-abi-test-plugin` ↔ `test-plugin` の語彙不一致に対し、
  移行期間中の挙動（deserialize 両綴り受容・emit は旧綴り維持・
  historical evidence 不変）が定義され、deserialize 側の後方互換
  が実装・pin される（**emit 側の統一は第2 slice — 本 slice では
  「gap 解消済み」と主張しない**）
- [ ] `mise run test` 系の正規検証が pass する
- [ ] 変更が証拠 lane（`ci/` 系の EnvironmentKind を参照する経路）
  で非回帰であることを確認する

## Deferred Design Concerns

- emit flip（出力語彙の `test-plugin` 統一）: `#[serde(rename =
  "test-plugin", alias = "real-abi-test-plugin")]` への切替 +
  `as_str()` flip + `crates/ruviform-plugin-dock-test/` の
  literal assert・コメント更新を必須とする。再開条件: rf#631
  lease 解除後または Coordinator 調整。Issue #725 の完全 close
  には本 slice が必須。
- consumer 側 pin bump: 影響評価の結果不要と判定。再開条件:
  consumer が `metadata` module を消費し始めた場合、または emit
  flip 後に consumer 側で語彙を参照する経路が生じた場合。

## Sources and Relations

- Issue: `scarecrow173/ruviform#725`・`#629`（発見元）・`#389`
  （占有解除済み）・PR #711（未検証項目の出所）
- consumer 照合: `scarecrow173/audio-lens#127`・
  `scarecrow173/synvelis#28`・`scarecrow173/synvelis#126`
- schema 正本: `ci/consumer-verify/evidence.mjs`・
  `docs/designs/issue-627-virtual-first-verification-contract-
  design.md`・`docs/specs|designs/issue-294-virtual-first-evidence-
  schema.md`（read-only）
- gap 記録: `docs/designs|specs|test-specs/issue-629-*.md`
