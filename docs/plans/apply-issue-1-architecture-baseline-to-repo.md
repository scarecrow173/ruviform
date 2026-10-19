---
id: "PLAN-034IR3qOr15LGNY9mEindJ"
type: "plan"
status: "approved"
title: "Issue #1 設計ベースラインの開発基盤・README／AGENTS への反映"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
test-spec-skip: "外部から観測可能な変化は cargo metadata の publish メタデータに限定され、V-1 で機械検証するため test-spec は作成しない"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  changes:
    added: []
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/ruviform-core/Cargo.toml"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/overview.md"
      - type: "file"
        path: "docs/specs/architecture-design-baseline-deliverables.md"
      - type: "file"
        path: "docs/designs/ruviform-architecture-design-baseline.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/plans/README.md"
  implements:
    - "../specs/architecture-design-baseline-deliverables.md"
  implemented-by: []
  depends-on:
    - "dev-foundation-bootstrap.md"
    - "replace-template-derived-docs-with-ruviform.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../Cargo.toml"
    - "../../crates/ruviform-core/Cargo.toml"
    - "../../README.md"
    - "../../AGENTS.md"
    - "../../mise.toml"
    - "../../.github/workflows/ci.yml"
    - "../designs/overview.md"
    - "../designs/dev-foundation.md"
    - "../adr/licensing-and-distribution-policy.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/ui-architecture-webview-hybrid.md"
    - "../adr/plugin-architecture-clap-vst3.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/architecture-design-baseline-deliverables.md"
    - "../designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/2"
    - "https://github.com/scarecrow173/ruviform/issues/3"
  defers: []
  deferred-by: []
---

# Issue #1 設計ベースラインの開発基盤・README／AGENTS への反映 Implementation Plan

## Goal

Issue #1 の設計成果物として approved 済みの design `DESIGN-034IPynWe2V30FLubzQenb` が「#2 開発基盤への反映と README／AGENTS 差分タスク（A.8）」節（L524–L555）でタスク化した範囲を、リポジトリの現行ファイルへ適用する。対象は `Cargo.toml`（workspace）への `publish = false` 方針適用と edition/MSRV 評価結論の反映、`README.md` の未確定節更新・導線・RT 契約注意書き、`AGENTS.md` の暫定原則→確定契約置換であり、あわせて `docs/designs/overview.md` の stale status 注記を整合し、spec・design の `derived-by` に本計画へのバックリンクを追記する。spec `SPEC-034INYy2aDLxse9dy8MgGZ` の Acceptance Criteria 7 件のうちリポジトリ反映に関わる充足状態を検証マトリクスで確認する。

## Scope

単一の実装ストリームとして扱う（分割しない）。全スライスが「design A.8 のタスク化項目を既存ファイルへ反映する」という同一のクリティカルパス上にあり、変更は 5 つの実装対象ファイル（`Cargo.toml`×2・`README.md`・`AGENTS.md`・`docs/designs/overview.md`）に加え、spec・design の `derived-by` バックリンク追記という relation bookkeeping 2 ファイルに閉じる。design の差分タスクを 4 つの実装スライス＋検証スライスに写像する。

| スライス | 内容 | 主な触れるファイル |
| --- | --- | --- |
| T-PUB | `[workspace.package]` に `publish = false` 追加＋`crates/ruviform-core` で `publish.workspace = true` 継承＋edition/rust-version コメントの評価結論反映（値は変更しない） | `Cargo.toml`、`crates/ruviform-core/Cargo.toml` |
| T-README | `README.md` の「未確定」節再構成・「検討中」節ラベル整合・冒頭案内と `docs/designs/` 導線更新・RT 契約注意書き追加 | `README.md` |
| T-AGENTS | `AGENTS.md` の暫定原則節を確定契約（RT 契約・SIMD 構造仮説・依存方向 DAG 引用）へ差替＋ワークスペース構成節の整合 | `AGENTS.md` |
| T-IDX | `docs/designs/overview.md` の baseline 参照に残る stale 注記の更新と「#3 の境界」注記への #1 A.8 追記（索引整合。design 明示タスク外・plan 追加） | `docs/designs/overview.md` |
| T-VERIFY | publish 機械確認・prek・mise 検証・文書監査・spec AC 突合・diff スコープ確認 | 全変更ファイル（検査） |

