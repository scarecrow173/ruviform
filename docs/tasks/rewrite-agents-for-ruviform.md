---
id: "TASK-034IwCiByN74V3hysJ9iTR"
type: "task"
status: "done"
title: "AGENTS.md の Ruviform 向け節単位改稿"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/rewrite-agents-for-ruviform.md"
    modified:
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/replace-template-derived-docs-with-ruviform.md"
  implemented-by:
    - "../impl/ir/rewrite-agents-for-ruviform.md"
  depends-on:
    - "../plans/replace-template-derived-docs-with-ruviform.md"
    - "baseline-placeholder-sweep.md"
    - "update-apm-identity-and-lockfile.md"
  blocks:
    - "run-placeholder-sweep-and-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
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

# AGENTS.md の Ruviform 向け節単位改稿

## Work

plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-AGENTS（Phase G-1〜G-8）。`AGENTS.md` を design の再編成表どおり節単位で改稿する。`## 開発・設計原則`（現行 L7–L66・7 サブセクション）は verbatim 保持を原則とし、冒頭・作業契約・mise 節・APM 節・Git 節を現状適合させ、新規節 `## Rust ワークスペースと暫定開発規約`（仮題）を追加する。テンプレート前提の禁止事項・派生リポジトリ表現は除去する。

- [x] G-1. 冒頭（L1–L5）を書き換える: 「このリポジトリは Ruviform — Rust を中心とした再利用可能なマルチメディア基盤（設計・初期セットアップ段階）である」旨と、以下がエージェント共通の作業契約であることを述べる。「言語・フレームワークに依存しない開発基盤のテンプレート」「派生リポジトリ」への言及は消す → L3–L4 を Ruviform の作業契約である旨に書き換え済み（テンプレート・派生表現なし）
- [x] G-2. `## リポジトリの作業契約` を編集する: 作業契約の箇条は保持し、「このテンプレートに言語・フレームワーク固有の構成や、不要な `.github/`・`docs/` ディレクトリを追加しない。派生プロジェクトで必要に応じて定義する。」（L75）を削除し、「実装されていない API・クレート・検証を実在/実行済みと報告しない」を明文化する → 旧 L75 を削除し L74 に「実装されていない API・クレート・検証を、実在するものまたは実行済みと報告しない」を追加。他箇条は verbatim 保持
- [x] G-3. `## 開発環境（mise）` を編集する: 「テンプレートには共通基盤ツールのみを置く…」（L80）を現状に書き換え（`mise.toml` は共通基盤ツール＋`rust = "1.99.0"`・actionlint を管理し、検証タスクも同ファイルが正本）、初回セットアップのコマンド列を IR 検証済み手順に同期（`mise install --locked` が正本・`mise install` はフォールバック・`mise run setup`・厳格検証 `mise exec -- apm install --frozen`）、「派生リポジトリのmiseタスク」（L91）を「`mise.toml` のタスク」に修正し、lockfile 再生成条件（`mise.lock`/`apm.lock.yaml`/`Cargo.lock` の各トリガと復旧手順）を IR の表に基づき追記する → L79・L80–L94・L98・L101–L109 を編集。セットアップ列は IR 検証済み手順に同期（検証環境限定・caveat・フォールバック・厳格検証を併記）、lockfile 再生成条件を表＋復旧手順で追記
- [x] G-4. `## AIエージェント環境（APM）` を微修正する: 運用規約は実態どおり保持しつつ、`apm.yml` の `name` が `ruviform` である現状に記述を合わせる → 先頭箇条を「`apm.yml`（パッケージ名 `ruviform`）を正本とする」に修正。他箇条は verbatim 保持
- [x] G-5. `## Gitと安全性` に LFS フィクスチャ方針を 1 箇条追加する: 小さなテストフィクスチャは非メディア形式（JSON/YAML 等）で Git に置き、CI はフィクスチャが必要になるまで LFS オブジェクトを取得しない（現行 `ci.yml` も LFS 取得なし・ADR 決定の文書化） → L169 に箇条追加（`ci.yml` に `lfs` 記述なしを grep で実測確認）。併せて lockfile 追跡箇条に `Cargo.lock` を追加
- [x] G-6. 新規節 `## Rust ワークスペースと暫定開発規約`（仮題）を追加する: 構成（ルート `Cargo.toml` の `[workspace]`/`workspace.package`/`workspace.lints`/`[profile.dev]`/`[profile.ci]`＋仮置きの最小メンバー `crates/ruviform-core`。最終分割・公開 API は #1）、検証入口（#2 の mise タスク対応表と「cargo コマンドは `mise run` 経由で呼ぶ」規約。`*-ci` は CI 用途限定）、暫定規約（`unsafe` 原則禁止＋SAFETY コメント付き局所許可・default feature 最小・依存追加は理由記録・`workspace.lints` が正本で `-D warnings` ゲート・公開 API の例は doctest）、暫定原則（下位基盤クレートは上位クレートに依存しない・SIMD の前提/不変条件。詳細契約は #1 で確定）、ライセンス注記（`LICENSE` 不在・未決定・`license = "MIT"` は仮置き）、RT/オフラインの設計意図（リアルタイム音声経路ではブロッキング操作・意図しない allocation を避ける方向で設計する暫定原則。API ごとの保証は #1 で確定し未確定 API を RT-safe と断定しない旨を明記） → L111–L152 に 5 サブセクション（ワークスペース構成/検証入口/暫定規約/暫定原則/ライセンス注記）で追加。全要素を記載
- [x] G-7. `## 開発・設計原則`（7 サブセクション）が保持されていることを `git diff` で確認する（verbatim またはほぼ verbatim） → `git diff` の hunk は旧 L1–8・L72–100・L107–110 のみで、原則節（旧 L7–66）は全 hunk 外・コンテキスト行のみ。verbatim 保持を確認
- [x] G-8. 表現ルールの自己監査を行う: 構想/検討中/未確定/未検証/仮置きの各区分がラベル明示され、design の禁止記載に該当する表現がないことを確認する → grep＋目視監査で全ラベル該当箇所に適用・禁止記載ゼロを確認（IR `Validation` に記録）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/replace-template-derived-docs-with-ruviform.md](../plans/replace-template-derived-docs-with-ruviform.md) — `PLAN-034IvX2cFeBjPV4UMfDJ2x`（approved）。Phase G-1〜G-8 / V-2・V-3・V-9 の正本
- [../designs/replace-template-derived-docs-with-ruviform.md](../designs/replace-template-derived-docs-with-ruviform.md) — `DESIGN-034IvBNBZvoikAgd6DJxdD`（approved）。`AGENTS.md` 再編成表・表現ルール・禁止記載の直接入力
- [../designs/dev-foundation.md](../designs/dev-foundation.md) — `DESIGN-034IddXFCRKDIW01WXkSla`。暫定開発規約・mise タスク対応表の転記元
- [../adr/replace-template-derived-docs-metadata-with-ruviform.md](../adr/replace-template-derived-docs-metadata-with-ruviform.md) — `ADR-034Ivm746Q38Wa21HeqUiI`（accepted）。作業契約の Ruviform 固有化・LFS ポリシー文書化の決定
- [../specs/replace-template-derived-docs-with-ruviform.md](../specs/replace-template-derived-docs-with-ruviform.md) — `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（approved）。暫定原則（依存方向・SIMD）の要件根拠
- [../impl/ir/verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md) — #2 の一次記録。「作業メモ（#3 転記用）」の暫定開発規約・lockfile 再生成条件・LFS 方針の転記元

## Dependencies

- 前提タスク: `baseline-placeholder-sweep.md`（`TASK-034IwBWH50nD9Unn8HghJD`）、`update-apm-identity-and-lockfile.md`（`TASK-034IwCbI6X4pBBby5Donzz`）— G-4 が `apm.yml` の `name: ruviform` 確定済みを前提とする
- 後続をブロック: `run-placeholder-sweep-and-verification.md` — 最終スイープ・内容監査の対象を確定する

## Affected Files

- `AGENTS.md` — 節単位改稿（`## 開発・設計原則` は保持、その他は編集・追加・削除）
- `docs/impl/ir/rewrite-agents-for-ruviform.md` — 新規。節ごとの編集内容・`git diff` 確認結果・自己監査結果を記録する IR
- `docs/impl/ir/README.md` — IR index への追記

