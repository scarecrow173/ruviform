---
id: "DISC-034YZUNuibc7hwIC9PXWm9"
type: "discovery"
status: "resolved"
title: "Issue #725 EnvironmentKind serde 語彙 gap — consumer 影響評価と移行方針（Phase A）"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/725"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "evidence/rf725/consumer-impact-assessment.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  related:
    - "../specs/issue-725-serde-vocab-phase-a.md"
---

# Issue #725 EnvironmentKind serde 語彙 gap — consumer 影響評価と移行方針（Phase A）

## Summary

`EnvironmentKind::RealAbiTestPlugin` の serde 綴り
`real-abi-test-plugin` と Case Schema v1 / consumer-verify 契約語彙
`test-plugin` の不一致（#629 slice で検出・記録された既知 gap）を
解消するための調査記録。consumer 影響評価の結果、両 consumer
（audio-lens・synvelis）は `metadata` module を一切消費しておらず
影響ゼロと確認。移行方針は Phase A = deserialize 側 alias 受容、
第2 slice = emit flip とする 2 段階を推奨する（confirmed）。

## Exploration Goal

Issue #725 の再開条件「consumer 影響評価 + 承認」のうち、評価
部分を実コードで確定する。あわせて serde 移行方針（rename+alias
等）の選択肢を比較し、Phase A slice で実施する範囲と第2 slice
へ分離する範囲を切り分ける。

## 語彙 gap の実測確認

- emit/serde 側: `crates/ruviform-audio-test/src/metadata.rs`
  `EnvironmentKind::RealAbiTestPlugin` は `#[serde(rename_all =
  "kebab-case")]` により `real-abi-test-plugin` を emit し、
  `as_str()` も同綴りを返す。
- schema 正本（read-only）: `ci/consumer-verify/evidence.mjs`
  `ENVIRONMENT_KINDS` は `test-plugin`。lane record の
  `environment_kind` 検証（`evidence.mjs`:362）で
  `real-abi-test-plugin` は未知語彙として拒否される。
- emit 経路: `crates/ruviform-plugin-dock-test/src/scenario.rs`
  が `json!(kind.as_str())` で evidence JSONL に流し、
  `tests/plugin_dock.rs`:168,211 が `real-abi-test-plugin` を
  literal assert（rf#631 占有 — 本 slice では変更不可）。

## Key Issues and Assumptions

- consumer が `metadata` module を消費している可能性 → **反証済み**:
  両 consumer の `ruviform-audio-test` 消費は `oracle`/`pcm`/
  `backend`/`clock` のみ（`evidence/rf725/consumer-impact-
  assessment.md` に実測記録）。
- `#[serde(alias)]` が serialize 出力を変えないこと → serde の
  仕様上 alias は deserialize 側の追加受容名であり、serialize は
  canonical（`rename_all` 由来）を維持する。pin テストで固定する。
- `environment_kind_serde_matches_as_str` テストが serde emit ↔
  `as_str()` の一致を強制する設計 → emit 側を Phase A で動かすと
  このテストが即座に失敗するため、Phase A は deserialize alias のみ
  とし emit 綴りを不変に保つのが整合的な分割（confirmed）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Phase A: `#[serde(alias = "test-plugin")]` のみ（emit 不変） | 後方互換・consumer 影響ゼロ・plugin-dock-test 不変で merge 可能 | emit 側の gap は残る（第2 slice 必須） | yes |
| 一括 flip: `rename = "test-plugin"` + `as_str()` flip | gap を一度に解消 | plugin-dock-test（rf#631 占有）の literal assert・emit 経路の同時更新が必須で現 slice では不可 | no（第2 slice） |
| custom Deserialize/Serialize impl | 細かい制御 | `rename_all`+`alias` で足りる要件に対し過剰・保守コスト増 | no |
| `as_str()` のみ変更 | — | serde emit と乖離し `environment_kind_serde_matches_as_str` が壊れる。不整合を生む | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: consumer 影響はゼロ。両 consumer は
  `metadata`/`EnvironmentKind`/`RunMetadata`/`environment_kind`
  を未消費であり、語彙変更は compile・証拠経路ともに無影響
  （`evidence/rf725/consumer-impact-assessment.md`）。
