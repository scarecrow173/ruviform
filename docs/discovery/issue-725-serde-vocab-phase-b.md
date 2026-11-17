---
id: "DISC-034Z7dvDpqTYOW3pCKWH8I"
type: "discovery"
status: "resolved"
title: "Issue #725 EnvironmentKind serde 語彙 gap 解消 Phase B — emit flip"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/725"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "evidence/rf725/consumer-impact-assessment.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-725-serde-vocab-phase-b.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-725-serde-vocab-phase-b.md"
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

# Issue #725 EnvironmentKind serde 語彙 gap 解消 Phase B — emit flip

## Summary

`EnvironmentKind::RealAbiTestPlugin` の serde 語彙 gap
（emit 側 `real-abi-test-plugin` ↔ schema 正本 `test-plugin`）の
残件である emit flip を実施する slice の調査記録。Phase A
（PR#736・merge commit `49c41399`）で deserialize 側 alias 受容と
移行方針（`docs/designs/issue-725-serde-vocab-phase-a.md`、
status: approved）が確定済みで、本 slice はその「第2 slice の
作業範囲」（同 design :100-111）をそのまま実行する。旧直列要因
だった rf#631 catalog flip は PR#741（`d530ee76`）で merge 済み・
占有 lease 解除済みのため、`crates/ruviform-plugin-dock-test/`
の literal assert 更新を同一 slice で行える（confirmed）。

## Exploration Goal

Phase A design が定めた第2 slice の作業範囲が、merge 済みの
main（`1d18e297`）時点でそのまま実行可能かを実コードで再確認し、
atomic flip の対象箇所・検証入口・記録先を確定する。

## Key Issues and Assumptions

- `environment_kind_serde_matches_as_str`（`tests/metadata.rs`:
  169-183）が serde emit == `as_str()` を全 variant で強制する
  → serde `rename` 切替と `as_str()` flip は同一 slice で不可分
  （Phase A design :93-98 どおり）。
- `crates/ruviform-plugin-dock-test/src/scenario.rs`:280 が
  `json!(kind.as_str())` で emit 綴りを evidence JSONL に流す
  → `as_str()` flip に自動追従し、同ファイルの編集は不要
  （lease 外でもある）。
- `tests/plugin_dock.rs`:168,211 と `Cargo.toml`:77-79 が
  `real-abi-test-plugin` を literal pin/言及 → flip と同時更新が
  必須。rf#631 lease は解除済みで編集可能。
- schema 正本 `ci/consumer-verify/evidence.mjs`:73-79
  `ENVIRONMENT_KINDS` は既に `test-plugin` — flip は schema への
  収束であり contract 側の変更は不要（read-only 確認済み）。
- consumer（audio-lens・synvelis）は `metadata` module 未消費で
  影響ゼロ（`evidence/rf725/consumer-impact-assessment.md`、
  AC1）→ consumer pin bump 不要。
- `plugin_dock.rs`:168,211 の assert は test_gain cdylib が
  `deps/` に解決できる環境でのみ実行される fail-closed degrade
  設計（:137-149 early-return）→ headless 環境では degrade 経路
  のみ通る可能性があり、実効確認の可否を検証結果として正直に
  記録する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Phase B 設計どおり atomic flip（rename+alias 切替 + as_str flip + plugin-dock-test 更新を同一 slice） | gap を完全解消・schema 語彙に収束・全 pin が整合 | plugin-dock-test の cdylib 依存 assert は環境により degrade | yes |
| serde rename のみ先行（as_str 据え置き） | — | `environment_kind_serde_matches_as_str` が即失敗。emit 経路の authoritative 入口は `as_str()` で乖離が残る | no |
| `as_str()` のみ先行 | — | serde emit と乖離し同テストが失敗。逆方向の不整合 | no |
| custom Serialize/Deserialize impl | — | `rename`+`alias` で要件充足。保守対象を増やすだけ（Phase A design で却下済み） | no |
| stale branch `scarecrow173/rf725-serde-vocab` の再利用 | — | main に完全 merge 済み・Spec が新規 branch `feature/725-serde-vocab-phase-b` を指定。再利用・削除ともに不可 | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: Phase A design の第2 slice 手順がそのまま実行
  可能。対象行・lease・検証入口は Spec 記載どおり。
- **confirmed**: `EnvironmentKind` 型名自体は rename しない
  （Issue「触らない」— serde 語彙のみ対象）。
- **confirmed**: `#[serde(rename = "test-plugin", alias =
  "real-abi-test-plugin")]` で canonical wire 綴りを
  `test-plugin` に切替え、旧綴りは deserialize alias で恒久的に
  受容（historical evidence の読み戻しを維持）。