## Done When

- [x] G-1〜G-8 がすべて完了し、design の再編成表どおりになっている → 全ステップ `[x]` 済み（IR `Changes` に節ごとの記録）
- [x] `## 開発・設計原則` が verbatim（またはほぼ verbatim）で保持されている → `git diff` で 7 サブセクションの変更行なしを確認
- [x] テンプレート前提の禁止事項・「派生リポジトリ/派生プロジェクト」表現が除去され、「未実装 API・検証を実在/実行済みと報告しない」規約が明文化されている → `git grep -in "template\|テンプレート"`・`git grep -n "派生リポジトリ\|派生プロジェクト"` とも exit 1、L74 に明文化済み
- [x] 新規節に mise タスク対応表・暫定規約・暫定原則・ライセンス注記・RT/オフライン設計意図が含まれ、未確定 API を RT-safe と断定する表現がない → L111–L152 に全要素を記載、RT-safe 断定なし（「断定しない」と明記）
- [x] AGENTS 記載の mise タスク名がすべて `mise.toml` に実在する（V-9 の対象） → `setup`/`fmt`/`fmt-check`/`check`/`lint`/`test`/`build`/`check-ci`/`lint-ci`/`test-ci`/`build-ci`/`verify-ci` 全 12 件を `mise.toml` の `[tasks.*]` で確認
- [x] 改稿結果と自己監査結果が IR `docs/impl/ir/rewrite-agents-for-ruviform.md` に記録されている → IR `IMPL-034Ix98fGRcsRNYZ3EZnZJ` に記録

## Verification

- [x] `git grep -in "template\|テンプレート" -- AGENTS.md` exits 1（テンプレート前提文言なし） → exit 1（0 hit）確認済み
- [x] `git grep -n "派生リポジトリ\|派生プロジェクト" -- AGENTS.md` exits 1 → exit 1（0 hit）確認済み
- [x] `grep -n "SAFETY\|workspace.lints\|doctest" AGENTS.md` が暫定規約の記載に一致する → L115/138/141/142 に一致
- [x] `grep -n "mise install --locked\|mise run setup\|apm install --frozen" AGENTS.md` が初回セットアップ・厳格検証の記載に一致する → L84/87/90/93/97 に一致
- [x] `mise exec -- prek run --all-files` exits 0（end-of-file-fixer・mixed-line-ending 等の軽量チェック） → exit 0（全 hook Passed、check illegal windows names のみ Skipped）
- [x] Manual: `git diff AGENTS.md` で `## 開発・設計原則`（7 サブセクション）の保持・新規節の要素・LFS 箇条・ライセンス注記を確認（V-2・V-3・V-9 の目視監査。補助） → hunk 範囲と本文の目視監査で全項目確認（IR `Validation` に記録）