境界（Out of Scope）は design A.8・spec と同一。design が「本設計の diff には含めない」と明示した `.github/workflows/ci.yml`（RT 検証 job・OS matrix・ベンチ workflow）と `mise.toml`（criterion ベンチ・`test-rt` 等の RT 検査タスク）は変更しない。`edition`/`rust-version`/`lints`/`profiles` の値変更は専用の確認タスク（design が挙げる後続「edition/MSRV/publish 反映 Issue」）へ defer されており値は変更しない。`members` への新規クレート追加と `crates/ruviform-core` の削除は Phase 1 基盤 Issue のタスク。design「後続実装 Issue の提案一覧」（19 件）の GitHub Issue 実作成は本計画では行わない（一覧の記録は design に存在。作成するかはマージ時のユーザー判断）。`LICENSE` 作成・`license` 確定・Phase 1–3 のコード実装全般も含めない。

## Upstream Documents

- [../specs/architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md) — `SPEC-034INYy2aDLxse9dy8MgGZ`（approved）。Requirements A.8 と Acceptance Criteria 7 件が本計画の検証マトリクスの正本
- 一次ソース: GitHub Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（`relations.source`）
- 前提: 計画 [dev-foundation-bootstrap.md](./dev-foundation-bootstrap.md)（`PLAN-034IdrispOUms1c32192B6`、approved）と [replace-template-derived-docs-with-ruviform.md](./replace-template-derived-docs-with-ruviform.md)（`PLAN-034IvX2cFeBjPV4UMfDJ2x`、approved）に `depends-on`。本計画が編集する `Cargo.toml`・`README.md`・`AGENTS.md` の現行状態はこの 2 計画の task 群（全て done）が生成した状態である

## Design Inputs

- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。以下の節を実装の直接入力とする:
  - 「#2 開発基盤への反映と README／AGENTS 差分タスク（A.8）」（L524–L555）— 差分タスクの正本
  - 「後続実装 Issue の提案一覧」（L557–L586）— 19 件の実装 Issue 提案と 6 件の決定フォーラム（記録のみ・作成しない）
  - 「feature flag・公開方針」（L203–L210）— publish 制御（未決定クレート原則 `publish = false`）の根拠
  - 「レイヤー構造と依存方向ルール」（L111–L173）— AGENTS の依存方向契約の引用元
  - 「RT／非 RT の分離方式」（L272–L281）・「RT 契約（API 単位の保証定義）」（L420–L425）・「RT 違反検出の分担」（L432–L437）— AGENTS の RT 契約・README の注意書きの引用元
  - 「DSP／SIMD 設計」採否（L355–L369）— AGENTS の SIMD 原則の引用元
  - 「暫定実装ロードマップ（A.6）」（L495–L522）— `members` 段階追加と `ruviform-core` 処遇（Phase 1 で削除）の根拠
- [../adr/licensing-and-distribution-policy.md](../adr/licensing-and-distribution-policy.md) — `ADR-034IO02s3n3KHsxnHckpRn`（accepted）。未決定クレートは `publish = false` 原則・ライセンス未決定の根拠
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — `ADR-034IO00JjhxeTXUbCZtcDX`（accepted）。RT 契約・CI 成功は完全証明でない旨の根拠
- [../adr/platform-and-toolchain-targets.md](../adr/platform-and-toolchain-targets.md) — `ADR-034IO01ZSFQcu2jEt2p0Tb`（accepted）。対象プラットフォーム（Windows/macOS/Linux・x86_64＋aarch64）決定の根拠
- [../designs/dev-foundation.md](../designs/dev-foundation.md) — #2 基盤の仮置き決定と根拠（MSRV・pin 版・CI 構成の現行値の一次ソース）
- 現行対象ファイル: `Cargo.toml`（`[workspace.package]` に `publish` 未設定・edition/rust-version コメントが `#1 で再評価` を示す）、`crates/ruviform-core/Cargo.toml`（`publish` 未継承）、`README.md`（L20–L31 の構想/検討中/未確定節・L92–L96 Documentation 節）、`AGENTS.md`（L110–L151 のワークスペース構成・暫定規約・暫定原則・ライセンス注記）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `Cargo.toml` | workspace 正本（`[workspace.package]`・lints・profiles） | T-PUB。`[workspace.package]` に `publish = false` 追加（licensing ADR）。`edition`/`rust-version`/`license`/lints/profiles の値は無変更。`#1 で再評価` コメントを評価結論（専用タスクへ defer）に更新 |
| `crates/ruviform-core/Cargo.toml` | 最小メンバークレート（smoke test） | T-PUB。`publish.workspace = true` を追加して継承。クレート削除は Phase 1 基盤 Issue のタスクであり本計画では残置 |
| `README.md` | リポジトリの顔 | T-README。section-edit。「未確定（#1 で決定）」節の再構成、「検討中（採否は #1 で決定）」節のラベル整合、冒頭案内・Documentation 導線、RT 契約注意書き |
| `AGENTS.md` | エージェント共通の作業契約 | T-AGENTS。section-edit。「暫定原則（詳細契約は #1 で確定）」→確定契約節へ差替、「ワークスペース構成」の publish/分割記述整合。feature/unsafe 規約・ライセンス注記は現行維持 |
| `docs/designs/overview.md` | 設計成果物の全体マップ | T-IDX。annotation-edit。baseline 参照の stale 注記（L35・L40・L94）を approved に更新し、L42「README／AGENTS 本文の改稿は #3 の境界」に #1 A.8 タスク実施を追記。front matter `status: "draft"` は本計画では変更しない（Open Questions / Gaps 参照） |
| `docs/specs/architecture-design-baseline-deliverables.md` | 上流 spec（approved） | T-VERIFY。metadata-edit。`relations.derived-by` に本 plan へのバックリンクを追加し双方向リンクを一致させる（`derived-by` 追記に伴い `updated:` は適用日に更新）。本文・`status: "approved"` は無変更 |
| `docs/designs/ruviform-architecture-design-baseline.md` | 上流 design（approved） | T-VERIFY。metadata-edit。`relations.derived-by`（現行 `[]`）に本 plan へのバックリンクを追加し双方向リンクを一致させる（`derived-by` 追記に伴い `updated:` は適用日に更新）。本文・`status: "approved"` は無変更 |
| `mise.toml` | ツール pin・検証タスクの正本 | no-change。design は後続 Issue での criterion/`test-rt` タスク追加を提案するのみ。現行タスク表は正本として維持 |
| `.github/workflows/ci.yml` | `workflow_dispatch` 専用・Linux 単一 job CI | no-change。design は現行維持を明示。RT 検証 job・OS matrix・ベンチは後続 Issue |
| `docs/plans/README.md` | plan 索引（generated） | generated。`new_plan.js` により本 plan 行が追加済み |
| `LICENSE` | — | 非作成（design Out of Scope・spec Deferred）。`test ! -f LICENSE` を V-8 で確認 |

