---
id: "DESIGN-034YZXlJnaKqZu7Z6E9Lmy"
type: "design"
status: "approved"
title: "Issue #725 EnvironmentKind serde 語彙移行方針 — Phase A deserialize alias"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/discovery/issue-725-serde-vocab-phase-a.md"
    - "evidence/rf725/consumer-impact-assessment.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-725-serde-vocab-phase-a.md"
  implements:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
  implemented-by:
    - "docs/plans/issue-725-serde-vocab-phase-a.md"
  depends-on:
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/discovery/issue-725-serde-vocab-phase-a.md"
  blocks:
    - "docs/plans/issue-725-serde-vocab-phase-a.md"
  supersedes: []
---

# Issue #725 設計 — EnvironmentKind serde 語彙移行方針（Phase A）

## Context

`EnvironmentKind::RealAbiTestPlugin` は `#[serde(rename_all =
"kebab-case")]` により wire 綴り `real-abi-test-plugin` を emit
するが、Case Schema v1 / consumer-verify 契約語彙の正本は
`test-plugin`（`ci/consumer-verify/evidence.mjs` `ENVIRONMENT_KINDS`、
issue-627/294 契約文書、`.github/workflows/consumer-verify.yml`
静的 lane 行で一貫）。#629 slice で検出されたこの gap の解消が
本設計の対象。

consumer 影響評価（`evidence/rf725/consumer-impact-assessment.md`、
AC1）は完了しており、audio-lens・synvelis は `metadata` module を
未消費で影響ゼロ。emit flip は `crates/ruviform-plugin-dock-test/`
（rf#631 占有）の literal assert・emit 経路更新を必須とするため、
本 slice（Phase A）と第2 slice に分割する。

## Scope

### In Scope

- `crates/ruviform-audio-test/src/metadata.rs`:
  `#[serde(alias = "test-plugin")]` 付与・enum doc 更新。
- `crates/ruviform-audio-test/tests/metadata.rs`: alias 受容 pin。
- 移行方針の決定記録（本書）・issue-629 docs への status 追記・
  docs chain・evidence。

### Out of Scope

- emit flip・`as_str()` flip・`crates/ruviform-plugin-dock-test/`
  他占有クレート・`ci/`・`.github/**`・consumer pin bump・
  historical evidence 書き換え・型名 rename・docs index。

## Components and Boundaries

- **`metadata.rs`（変更）**: `EnvironmentKind` enum が語彙の
  authoritative な所有者。deserialize の受容集合をここで拡張する。
- **`tests/metadata.rs`（変更）**: 受容集合と emit 綴り不変を
  pin するテスト層。
- **`ci/consumer-verify/`（read-only）**: schema 語彙の正本。
  `test-plugin` が契約値であり変更不要。
- **`crates/ruviform-plugin-dock-test/`（read-only、rf#631 占有）**:
  emit 側の直接 consumer。`as_str()` 綴りを evidence に流し
  literal assert で pin しているため、emit flip は同クレートの
  更新と不可分 — 第2 slice の対象。

## Data and Control Flow

### 移行期間中の挙動（本設計の中核 — AC3 の「定義」部分）

| 経路 | Phase A（本 slice） | 第2 slice（emit flip 後） |
| --- | --- | --- |
| deserialize `"real-abi-test-plugin"` | `RealAbiTestPlugin` を返す（canonical 綴りとして従来どおり受容） | `alias` 側で引き続き受容（後方互換維持） |
| deserialize `"test-plugin"` | `RealAbiTestPlugin` を返す（`alias` で新規受容） | canonical 綴りとして受容 |
| serialize / `as_str()` emit | `"real-abi-test-plugin"`（不変） | `"test-plugin"`（schema 語彙に統一） |
| `RunMetadata::from_json`/`to_json` round-trip | 旧綴りで emit → 旧綴りで受容（不変）。新綴り文書も受容可能に | 新綴りで emit → 新綴り受容 + 旧綴り文書も alias で受容 |
| historical evidence JSONL（`evidence/rf389*/`・`evidence/rf631/` 等） | 過去 run の SHA 付き記録として**書き換えない**。旧綴りのまま履歴に残り、deserialize 側は引き続き受容するため読み戻しも可能 | 同左（emit flip は過去記録に遡及しない） |

