---
id: "TASK-034IwCjptfvbSFBQaSKigx"
type: "task"
status: "todo"
title: "プレースホルダー最終スイープ・総合検証と ADR メタデータ編集"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/run-placeholder-sweep-and-verification.md"
    modified:
      - type: "file"
        path: "docs/plans/replace-template-derived-docs-with-ruviform.md"
      - type: "file"
        path: "docs/adr/replace-template-derived-docs-metadata-with-ruviform.md"
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
    - "rewrite-readme-for-ruviform.md"
    - "update-apm-identity-and-lockfile.md"
    - "rewrite-agents-for-ruviform.md"
    - "review-config-files-minimal-edits.md"
  blocks: []
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

# プレースホルダー最終スイープ・総合検証と ADR メタデータ編集

## Work

plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-VERIFY（S-1 のベースライン共有＋Phase F-1〜F-8）。全スライス完了後にプレースホルダー最終スイープと plan の検証一覧 V-1〜V-11 を実行し、ADR/plan の双方向メタデータ編集を行う。S-1 のベースラインは `baseline-placeholder-sweep.md`（`TASK-034IwBWH50nD9Unn8HghJD`）の IR `docs/impl/ir/baseline-placeholder-sweep.md` から引き継ぎ、最終スイープと比較する（S-1 自体の実行は本タスクの担当範囲外）。実行した検証と未検証条件を区別して本タスクの IR `docs/impl/ir/run-placeholder-sweep-and-verification.md` に記録する。