未確定要素: README/AGENTS の最終文案は design が逐語確定していないため、実装タスクで design の引用元節（Design Inputs 記載）に従って執筆する（Open Questions / Gaps 参照）。

## Implementation Sequence

実装は S-1（ベースライン記録）→ T-PUB ∥ T-README ∥ T-IDX（並行可）→ T-AGENTS（T-PUB 後）→ T-VERIFY の順がクリティカルパス。各ステップは 1 アクション。

### Phase S: ベースライン記録（共通前提）

- [ ] S-1. 変更前状態を記録する: `git grep -n "暫定原則\|#1 で確定\|#1 で決定\|未確定\|検討中" -- README.md AGENTS.md` のヒット一覧、`mise exec -- cargo metadata --format-version 1 --no-deps` における `ruviform-core` の `publish` 値（現行: `null` ＝未設定）、`docs/designs/overview.md` の `(draft)` 注記箇所を記録し、T-VERIFY の最終比較用ベースラインとする

### Phase T-PUB: `publish = false` 方針の適用

- [ ] P-1. ルート `Cargo.toml` の `[workspace.package]` に `publish = false` を追加し、未決定クレートは公開しない licensing ADR 原則であり、メンバーは `publish.workspace = true` で継承する旨のコメントを付ける。`edition`/`rust-version` の `# 仮置き。#1 で edition / MSRV を再評価`・`# 仮置き MSRV。` コメントは、評価結論「専用の確認タスク（design が挙げる edition/MSRV/publish 反映 Issue）へ送付・現段階で変更しない」を指す文言に更新する。`license = "MIT"` とその仮置きコメントは無変更（design: 仮置きのまま維持・未決定の明記を確認）
- [ ] P-2. `crates/ruviform-core/Cargo.toml` に `publish.workspace = true` を追加する（`license.workspace = true` 等の既存継承行と同じ方式。値行のみ・他フィールド無変更）。あわせて L2 の `# 仮置き名。#1 で最終クレート分割を再評価` コメントを、評価結論「Phase 1 で `ruviform-*` クレート群への分割置換として削除予定（design A.8「`crates/ruviform-core` の処遇」）」を指す文言に更新する
- [ ] P-3. `mise exec -- cargo metadata --format-version 1 --no-deps` を実行し、`ruviform-core` の `publish` が `[]`（publish 不可）であることを確認する（V-1）

### Phase T-README: `README.md` 差分更新