- **hypothesis→検証で確定**: headless 環境で `plugin_dock.rs` の
  綴り assert が実効実行されるかは cdylib 解決可否に依存 —
  degrade のみの場合は NotRun として PR 本文に明記する。

## Open Questions

- なし。emit flip の方式・範囲・検証は Phase A design（approved）
  と Task Spec で確定済み。

## Promotion Candidates

- [x] spec-doc needed: Phase B の受入条件（emit flip・pin 更新・
  issue-629 status 解消記録・非回帰検証）を
  `docs/specs/issue-725-serde-vocab-phase-b.md` に正式化する。
- [ ] adr-doc needed: 移行方針は Phase A design で確定済み。
  Phase B はその実行 slice であり新規のアーキテクチャ決定を
  含まないため ADR は作成しない（Phase A と同じ判断）。

## Sources and Relations

- Issue: `scarecrow173/ruviform#725`（本 slice）・`#629`（発見元・
  gap 記録の追記先）
- Phase A chain: `docs/discovery|specs|designs|plans|tasks|
  test-specs|impl/*/issue-725-serde-vocab-phase-a.md`・
  `docs/impl/exp/issue-725-serde-vocab-phase-a-exp.jsonl`
- 移行方針の正本: `docs/designs/issue-725-serde-vocab-phase-a.md`
  （status: approved — 移行期間中の挙動 :81-87・atomic flip
  :93-98・第2 slice 手順 :100-111）
- consumer 影響評価: `evidence/rf725/consumer-impact-assessment.md`
- schema 正本（read-only）: `ci/consumer-verify/evidence.mjs`・
  `.github/workflows/consumer-verify.yml`・
  `docs/designs|specs|test-specs/issue-627-*`・`issue-294-*`
- gap 記録（追記のみ）: `docs/designs|specs|test-specs/
  issue-629-*.md` 5 ファイル

## Evidence

### 2026-11-17 — Phase A merge と前提の再確認

- Phase A は PR#736（merge commit `49c41399`）で merge 済み。
  `metadata.rs`:116 に `#[serde(alias = "test-plugin")]` 適用済み、
  :145 `as_str()` は `"real-abi-test-plugin"` のまま（emit gap
  残存を main `1d18e297` で確認）。
- rf#631 catalog flip は PR#741（`d530ee76`）で merge 済み —
  `crates/ruviform-plugin-dock-test` の編集 lease は本 slice に
  付与済み。
- `git worktree list` で stale branch `scarecrow173/
  rf725-serde-vocab` は残存するが main に完全 merge 済み —
  本 slice は新規 branch `feature/725-serde-vocab-phase-b`
  （worktree `G:/orca/workspaces/ruviform/rf725-serde-vocab-
  phase-b`）で実施し、stale branch は再利用・削除しない。

### 2026-11-17 — emit flip 対象の実測

- `metadata.rs`:116 `#[serde(alias = "test-plugin")]` →
  `#[serde(rename = "test-plugin", alias = "real-abi-test-
  plugin")]` へ切替、:145 `as_str()` `"real-abi-test-plugin"`
  → `"test-plugin"`。
- `tests/metadata.rs`: `environment_kind_wire_spellings_match_
  case_schema_v1`（:186-204）に `RealAbiTestPlugin` →
  `test-plugin` pin 追加、`environment_kind_deserialize_accepts_
  both_vocab_spellings`（:206-232）の emit 期待値を
  `"test-plugin"` に flip（deserialize 両綴り受容 assert は維持）、
  `run_metadata_from_json_accepts_test_plugin_spelling`
  （:234-253）の re-emit 期待値を `"test-plugin"` に flip。
  `environment_kind_serde_matches_as_str`（:169-183）は不変条件
  として変更しない。
- `plugin_dock.rs`:168 `"real-abi-test-plugin"` →
  `"test-plugin"`、:211 `Some("real-abi-test-plugin")` →
  `Some("test-plugin")`。`Cargo.toml`:77-79 コメントの語彙言及を
  更新。
- `scenario.rs`:280 `json!(kind.as_str())` は `as_str()` flip に
  自動追従 — 編集不要を確認（lease 外・read-only 参照）。
- `real_dock.rs` の `environment_kind` 比較は `"real-window"`
  のみで非影響（lease 外・変更不要）。
- issue-629 docs 5 ファイルの gap 記述（Phase A が status 追記
  済みの 13 箇所）へ「解消済み」の status を flip 実施・検証後に
  追記（append-only）。
