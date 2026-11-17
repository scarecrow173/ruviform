---
id: "DESIGN-034Z7dLBI0cjzvj59p11n2"
type: "design"
status: "approved"
title: "Issue #725 serde 語彙 Phase B — emit flip 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/specs/issue-725-serde-vocab-phase-a.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
    - "evidence/rf725/consumer-impact-assessment.md"
    - "docs/discovery/issue-725-serde-vocab-phase-b.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-725-serde-vocab-phase-b.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
  implemented-by:
    - "docs/plans/issue-725-serde-vocab-phase-b.md"
  depends-on:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  blocks:
    - "docs/plans/issue-725-serde-vocab-phase-b.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  refines:
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #725 serde 語彙 Phase B — emit flip 設計

## Context

Phase A（PR#736・`49c41399`）で `RealAbiTestPlugin` に
`#[serde(alias = "test-plugin")]` が適用され、deserialize は
新旧両綴りを受容する状態になった。一方 emit 側は serde
`rename_all = "kebab-case"` 由来の `real-abi-test-plugin` と
`as_str()` の同綴りを維持しており、schema 正本
`ci/consumer-verify/evidence.mjs` `ENVIRONMENT_KINDS` の
`test-plugin` と不一致が残存している。本設計は Phase A design
（`docs/designs/issue-725-serde-vocab-phase-a.md`、approved）の
「第2 slice の作業範囲」（:100-111）を実行する emit flip の
設計を記録する。移行方針そのもの（deserialize 両綴り受容・
historical evidence 不変・schema `test-plugin` が最終正本）は
Phase A design :79-98 で確定済みであり、本設計はその実施形態
のみを扱う。

## Scope

### In Scope

- `crates/ruviform-audio-test/src/metadata.rs`:
  serde attr 切替・`as_str()` flip・enum doc/variant doc 更新。