- [ ] R-1. 「### 未確定（#1 で決定）」節（L29–L31）を「### 決定済み・未確定」に改題して再構成する: 対応プラットフォーム（Windows/macOS/Linux・x86_64＋aarch64）は platform ADR・design で決定済みとして「決定済み」側に列挙し、公開 API・サポート形式・ライセンス・最終的なクレート分割（design に DAG 設計案として記録・実装 Issue で確定）・edition/MSRV（専用確認タスクへ defer・`Cargo.toml` 現行値は仮置き）は「未確定」側に列挙して区別して記述する
- [ ] R-2. 「### 検討中（採否は #1 で決定）」節（L25–L27）のラベルと内容を design の採否に整合させる: WebView UI（UI ADR で WebView ファースト・ハイブリッドが方針決定・具体技術は要 PoC）、`wgpu`（エンジン層 opt-in adapter・採否は要 PoC）、ネイティブ UI（要 PoC）、プラグイン連携（plugin ADR・design でアーキテクチャ決定・CLAP/VST3 同格）の区分を反映する
- [ ] R-3. 冒頭段落の「詳細設計は #1 の後続タスクで確定します」を設計ベースライン approved の現状に更新し、「## Documentation」節に `docs/designs/ruviform-architecture-design-baseline.md` への導線（Issue #1 由来の設計ベースラインである旨）を追加する（design タスク: 「`docs/designs/` へのエントリポイントとして本書を追記」）。あわせて「## Documentation」節の Issue #1 行（L96）「プロダクト本体の詳細設計（後続タスク）」を「詳細設計ベースライン approved・実装は後続 Issue」等の現状記述へ更新する
- [ ] R-4. RT 契約・保証範囲の注意書きを追加する: RT 契約は API 単位で定義・明示、保証は Ruviform 自身の RT 対応実装に限定、CI 成功は完全証明ではない、OS レベルのハードリアルタイム保証は提供しない旨（design「RT 契約（API 単位の保証定義）」「RT 違反検出の分担」の明記要求）
- [ ] R-5. ラベル監査を行う: 「実在・検証済み」「構想」「検討中」「決定済み」「未確定」各区分のラベルが保持され、未確定項目を確定済み・未実装を実装済みと記述していないことを確認する

### Phase T-AGENTS: `AGENTS.md` 暫定原則 → 確定契約

- [ ] G-1. 「### 暫定原則（詳細契約は #1 で確定）」（L143–L147）を確定契約節へ差し替える: RT/オフラインの設計意図を design の確定契約（API・ライフサイクルで分離・`Processor`/`RtContext`・`prepare`/「process」分離・RT パスは alloc/dealloc 禁止・保証範囲は API 単位で明示・未確定 API を RT-safe と断定しない・CI 成功は完全証明ではない・OS ハード RT 保証なし）へ置き換え、詳細は design「RT／非 RT の分離方式」「RT 契約」節を正本として参照する旨を記す
- [ ] G-2. 同節の SIMD 暫定原則を design の構造仮説へ更新する: スカラー参照実装必須・等価性契約（ビット一致または ulp 上限）・stable-only・`unsafe` 局所許可は現行規約と整合・SIMD 経路の採否と dispatch 層の `ruviform-math` 集約は要 PoC である旨（design「DSP／SIMD 設計」採否）
- [ ] G-3. 同節のクレート依存方向を design の DAG ルールへの引用へ更新する: 上位→下位のみ・循環依存禁止・基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない、詳細な層内順序・adapter ルール・契約型所有は design「レイヤー構造と依存方向ルール」節が正本である旨を記す
- [ ] G-4. 「### ワークスペース構成」（L114–L115）を整合させる: `[workspace.package]` の列挙に `publish = false`（未決定クレート原則・licensing ADR）を加え、「最終的なクレート分割と公開 API は Issue #1 の詳細設計で確定する」を「クレート DAG・分割案は `docs/designs/ruviform-architecture-design-baseline.md` に記録され、`members` は実装 Phase に応じて段階追加される」現状に更新する（`members` 段階追加方針の根拠: design A.8「クレート追加計画」L533）
- [ ] G-5. 「### 暫定規約」の feature/unsafe 規約が現行のまま design（DSP/SIMD の unsafe 方針・feature 外部バックエンド分離クレート方針）と整合することを確認し、結果（変更/無変更と理由）を実施記録に残す（design: 「feature/unsafe 規約は現行維持で整合を確認」）
- [ ] G-6. 「### ライセンス注記」（L149–L151）が無変更であることを確認する（design: 維持・仮置き・未決定）
- [ ] G-7. `#1 で確定`・`#1 で決定`・`後続タスク` 等の陳腐化した参照が README/AGENTS に残っていないことを `git grep -n "#1 で確定\|#1 で決定\|後続タスク" -- README.md AGENTS.md` で確認する（S-1 ベースラインと比較）

### Phase T-IDX: `docs/designs/overview.md` 注記整合

