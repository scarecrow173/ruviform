---
id: "TASK-034IwBWH50nD9Unn8HghJD"
type: "task"
status: "todo"
title: "テンプレート残存プレースホルダーのベースラインスイープ"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/baseline-placeholder-sweep.md"
    modified:
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
  implemented-by: []
  depends-on:
    - "../plans/replace-template-derived-docs-with-ruviform.md"
  blocks:
    - "rewrite-readme-for-ruviform.md"
    - "update-apm-identity-and-lockfile.md"
    - "rewrite-agents-for-ruviform.md"
    - "review-config-files-minimal-edits.md"
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

# テンプレート残存プレースホルダーのベースラインスイープ

## Work

plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` の前提ステップ S-1（ベースラインスイープ）。plan ではいずれのスライスにも属さない共有の pre-dispatch ステップとして定義されているが、タスク DAG の依存先は実在する task doc を指す必要があるため本タスクとして切り出した。`git ls-files` を母集団にプレースホルダー所在を確定して記録し、T-VERIFY（`run-placeholder-sweep-and-verification.md`）の最終スイープ（F-2 / V-1）と比較できるベースライン一覧を残す。本タスクは計測・記録のみであり、対象ファイルを一切書き換えない。

- [ ] `git ls-files` で追跡ファイルの母集団を確定する
- [ ] `git grep -in "project.template"` を実行し、一致箇所（パス・行番号・行内容）を記録する
- [ ] `git grep -in "Use this template"` を実行し、一致箇所を記録する
- [ ] `git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存\|言語・フレームワークに依存しない"` を実行し、一致箇所を記録する
- [ ] catch-all として `git grep -in "template\|テンプレート" -- . ':(exclude)docs/**'` を実行し、変種の残存を記録する
- [ ] `docs/` 配下の履歴記録と `apm.lock.yaml` 等の生成物内 APM パッケージ由来パス（`assets/templates/` 等）は書き換え対象外であることを明示し、ベースライン一覧を本タスクの IR `docs/impl/ir/baseline-placeholder-sweep.md`（impl-doc 規約で新規作成）に記録する。plan の現状把握では `README.md`・`AGENTS.md`・`apm.yml`・`mise.toml` L8 コメントが主な残存箇所

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/replace-template-derived-docs-with-ruviform.md](../plans/replace-template-derived-docs-with-ruviform.md) — `PLAN-034IvX2cFeBjPV4UMfDJ2x`（approved）。S-1 と V-1 の正本
- [../designs/replace-template-derived-docs-with-ruviform.md](../designs/replace-template-derived-docs-with-ruviform.md) — `DESIGN-034IvBNBZvoikAgd6DJxdD`（approved）。除外対象（`docs/` 履歴・APM 生成物パス）の判定基準
- [../specs/replace-template-derived-docs-with-ruviform.md](../specs/replace-template-derived-docs-with-ruviform.md) — `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（approved）。Acceptance Criteria「テンプレート残存なし」の根拠
- [../adr/replace-template-derived-docs-metadata-with-ruviform.md](../adr/replace-template-derived-docs-metadata-with-ruviform.md) — `ADR-034Ivm746Q38Wa21HeqUiI`（accepted）。書き換え対象の決定記録
- [../impl/ir/verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md) — #2 の一次記録（plan `references`）。実機検証済み事実の出典

## Dependencies

- 前提タスク: なし（スライス dispatch 前に一度だけ実行する共有 pre-dispatch ステップ）
- 後続をブロック: `rewrite-readme-for-ruviform.md`、`update-apm-identity-and-lockfile.md`、`rewrite-agents-for-ruviform.md`、`review-config-files-minimal-edits.md` — ベースライン確定前に編集が入ると差分比較の基準を失うため
- ベースラインの消費先: `run-placeholder-sweep-and-verification.md` — F-2 で最終スイープと本 IR のベースラインを比較する（plan の依存グラフでは S-1 → T-VERIFY も blocking 関係。本 DAG では 4 スライス経由の推移的依存として扱い、ベースラインは本タスクの IR `docs/impl/ir/baseline-placeholder-sweep.md` 経由で引き渡す）

## Affected Files

- `docs/impl/ir/baseline-placeholder-sweep.md` — 新規。ベースライン一覧（grep 種別・パス・行番号・一致文字列・除外判定）を記録する IR
- `docs/impl/ir/README.md` — IR index への追記
- その他の追跡ファイル — 計測のみ。変更しない

## Done When

- [ ] 4 系統の grep（`project.template`・`Use this template`・日本語派生表現・catch-all `template|テンプレート`）が実行され、一致箇所が一覧化されている
- [ ] 除外対象（`docs/` 履歴・生成物内 APM パッケージ由来パス）が明示され、ベースラインが IR に記録されている
- [ ] 後続スライスが比較可能な粒度（パス＋行番号＋一致パターン）で記録されている
- [ ] 本タスクの実行で記録先の `docs/` 配下以外の追跡ファイルに差分がない

## Verification

- [ ] `git grep -in "project.template"` を実行し、出力が IR のベースライン記録と一致する
- [ ] `git grep -in "Use this template"` を実行し、同上
- [ ] `git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存\|言語・フレームワークに依存しない"` を実行し、同上
- [ ] `git grep -in "template\|テンプレート" -- . ':(exclude)docs/**'` を実行し、同上
- [ ] `test -f docs/impl/ir/baseline-placeholder-sweep.md` exits 0
- [ ] Manual: `git status --porcelain` で `docs/` 配下の記録物以外に意図しない変更がないこと（補助）