- **confirmed**: schema 語彙 `test-plugin` が最終正本
  （`ci/consumer-verify`・issue-627/294 契約文書が一貫）。
- **hypothesis→design で確定**: Phase A = deserialize alias のみ、
  emit flip は rf#631 占有解除後の第2 slice。移行期間中の挙動
  （deserialize 両綴り受容・emit は旧綴り維持・historical
  evidence は書き換えない）を design 文書に定義する。

## Open Questions

- なし（consumer 影響評価でブロッカーは解消。emit flip のタイミングは
  rf#631 lease 状況に依存し Coordinator 調整事項として design/PR に記録）。

## Promotion Candidates

- [x] spec-doc needed: Phase A の受入条件（alias 受容・emit 不変・
  consumer 影響評価記録）を `docs/specs/issue-725-serde-vocab-phase-a.md`
  に正式化する。
- [ ] adr-doc needed: serde 語彙移行は契約語彙の変更だが、決定内容は
  「deserialize alias → emit flip」の 2 段階移行であり、issue-721
  前例（設計判断は `docs/designs/` に記録）に倣い design 文書で
  記録する。MADR 級の恒久的アーキテクチャ決定ではないため ADR は
  作成しない判断（exp jsonl に記録）。

## Sources and Relations

- Issue: `scarecrow173/ruviform#725`（本 slice）・`#629`（発見元）・
  `#389`（占有解除済み）
- consumer 照合 Issue: `scarecrow173/audio-lens#127`・
  `scarecrow173/synvelis#28`・`scarecrow173/synvelis#126`
- schema 正本: `ci/consumer-verify/evidence.mjs`（read-only）
- 契約語彙出典: `docs/designs/issue-627-virtual-first-verification-
  contract-design.md`・`docs/specs|designs/issue-294-virtual-first-
  evidence-schema.md`（read-only）
- gap 記録: `docs/designs|specs|test-specs/issue-629-*.md`（追記のみ）

## Evidence

### 2026-11-17 — consumer 影響評価（read-only 実測）

- audio-lens: `crates/al-analysis` dev-dep。消費は
  `tests/rf628_oracle.rs` の `oracle`/`pcm` module のみ。
  `metadata`/`EnvironmentKind`/`environment_kind` 使用ゼロ。
  pin: ruviform `cbcc83437616fdd7418da37183c6be66b6cfe225`。
- synvelis: `crates/synvelis-recording` dev-dep。消費は
  `tests/rf628_oracle.rs`・`tests/rf628_oracle_rt_alloc.rs` の
  `backend`/`clock`/`oracle`/`pcm` のみ。metadata 系使用ゼロ。
  `test-plugin` 文字列 2 件は `PluginRef.descriptor` で無関係。
  pin: 同上 `cbcc8343…`。
- 詳細・検証コマンド: `evidence/rf725/consumer-impact-assessment.md`。

### 2026-11-17 — emit flip が第2 slice 必須の根拠

- `crates/ruviform-plugin-dock-test/src/scenario.rs`:280 が
  `json!(kind.as_str())` で emit 綴りを evidence JSONL に流す。
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`:168,211 が
  `"real-abi-test-plugin"` を literal assert — flip で直撃失敗。
  同クレートは rf#631 占有のため本 slice の編集範囲外。
- `tests/metadata.rs` の `environment_kind_serde_matches_as_str`
  が serde emit ↔ `as_str()` 一致を強制 — serde `rename` と
  `as_str()` の flip は同時に行う必要がある。
- historical evidence（`evidence/rf389*/`・`evidence/rf631/` 等）
  の JSONL に旧綴りの記録が残存 — 過去 run の SHA 付き記録であり
  書き換えない（design で扱いを明文化）。