- [ ] X-1. `docs/designs/overview.md` の `ruviform-architecture-design-baseline.md` 参照に付く stale 注記を approved へ更新する: L35 `（draft）` → `（approved）`、L40 `、draft` → `、approved`、L94 `(draft)` → `(approved)` の 3 形式。あわせて L42「README／AGENTS 本文の改稿は #3 の境界」に「#1 設計ベースライン approved に伴う設計反映の差分更新は #1 A.8 タスクとして実施」旨を追記する（designs README 索引は approved を表示済み・body 注記のみ追従。front matter `status: "draft"` は変更しない）

### Phase T-VERIFY: 総合検証

- [ ] F-1. `mise exec -- cargo metadata --format-version 1 --no-deps` で `ruviform-core` の `publish` が `[]` であることを再確認する（V-1）
- [ ] F-2. `mise exec -- prek run --all-files` を実行する（V-2。auto-fix 系フックの修正が出たら取り込んでから再実行）
- [ ] F-3. `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test` を実行する（V-3 の挙動不変スモーク。本変更はビルド入力に影響しないが manifest parse・lint ゲートを確認する）
- [ ] F-4. README/AGENTS の内容監査を行う（V-4・V-5。design A.8 タスク・採否表との突合）
- [ ] F-5. spec `docs/specs/architecture-design-baseline-deliverables.md` と design `docs/designs/ruviform-architecture-design-baseline.md` の front matter `relations.derived-by` に本 plan（`../plans/apply-issue-1-architecture-baseline-to-repo.md`）へのバックリンクを追加し、本 plan の `derives-from`（spec・design）と双方向リンクを一致させる（relation 更新は規約上許容される in-place edit。`derived-by` 追記に伴い両文書の `updated:` は適用日に更新する。本文・status は無変更）
- [ ] F-6. `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` を実行し docs グラフの不整合がないことを確認する（V-6。`node` は環境前提のため導入済みであることを先に確認する）
- [ ] F-7. spec `SPEC-034INYy2aDLxse9dy8MgGZ` の Acceptance Criteria 7 件を突合し、本計画の変更による充足/非退行を記録する（V-7）
- [ ] F-8. `git diff --name-only` と `git diff Cargo.toml crates/ruviform-core/Cargo.toml` で diff スコープを確認する（V-8: 宣言ファイルのみ・edition/rust-version/license 値行不変・mise.toml/ci.yml/Cargo.lock 無変更・`LICENSE` 非作成）し、実行した検証と未検証条件を区別して実施記録に報告する

## Dependencies and Risks

### Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| S-1（ベースライン記録） | — | T-PUB, T-README, T-AGENTS, T-IDX, T-VERIFY |
| T-PUB | S-1 | T-AGENTS, T-VERIFY |
| T-README | S-1 | T-VERIFY |
| T-AGENTS | S-1, T-PUB | T-VERIFY |
| T-IDX | S-1 | T-VERIFY |
| T-VERIFY | T-PUB, T-README, T-AGENTS, T-IDX | — |