- [ ] F-1. ADR `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md` の front matter `relations.derived-by` に本 plan（`../plans/replace-template-derived-docs-with-ruviform.md`）へのバックリンクを追加し、design へのバックリンクが存在することを確認する。あわせて本 plan の `relations.derives-from` に当該 ADR を追加して双方向リンクを一致させ、design の先例に倣い重複を避けるため `relations.related` からは当該 ADR を除去する（plan の relation 更新は規約上許容される in-place edit。ADR の本文・`status: "accepted"` は無変更）
- [ ] F-2. プレースホルダー最終スイープを実行し S-1 のベースラインと比較する（V-1）
- [ ] F-3. `mise exec -- prek run --all-files` を実行する（V-5。auto-fix 系フックの修正が出たら取り込んでから再実行）
- [ ] F-4. `mise install --locked` と `mise exec -- apm install --frozen` を実行する（V-4・V-6 の厳格確認。global-config caveat は Risk 8 の判定基準に従う）
- [ ] F-5. `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run verify-ci` を実行する（V-6 の挙動不変スモーク）
- [ ] F-6. README/AGENTS の内容監査を行う（V-2・V-3・V-7・V-8・V-9）
- [ ] F-7. `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` を実行し docs グラフの不整合がないことを確認する（V-10。`node` は repo 管理の mise ツールではなく global mise config 等に由来する環境前提のため、導入済みであることを先に確認する）
- [ ] F-8. 実行した検証と未検証条件（macOS/Linux・mise 未導入の素の Windows・GitHub Actions 実ディスパッチ）を区別して実施記録に報告する（V-11）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/replace-template-derived-docs-with-ruviform.md](../plans/replace-template-derived-docs-with-ruviform.md) — `PLAN-034IvX2cFeBjPV4UMfDJ2x`（approved）。Phase F-1〜F-8 / V-1〜V-11 の正本。F-1 で本ファイルの `derives-from`/`related` を編集する
- [../designs/replace-template-derived-docs-with-ruviform.md](../designs/replace-template-derived-docs-with-ruviform.md) — `DESIGN-034IvBNBZvoikAgd6DJxdD`（approved）。表現ルール・禁止記載・ADR メタデータ編集方針の直接入力
- [../adr/replace-template-derived-docs-metadata-with-ruviform.md](../adr/replace-template-derived-docs-metadata-with-ruviform.md) — `ADR-034Ivm746Q38Wa21HeqUiI`（accepted）。F-1 の編集対象（`relations.derived-by` への plan バックリンク追加）
- [../specs/replace-template-derived-docs-with-ruviform.md](../specs/replace-template-derived-docs-with-ruviform.md) — `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（approved）。Acceptance Criteria の根拠
- [../impl/ir/verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md) — #2 の一次記録。実機検証済み手順・global-config caveat の根拠

## Dependencies

- 前提タスク: `rewrite-readme-for-ruviform.md`（`TASK-034IwCbgksOGvhF5E4CQbp`）、`update-apm-identity-and-lockfile.md`（`TASK-034IwCbI6X4pBBby5Donzz`）、`rewrite-agents-for-ruviform.md`（`TASK-034IwCiByN74V3hysJ9iTR`）、`review-config-files-minimal-edits.md`（`TASK-034IwCiYXp6ub3O8Brcs3L`）— 全スライスの成果物が検証対象
- ベースラインの出典: `baseline-placeholder-sweep.md` の IR `docs/impl/ir/baseline-placeholder-sweep.md`（S-1 のベースラインを消費して最終スイープと比較する）
- 後続をブロック: なし（クリティカルパスの終端）

## Affected Files

- `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md` — F-1 で `relations.derived-by` に plan へのバックリンクを追加（本文・`status` は無変更）
- `docs/plans/replace-template-derived-docs-with-ruviform.md` — F-1 で `relations.derives-from` に ADR を追加し `relations.related` から除去（in-place relation 更新のみ）
- `docs/impl/ir/run-placeholder-sweep-and-verification.md` — 新規。実行した検証と未検証条件を区別して記録する IR
- `docs/impl/ir/README.md` — IR index への追記
- その他の既存ファイル — 検証のみ。変更しない（prek auto-fix 取り込みを除く）

## Done When

- [ ] F-1〜F-8 がすべて完了し、V-1〜V-11 の各判定基準を満たす（または未実行・caveat が IR に記録されている）
- [ ] 最終スイープが S-1 ベースラインとの差分として「テンプレート残存なし」（除外対象を除く）を示す
- [ ] ADR `derived-by` に plan のバックリンクが追加され、plan `derives-from` に ADR・`related` から ADR 除去の双方向整合が取れている
- [ ] IR `docs/impl/ir/run-placeholder-sweep-and-verification.md` が実行済み検証と未検証条件（macOS/Linux・mise 未導入の素の Windows・CI 実ディスパッチ）を区別して記録している

## Verification

- [ ] `git grep -in "project.template"`・`git grep -in "Use this template"`・`git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存\|言語・フレームワークに依存しない"` を追跡ファイルに実行し、catch-all `git grep -in "template\|テンプレート" -- . ':(exclude)docs/**'` の残存が除外対象（`docs/` 履歴・APM 生成物パス）のみである（V-1。S-1 IR のベースラインと比較）
- [ ] `git diff -- apm.yml` が `name`/`description` の 2 行のみの変更を示す（V-4）
- [ ] `mise exec -- prek run --all-files` exits 0（V-5）
- [ ] `mise install --locked` exits 0（V-4・V-6。global mise config 由来の repo lockfile 外ツールがある環境ではその分のみ exit 1 となり得る caveat は Risk 8 判定基準に従い IR に記録）
- [ ] `mise exec -- apm install --frozen` exits 0（V-4）
- [ ] `mise run fmt-check && mise run check && mise run lint && mise run test && mise run verify-ci` exits 0（V-6 挙動不変スモーク）
- [ ] `test ! -f LICENSE` exits 0 かつ `git diff --exit-code -- Cargo.toml` exits 0（V-7）
- [ ] `git diff --exit-code -- .gitattributes` exits 0 かつ `git diff --exit-code -- mise.lock .mise/locks/` exits 0（V-8・V-6）
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` が新規の不整合を報告しない（V-10。`node` 導入確認結果を IR に記録）
- [ ] `grep -n "derived-by" docs/adr/replace-template-derived-docs-metadata-with-ruviform.md` の節に plan へのバックリンクがあり、`grep -n "replace-template-derived-docs-metadata-with-ruviform" docs/plans/replace-template-derived-docs-with-ruviform.md` が `derives-from` に一致し `related` に残らない（V-11）
- [ ] Manual: README/AGENTS の内容監査（V-2・V-3・V-7・V-8・V-9）と、記載 `mise run <name>` の `mise.toml`/`mise tasks` 突合、実施記録の実行済み/未検証区分（補助）