- **canonical wire 綴りの所在**: serde の serialize は
  `rename_all = "kebab-case"` 由来の `real-abi-test-plugin` を
  出力し続ける。`#[serde(alias)]` は deserialize 側の追加受容名
  のみで serialize 出力を変えない（serde の仕様）。
- **`as_str()` は emit の authoritative 入口**:
  `crates/ruviform-plugin-dock-test/src/scenario.rs` が
  `json!(kind.as_str())` で evidence JSONL に流すため、
  `as_str()` の flip が emit flip の実効点になる。serde `rename`
  と `as_str()` は同一 slice で同時に flip する必要がある
  （`environment_kind_serde_matches_as_str` テストが一致を強制）。

### 第2 slice の作業範囲（記録のみ — 本 slice では実施しない）

1. `metadata.rs`: `RealAbiTestPlugin` に
   `#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]`
   を付与し `as_str()` を `"test-plugin"` に flip。
2. `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`:
   literal assert の綴り更新、`Cargo.toml` コメント更新、
   scenario emit 経路の確認（`as_str()` flip で自動追従）。
3. `tests/metadata.rs`: emit 綴り pin の期待値更新
   （`environment_kind_wire_spellings_match_case_schema_v1` に
   `RealAbiTestPlugin` → `test-plugin` を追加する形が自然）。
4. issue-629 docs の status を「解消済み」に更新。

## Non-Functional Constraints

- `unsafe` なし・新規 dependency/feature なし（serde derive は
  既存 `features = ["derive"]` で十分 — `Cargo.toml` 変更不要）。
- 公開 enum の `#[non_exhaustive]`・variant 追加なし（既存 variant
  への attribute 追加のみ — additive・非破壊）。
- RT 契約: `as_str()` は `const` のまま不変。serde 変更は
  non-RT の JSON 経路のみに影響。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| Phase A だけでは schema 語彙と emit が一致しない | `environment_kind=real-abi-test-plugin` の emit が consumer-verify 検証で拒否される状態は継続 | 既知 gap として issue-629 docs・本 design・PR 本文に明記。「解消済み」と誤記しない。emit flip は第2 slice で解消 |
| alias が旧綴りの綴り間違いを隠す | `test-plugin` 以外の未知綴りは引き続き拒否される（`json_rejects_unknown_vocabulary` で pin） | alias は `test-plugin` 1 語のみ。未知語彙拒否テストを維持・拡張 |
| 第2 slice が長期化 | schema 語彙との不一致が残存 | consumer 影響ゼロ・deserialize は両綴り受容のため実害は限定的。rf#631 lease 状況を Coordinator が管理 |
| historical evidence の旧綴りを「誤記」と誤認 | 過去 run 記録の書き換え圧力 | 本 design で「書き換えず履歴として残す」を明文化。deserialize が旧綴りを受容し続けるため読み戻し可能 |

## 却下した代替案

- **一括 flip（rename + as_str + plugin-dock-test 同時更新）**:
  `crates/ruviform-plugin-dock-test/` は rf#631 占有で本 slice の
  編集範囲外。占有クレートの literal assert が flip で直撃失敗
  するため、分割が必須。
- **custom `Deserialize`/`Serialize` impl**: `rename_all` +
  `alias` で要件を満たせる。独自 impl は保守対象を増やすだけで
  利益がない。
- **`as_str()` のみ先行変更**: serde emit（`rename_all` 由来）と
  乖離し `environment_kind_serde_matches_as_str` が壊れる。
  emit 経路の authoritative 入口は `as_str()` だが、serde と
  `as_str()` は同一語彙を共有する設計のため同時 flip が正しい。
- **`#[serde(other)]` 等の catch-all**: 未知語彙を別 variant に
  潰す設計は fail-closed の Case Schema v1 契約（未知値拒否）に
  反する。

## Verification Notes

- [ ] `mise run test ruviform-audio-test` — alias 受容・emit 不変の
  pin テスト pass。
- [ ] `mise run check`/`lint`/`fmt-check` — lint `-D warnings`・
  rustfmt 通過。
- [ ] `mise run test-ci-consumer-verify` — schema 側非回帰
  （`ci/consumer-verify/` は変更なし）。
- [ ] `prek run --all-files` — 軽量チェック通過。