- クリティカルパス: S-1 → T-PUB → T-AGENTS → T-VERIFY。AGENTS の「ワークスペース構成」記述（`publish = false` の存在）が T-PUB の適用結果を前提とするため T-AGENTS を T-PUB に先行させない
- 並行可能な作業: T-PUB ∥ T-README ∥ T-IDX。T-README と T-AGENTS は互いに独立だが、両方の完了が T-VERIFY の前提

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | 文書更新が design 決定と不整合（最終クレート分割を確定済みと記述・未確定 API を RT-safe 断定・CI 成功を証明扱い等） | med | high | 執筆を Design Inputs の引用元節（依存方向 L111–L173・RT 分離 L272–L281・RT 契約 L420–L425・SIMD 採否 L355–L369・採否/分類表）に限定。R-5/G-7 の自己監査＋V-4/V-5 の監査＋V-7 の AC 突合を実施。表現の最終担保は実装 PR のレビュー観点 |
| 2 | `publish.workspace = true` の記述漏れで `publish = false` が member に効かない（`[workspace.package]` の値は `*.workspace = true` 宣言でのみ継承される） | med | med | P-3/F-1 で `cargo metadata` の `publish == []` を機械確認（V-1）。未継承なら `null` のまま残り即検出できる |
| 3 | `edition`/`rust-version`/`lints`/`profiles` の値を誤って変更（design は現段階で変更しない） | low | med | F-8 の `git diff` で値行の不変を確認（V-8）。コメント更新のみを許容し値行は編集対象外 |
| 4 | spec `SPEC-034INYy2aDLxse9dy8MgGZ` の status 未承認（解決済み） | — | — | spec は approved へ更新済み（status 更新はコーディネータが実施・本計画の変更対象外）。本計画は approved design を直接の実装入力とし、spec AC は V-7 で検証突合する |
| 5 | README の「検討中」節再構成で design が分類表に載せていない項目を誤って確定/棄却扱いする | med | low | 各項目を design の採否・ADR 決定（ui ADR の WebView 方針・platform ADR の wgpu opt-in・plugin ADR の CLAP/VST3 同格）に対応付け、対応付け不能な項目は未確定側に残す。V-4 で design 分類表と突合 |
| 6 | 編集ファイルが prek フック（trailing-whitespace/end-of-file-fixer/mixed-line-ending 等）に抵触 | med | low | F-2 で `prek run --all-files` を実行。auto-fix 系フックの修正は取り込んでから再実行する |
| 7 | `docs/` 配下の宣言外文書（spec/ADR/design/task/test-spec 等の記録済み文書）を誤って書き換える | low | med | 書き換え対象を `relations.changes.modified` の宣言 7 ファイルに限定（spec/design は front matter `derived-by` のみの metadata-edit）し、F-8 の `git diff --name-only` で宣言外の逸脱を検出する |
| 8 | データ損失: README/AGENTS 節編集で現行の有効な記述（作業契約・mise 運用・検証タスク表等）を消失 | low | med | 編集対象を宣言節（暫定原則・ワークスペース構成・未確定/検討中節・冒頭・Documentation）に限定。ドキュメントのみの変更で `git revert`/`checkout` 可能。S-1 のベースライン記録で変更前状態を保全 |
| 9 | `cargo metadata`/`cargo` 系実行で `Cargo.lock` に意図しない差分が混入 | low | low | `publish` は lockfile に影響しない想定。F-8 で `git diff Cargo.lock` を確認し、差分が出れば `git checkout -- Cargo.lock` で復旧 |
| 10 | 外部依存の可用性: cargo registry 等のネットワーク障害で cargo 系コマンドが失敗 | low | low | `--no-deps` の metadata・ローカル検証タスクのみで外部 fetch を必要としない。失敗時は再試行 |

## Task Breakdown

task-doc 化の単位はスライスと一致させ、1 task = 1 実装まとまり・複数 commit 可とする。推奨分割:

| 候補 task | 含むステップ | 完了条件（検証エントリ） |
| --- | --- | --- |
| T-PUB: workspace への `publish = false` 適用（task-doc 案 `docs/tasks/apply-publish-false-policy-to-workspace.md`） | P-1〜P-3 | V-1 |
| T-README: `README.md` 差分更新（task-doc 案 `docs/tasks/update-readme-for-issue-1-baseline.md`） | R-1〜R-5 | V-4, V-5 |
| T-AGENTS: `AGENTS.md` 暫定原則の確定契約化（task-doc 案 `docs/tasks/update-agents-interim-principles-to-fixed-contracts.md`） | G-1〜G-7 | V-4, V-5 |
| T-IDX: `docs/designs/overview.md` 注記整合（task-doc 案 `docs/tasks/align-designs-overview-status-annotations.md`） | X-1 | V-6 |
| T-VERIFY: 総合検証・spec/design メタデータ編集・実施記録（task-doc 案 `docs/tasks/verify-issue-1-repo-reflection.md`） | S-1 のベースライン共有＋F-1〜F-8 | V-1〜V-8 |

前提ステップ: S-1（ベースライン記録）はいずれのタスクにも属さない共有の pre-dispatch ステップであり、スライス dispatch 前に一度だけ実行する。T-VERIFY 行の「S-1 のベースライン共有」は、そのベースラインを消費して最終比較を行う意味であり、S-1 自体の実行は T-VERIFY の担当範囲外である。

task-doc の作成は plan 承認後の planning-flow 後段で行う。本計画で外部から観測可能な変化は `cargo metadata` の publish メタデータに限定され V-1 で機械検証するため、test-spec は作成しない（front matter `test-spec-skip` 参照）。

## Verification

