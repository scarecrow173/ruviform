---
id: "TASK-034IwCiYXp6ub3O8Brcs3L"
type: "task"
status: "todo"
title: "設定ファイルのレビューと最小修正"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/review-config-files-minimal-edits.md"
    modified:
      - type: "file"
        path: "mise.toml"
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
    - "baseline-placeholder-sweep.md"
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

# 設定ファイルのレビューと最小修正

## Work

plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-CONFIG（Phase C-1〜C-6）。`mise.toml` のテンプレート前提コメントを現状適合させ、`.pre-commit-config.yaml`/`.gitignore`/`.gitattributes`/`Cargo.toml` を design の判定基準でレビューする。既定は無変更であり、Ruviform に不適切な文言が見つかった場合のみ最小修正する。レビュー結果（変更/無変更とその理由）は実施記録に残す。

- [ ] C-1. `mise.toml` L8 コメント `# Infrastructure tools only; add a project language toolchain as needed.` を Rust ツールチェーン追加済みの現状に合わせて書き換える（例: `# Shared infrastructure tools plus this repository's pinned Rust toolchain.`）。コメントは英語のまま維持。`[tools]`・task description・`run`/`env` 定義・`[settings]`/`[env]` は無変更
- [ ] C-2. `mise.toml` の全 task description をレビューし実態適合を確認する（食い違いが見つかった場合のみ description を修正。タスク定義本体は変更しない）
- [ ] C-3. `.pre-commit-config.yaml` をレビューする: L1「Language-agnostic checks」という定位と builtin フック群（trailing-whitespace/end-of-file-fixer/check-yaml/check-toml/mixed-line-ending/fix-byte-order-marker/detect-private-key/check-illegal-windows-names）が Ruviform に適合することを確認。既定は無変更とし、コメント文言が「テンプレート前提」と読める場合のみ微修正
- [ ] C-4. `.gitignore` をレビューする: `target/`・APM/Serena 生成物（`apm_modules/`・`.agents/`・各エージェント配布物）・秘密情報（`.env*`）・ローカル状態の除外が適合済みであることを確認。既定は無変更
- [ ] C-5. `.gitattributes` をレビューする: 音声・動画・大型編集ソースの拡張子 LFS ルールが維持されていることを確認する（属性行は変更しない。フィクスチャ方針は G-5・R-4 で文書化済みであることと整合確認）
- [ ] C-6. `Cargo.toml` が無変更であることを確認し、C-1〜C-5 のレビュー結果（変更/無変更とその理由）を実施記録に残す

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/replace-template-derived-docs-with-ruviform.md](../plans/replace-template-derived-docs-with-ruviform.md) — `PLAN-034IvX2cFeBjPV4UMfDJ2x`（approved）。Phase C-1〜C-6 / V-5・V-6・V-8 の正本
- [../designs/replace-template-derived-docs-with-ruviform.md](../designs/replace-template-derived-docs-with-ruviform.md) — `DESIGN-034IvBNBZvoikAgd6DJxdD`（approved）。`mise.toml` 差分設計・レビュー対象ファイルの判定基準の直接入力
- [../adr/replace-template-derived-docs-metadata-with-ruviform.md](../adr/replace-template-derived-docs-metadata-with-ruviform.md) — `ADR-034Ivm746Q38Wa21HeqUiI`（accepted）。LFS ポリシー（属性行維持）の決定
- [../specs/replace-template-derived-docs-with-ruviform.md](../specs/replace-template-derived-docs-with-ruviform.md) — `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（approved）。設定ファイル最小修正の要件根拠
- [../impl/ir/verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md) — #2 の一次記録。LFS フィクスチャ方針の確認結果

## Dependencies

- 前提タスク: `baseline-placeholder-sweep.md`（`TASK-034IwBWH50nD9Unn8HghJD`）— ベースライン記録済みであること
- 後続をブロック: `run-placeholder-sweep-and-verification.md` — 最終スイープ・挙動不変スモークの対象を確定する
- 並行可能: `rewrite-readme-for-ruviform.md`、`update-apm-identity-and-lockfile.md` とは S-1 後に並行着手可

## Affected Files

- `mise.toml` — L8 コメントのみ書き換え。task description は実態との食い違いがある場合のみ修正
- `.pre-commit-config.yaml` / `.gitignore` — レビューのみ。既定無変更（「テンプレート前提」と読める文言がある場合のみ微修正。修正した場合は本ブロックを追記更新する）
- `.gitattributes` / `Cargo.toml` / `mise.lock` / `.mise/locks/` — レビューのみ。変更しない
- `docs/impl/ir/review-config-files-minimal-edits.md` — 新規。C-1〜C-5 のレビュー結果（変更/無変更と理由）を記録する IR
- `docs/impl/ir/README.md` — IR index への追記

## Done When

- [ ] C-1〜C-6 がすべて完了し、`mise.toml` の diff がコメント（必要時 description）のみである
- [ ] `.gitattributes`・`Cargo.toml`・`mise.lock`/`.mise/locks/` が無変更である
- [ ] `.pre-commit-config.yaml`/`.gitignore` の変更/無変更の判定と理由が IR に記録されている
- [ ] C-1〜C-5 の全レビュー結果（変更/無変更と理由）が IR に記録されている

## Verification

- [ ] `grep -n "Infrastructure tools only" mise.toml` exits 1（テンプレート前提コメントなし）
- [ ] `mise exec -- prek run --all-files` exits 0（check-toml が `mise.toml`/`Cargo.toml`、check-yaml が `.pre-commit-config.yaml` の構文を検証）
- [ ] `git diff --exit-code -- .gitattributes` exits 0（属性行無変更）
- [ ] `git diff --exit-code -- Cargo.toml` exits 0（無変更）
- [ ] `git diff --exit-code -- mise.lock .mise/locks/` exits 0（コメントのみの変更で lockfile 復元結果に影響しないこと）
- [ ] `git diff -- mise.toml` がコメント・description 以外の変更を示さない（`[tools]`/`run`/`env`/`[settings]`/`[env]` 無変更）
- [ ] Manual: IR に C-1〜C-5 の変更/無変更判定と理由が記録されていること（補助）