- `crates/ruviform-audio-test/tests/metadata.rs`: emit pin の
  期待値更新（3 テスト）。
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`:
  :168・:211 literal assert 更新。
- `crates/ruviform-plugin-dock-test/Cargo.toml`: :77-79
  コメント更新。
- issue-629 docs 5 ファイルへの「解消済み」status 追記
  （append-only）。
- `evidence/rf725/` Phase B 検証記録・docs chain。

### Out of Scope

- `ci/consumer-verify/**`・`.github/workflows/**`（schema 正本は
  既に `test-plugin`、変更不要・read-only）。
- consumer pin bump・consumer repo 変更（影響評価で不要判定済み）。
- historical evidence JSONL の書き換え（過去 run の SHA 付き
  記録 — 旧綴りは alias で恒久的に受容）。
- `EnvironmentKind` 型名 rename・他 enum 語彙・
  `RealAbiTestPlugin` 以外の variant。
- `scenario.rs`/`real_dock.rs` 等の lease 外ファイル
  （`scenario.rs`:280 は `as_str()` flip に自動追従）。
- stale branch `scarecrow173/rf725-serde-vocab` の削除・再利用。
- opt-in lane の実走・evidence 再採取・依存変更・docs index。

## Components and Boundaries

- **`metadata.rs`（変更）**: `EnvironmentKind` enum が語彙の
  authoritative な所有者。serde canonical 綴りと `as_str()`
  emit をここで `test-plugin` に統一する。型名
  `RealAbiTestPlugin`・variant 集合・`#[non_exhaustive]`・
  他 variant は不変。
- **`tests/metadata.rs`（変更）**: emit pin 層。
  `environment_kind_serde_matches_as_str`（serde emit ==
  `as_str()` の不変条件）は変更せず、flip が atomic に行われた
  ことを検証する側として機能させる。
- **`plugin_dock.rs`（変更）**: 実 ABI 実行を主張する case の
  `environment_kind` literal assert — emit flip と同時更新が
  必須（不一致だと flip 後に fail）。
- **`Cargo.toml`（plugin-dock-test、変更）**: 依存コメントの
  語彙言及のみ。依存エッジ・feature・version は不変。
- **`scenario.rs`（read-only、lease 外）**: `json!(kind.
  as_str())` が emit 経路 — `as_str()` flip に自動追従するため
  編集不要であることを確認済み。
- **`ci/consumer-verify/`（read-only）**: schema 正本。
  `ENVIRONMENT_KINDS` は既に `test-plugin` で変更不要 —
  flip は schema への収束。

## Data and Control Flow

### flip 後の挙動（Phase A design :81-87 の右列の実現）

| 経路 | Phase B（本 slice） |
| --- | --- |
| deserialize `"real-abi-test-plugin"` | `RealAbiTestPlugin` を返す（`alias` 側で受容継続 — 後方互換維持） |
| deserialize `"test-plugin"` | `RealAbiTestPlugin` を返す（canonical 綴りとして受容） |
| serialize / `as_str()` emit | `"test-plugin"`（schema 語彙に統一） |
| `RunMetadata` round-trip | 新綴りで emit → 新綴り受容。旧綴り文書も alias で受容 |
| historical evidence JSONL | 過去 run の SHA 付き記録として**書き換えない**。旧綴りは deserialize alias で恒久的に受容されるため読み戻し可能 |

### atomic flip の根拠

- `environment_kind_serde_matches_as_str` が全 variant で
  serde emit == `as_str()` を強制するため、serde `rename` と
  `as_str()` は同一 slice で同時に flip する（Phase A design
  :93-98 で決定済み）。
- `plugin_dock.rs` の literal assert は emit 結果を直接 pin
  するため、`metadata.rs` の flip と同一 slice で更新しないと
  `mise run test ruviform-plugin-dock-test` が失敗する
  （cdylib 解決環境では assert が実効実行される）。
- serde の `rename` は serialize/deserialize 双方の canonical
  名を変更し、`alias` は deserialize 側の追加受容名 —
  `#[serde(rename = "test-plugin", alias =
  "real-abi-test-plugin")]` で「emit = `test-plugin`、旧綴りも
  受容」が一箇所で表現できる。

## Non-Functional Constraints

- `unsafe` なし・新規 dependency/feature なし（serde derive は
  既存 `features = ["derive"]` で十分 — `Cargo.toml` の依存
  エッジ変更不要）。
- 公開 enum の `#[non_exhaustive]`・variant 追加なし（既存
  variant への attribute 変更のみ — 型名・variant 集合は不変）。
- RT 契約: `as_str()` は `const` のまま。serde 変更は non-RT
  の JSON 経路のみに影響。
- `plugin_dock.rs` の fail-closed degrade 設計（:137-149
  early-return）は維持 — assert 更新は綴りのみで判定構造は
  変更しない。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| serde `rename` と `as_str()` の flip が非 atomic | `environment_kind_serde_matches_as_str` 失敗・emit 経路と serde の綴り乖離 | 同一 slice・同一コミット系列で実施。pin テストが検出する |
| plugin-dock-test literal assert の更新漏れ | cdylib 解決環境で `fixture_run_reports_honest_verdicts` が失敗 | :168・:211・Cargo.toml コメントを同 slice で更新。headless で degrade のみの場合は NotRun として PR 本文に明記 |
| 旧綴り受容の喪失 | historical evidence・既存 JSON 文書が読めなくなる | `alias = "real-abi-test-plugin"` を恒久的に維持。`environment_kind_deserialize_accepts_both_vocab_spellings` で pin |
| 「解消済み」の誤記（flip 前記載） | #629 design Risks 表が警告する誤記パターン | issue-629 docs への status 追記は flip 実装・検証後に実施。追記のみで既存記述は変更しない |
| `Cargo.lock` 差分 | 依存変更なしのため発生しない見込み | 発生した場合のみ再生成して PR 本文に明記（共通契約の例外条項） |

## 却下した代替案

- **serde `rename` のみ先行（`as_str()` 据え置き）/ 逆方向**:
  `environment_kind_serde_matches_as_str` が即失敗し、emit
  経路の authoritative 入口（`as_str()`）と serde emit が
  乖離する。Phase A design で却下済み。
- **custom Serialize/Deserialize impl**: `rename`+`alias` で
  要件を満たせる。独自 impl は保守対象を増やすだけ（Phase A
  design で却下済み）。
- **stale branch `scarecrow173/rf725-serde-vocab` の再利用**:
  main に完全 merge 済み・Spec が新規 branch を指定。再利用・
  削除ともに不可（破壊的操作はユーザー判断）。
- **`#[serde(other)]` catch-all**: 未知語彙を別 variant に潰す
  設計は fail-closed の Case Schema v1 契約（未知値拒否）に
  反する（Phase A design で却下済み）。

## Verification Notes

- [ ] `mise run check|lint|test ruviform-audio-test` — flip 後の
  pin（emit = `test-plugin`・deserialize 両綴り受容・未知語彙
  拒否）pass。
- [ ] `mise run check|lint|test ruviform-plugin-dock-test` —
  literal assert 更新後の非回帰。cdylib 解決可否による degrade
  経路の実効範囲を記録。
- [ ] `mise run test-ci-consumer-verify` — schema 側非回帰
  （`ci/consumer-verify/` は変更なし）。
- [ ] `mise run fmt-check`・`mise exec -- prek run --all-files`
  — 軽量チェック通過。
- [ ] `*-full`/`*-msrv`/3OS 横断は daily CI 委譲（NotRun として
  記録）。
- [ ] opt-in lane（`test-ui-plugin-dock*`）の実効 evidence 再採取
  — CI 必須 gate でなく運用判断（必要なら Coordinator 確認）。

## 実装記録への引き継ぎ

- `docs/plans/issue-725-serde-vocab-phase-b.md` に file map・
  sequence・verification matrix を展開する。
- issue-629 docs 5 ファイルの追記は flip 実装・検証完了後に
  行う（本設計時点では未実施 — 「解消済み」を先走って記載しない）。
- `docs/adr/` は作成しない — emit flip は Phase A design で
  確定済みの移行計画の実行であり新規のアーキテクチャ決定を
  含まない（Phase A と同じ判断。Coordinator 指示で ADR 作成を
  求められた場合は `docs/adr/issue-725-serde-vocab-phase-b.md`
  を最小限で追加する）。
- docs index（`docs/**/README.md`）は一切編集・`index apply`
  実行しない — Coordinator が統合時に再生成。