| Step | Verification command or criteria | Pass condition |
| --- | --- | --- |
| V-1（A.8: publish 方針適用） | `mise exec -- cargo metadata --format-version 1 --no-deps` の出力で `ruviform-core` の `publish` フィールドが `[]` であること、ルート `Cargo.toml` に `publish = false`・member に `publish.workspace = true` が存在することを確認 | `publish == []`・両ファイルに宣言あり・`edition`/`rust-version`/`license` 値行は不変 |
| V-2（構文有効・prek 実行可） | `mise exec -- prek run --all-files` | exit 0（check-toml が `Cargo.toml` 系・check-yaml 等の構文検証を通過。front matter の構文検証は audit_docs（V-6/F-6）が担う） |
| V-3（挙動不変スモーク） | `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test` | 全コマンド exit 0（manifest parse・lint ゲート・smoke test が変更前と同じく成功） |
| V-4（AC5・A.8: README/AGENTS が design 決定と整合） | README が「対応プラットフォーム決定済み」「公開 API・サポート形式・ライセンス・最終クレート分割・edition/MSRV 未確定」の区別、design baseline への導線、RT 契約注意書き（API 単位保証・CI 成功≠完全証明・OS ハード RT 保証なし）を持つこと、AGENTS が確定契約節・DAG 引用・`publish = false` 記載を持つことを目視監査 | 全該当箇所に適用済み・design の採否/分類表と矛盾なし |
| V-5（AC4/AC5: Requirements B 整合・禁止記載なし） | README/AGENTS が spec B の確定制約（stable-only・RT 契約・CLAP/VST3 同格・WebView ファースト・publish=false・ライセンス未決定・外部型非露出）に反しないこと、禁止記載（未確定項目の確定扱い・未作成クレートや API の実在・RT-safe 断定・CI 成功=完全証明・`LICENSE` 存在/ライセンス決定済み）がないことを目視監査 | B 制約と整合・禁止記載なし・feature/unsafe 規約・ライセンス注記が現行維持 |
| V-6（索引・監査整合） | `docs/designs/overview.md` の baseline 参照が `(approved)` であること、`node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` が新規の不整合を報告しないこと | 注記更新済み・audit_docs が不整合なし（`node` は環境前提） |
| V-7（spec AC 充足突合） | spec Acceptance Criteria 7 件を本計画の変更と突合: AC1・AC2・AC7 は design doc（approved）の存在と本計画の非退行で確認、AC3 は `publish = false` 反映の適用（V-1）＋後続 Issue 一覧が design に記録済みであること、AC4 は V-5、AC5 は V-4、AC6 は新規クレート非作成・`ruviform-core` 残置を `Cargo.toml`/`members` の diff で確認 | 7 件すべてについて充足または非退行が記録できる |
| V-8（diff スコープ・互換性） | `git diff --name-only` の結果が宣言 7 ファイル＋`docs/plans/README.md` に限定、`git diff Cargo.toml crates/ruviform-core/Cargo.toml` で `edition`/`rust-version`/`license`/`lints`/`profiles`/`members` の値行が不変、spec/design の diff が `derived-by` 行と `updated:` 行のみ（`derived-by` 追記に伴い `updated:` を適用日に更新）、`git diff mise.toml .github/workflows/ci.yml Cargo.lock` が空、`test ! -f LICENSE` | 宣言外変更なし・値行不変・no-change ファイル無変更・`LICENSE` 非作成。本 plan 自身の `status`/`updated` フリップと `docs/plans/README.md` 索引再生成は承認 bookkeeping として別コミットで処理済み前提とし、実装 diff のスコープ外 |

## Rollback Strategy

本計画の変更はすべて Git 追跡ドキュメント・manifest のテキスト編集であり、DB マイグレーションや公開 API 変更のような不可逆操作は含まない（`publish = false` は公開を防ぐ方向の制約であり破壊的操作ではない）。各対象のロールバック:

| 対象 | Trigger | Procedure | Verification |
| --- | --- | --- | --- |
| `Cargo.toml`/`crates/ruviform-core/Cargo.toml` | publish 確認失敗・値行の誤変更 | `git checkout -- Cargo.toml crates/ruviform-core/Cargo.toml` | `cargo metadata` の `publish` が `null`（変更前状態）に戻り V-1/V-8 が変更前状態を示す |
| `README.md`/`AGENTS.md` | design との不整合・禁止記載混入がレビューで発覚 | `git checkout -- README.md AGENTS.md` | S-1 ベースラインの grep ヒットが元に戻り V-4/V-5 が変更前状態を示す |
| `docs/designs/overview.md` | 注記編集の誤り | `git checkout -- docs/designs/overview.md` | `(draft)` 注記が元に戻る |
| 全体 | 任意の失敗 | `git revert <commit>`（全変更が Git 追跡ファイルの編集のみ） | 変更前 HEAD で V-1〜V-8 を再実行 |

## Open Questions / Gaps

- **spec の status（解決済み）**: `SPEC-034INYy2aDLxse9dy8MgGZ` は approved へ更新済み（status 更新はコーディネータが計画作成後に実施・本計画の変更対象外）。本計画は approved な design を直接の実装入力とし spec AC を検証対象として扱う
- **後続 Issue 19 件の GitHub Issue 実作成**: 一覧の記録は design L557–L577 に存在。作成するかはマージ時のユーザー判断とし、本計画のスライス・検証には含めない
- **`docs/designs/overview.md` 自身の front matter `status: "draft"`**: map 文書の lifecycle 判断は本計画の範囲外とし、body 内の stale `(draft)` 注記のみ整合する（T-IDX）
- **README/AGENTS の最終文案**: design は差分タスクと引用元節を定めるが逐語確定していないため、実装タスクで Design Inputs の範囲に従って執筆する。表現の担保は機械検証しにくく、最終確認は実装 PR のレビュー観点
- **「検討中」節・冒頭案内・AGENTS「ワークスペース構成」の整合拡張**: design の明示タスクは「未確定」節・暫定原則項等を名指しするが、同一ファイル内の「（採否は #1 で決定）」「#1 の詳細設計で確定する」等の陳腐化した参照を放置すると文書内で矛盾するため、design の採否表・DAG 決定を一次ソースとする整合更新を R-2/R-3/G-4 として本計画で追加した
- **edition/MSRV コメントの送り先**: design は「専用の確認タスク」と「edition/MSRV/publish 反映 Issue」を挙げるが GitHub Issue 番号は未存在のため、コメントは Issue 名での言及に留める

## Self-Review

- [x] spec Requirements A.8 と Acceptance Criteria 7 件が T-PUB/T-README/T-AGENTS/T-IDX/T-VERIFY のステップおよび V-1〜V-8 に対応している（AC 突合は V-7 に集約）
- [x] design A.8 の差分タスク（`publish = false` 適用・license 仮置き維持・edition/MSRV 現段階不変更・README/AGENTS 更新・ci.yml/mise.toml 無変更・ruviform-core 残置）が実装順序と検証に反映されている
- [x] `TBD`/`TODO`/`implement later` 等の placeholder は残っていない（未確定事項は Open Questions / Gaps に明示）
- [x] file path・relation・status・コマンド・用語が design/spec/ADR と整合している（`publish = false`・`publish.workspace = true`・`cargo metadata`・`workflow_dispatch`・`assert_no_alloc` 型 lane 後続・`(approved)` 等）

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（task-doc 化後、review checkpoint 付きで task ごとに実行）
- Independent work streams: T-PUB ∥ T-README ∥ T-IDX は S-1 後に並行可能。T-AGENTS は T-PUB 後、T-VERIFY は全タスク完了後の直列スイープ。変更面積が小さいため単一 implementer が依存順に実行してもよい
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 仮置きと判断が残る点: README/AGENTS の最終文案（design は差分タスク・引用元節・採否表を確定）、「検討中」節再構成の各項目対応付けは実装タスクに委ねている
- plan 追加の整合タスク: R-2（「検討中」節）・R-3（冒頭案内）・G-4（ワークスペース構成節）・T-IDX（overview.md 注記）は design の明示タスク文言を超えた整合拡張であり、design A.8 の差分目的（「#1 で決定/確定」の陳腐化参照の解消）との対応付けをレビューしてほしい
- spec `SPEC-034INYy2aDLxse9dy8MgGZ` は approved へ更新済み（status 更新は本計画の変更対象外）。approved design を上流ゲートとして本計画は成立する。design が提案する 19 件の後続実装 Issue の GitHub Issue 作成可否はマージ時のユーザー判断
- `relations.derives-from` は spec・design をリンク済み。本計画が適用する決定の根拠 ADR（licensing・realtime・platform・ui・plugin）は `relations.related` に記録した
- draft 段階では `audit_docs.js` が derives-from ↔ derived-by の片方向 warning を報告し得る（spec/design の `derived-by` に本 plan バックリンク未追記のため）。F-5 の metadata edit で解消する先例どおりの bookkeeping であり、実装完了後の V-6 で不整合なしを確認する
- レビュー重点: `publish.workspace = true` 継承方式の正しさ（workspace.package の値は `*.workspace = true` 宣言でのみ継承）、README の「未確定」節が design の採否分類（対応プラットフォーム=決定・公開 API/サポート形式/ライセンス/最終クレート分割/edition/MSRV=未確定）を正しく区別しているか、AGENTS の確定契約節が design の RT/SIMD/DAG 記述を超えた主張をしていないか、宣言 7 ファイル以外の diff がないか
- 上流参照: spec `SPEC-034INYy2aDLxse9dy8MgGZ`（Requirements A.8/Acceptance Criteria）、design `DESIGN-034IPynWe2V30FLubzQenb`（A.8 差分タスク・後続 Issue 一覧・採否表・Verification Notes）、ADR `ADR-034IO02s3n3KHsxnHckpRn`/`ADR-034IO00JjhxeTXUbCZtcDX`/`ADR-034IO01ZSFQcu2jEt2p0Tb`、Issue #1
