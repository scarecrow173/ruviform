---
id: "IMPL-034IwOgrHkNf8v0cnrOpIl"
type: "impl"
status: "completed"
title: "Baseline placeholder sweep"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/baseline-placeholder-sweep.md"
    modified:
      - type: "file"
        path: "docs/tasks/baseline-placeholder-sweep.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/baseline-placeholder-sweep.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/replace-template-derived-docs-with-ruviform.md"
    - "../../designs/replace-template-derived-docs-with-ruviform.md"
    - "../../specs/replace-template-derived-docs-with-ruviform.md"
    - "../../adr/replace-template-derived-docs-metadata-with-ruviform.md"
    - "../../tasks/run-placeholder-sweep-and-verification.md"
    - "../../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Baseline placeholder sweep

## Summary

task `TASK-034IwBWH50nD9Unn8HghJD`(plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` の共有 pre-dispatch ステップ S-1)として、`git ls-files` の 55 ファイルを母集団に、タスク指定の 4 系統 + 補助 2 系統の `git grep` を実行し、テンプレート由来プレースホルダーのベースラインを確定・記録した。計測・記録のみであり、書き換え対象の追跡ファイルは一切変更していない。

ベースライン結果: 書き換え対象のリテラル残存は **`README.md` 3 行・`AGENTS.md` 5 行・`apm.yml` 2 行の計 3 ファイル 11 行**。加えて `mise.toml` L8 コメント(リテラル非該当の意味的残存)と `.pre-commit-config.yaml` L1(`language.agnostic` 該当だが無変更既定のレビュー対象)を記録した。`apm.lock.yaml` の 198 hit はすべて生成物内 APM パッケージ由来パスで除外対象、`docs/` 配下の hit は履歴記録として除外対象であることを明示した。

## Context

- 実施タスク: `docs/tasks/baseline-placeholder-sweep.md`(`TASK-034IwBWH50nD9Unn8HghJD`)
- 上流: `docs/plans/replace-template-derived-docs-with-ruviform.md` S-1(実施手順の正本)・V-1(最終スイープとの比較先)、`docs/designs/replace-template-derived-docs-with-ruviform.md` ファイルマップと除外判定基準、`docs/specs/replace-template-derived-docs-with-ruviform.md` AC「テンプレート残存なし」、`docs/adr/replace-template-derived-docs-metadata-with-ruviform.md` 書き換え対象の決定記録
- 消費先: `docs/tasks/run-placeholder-sweep-and-verification.md`(T-VERIFY)が F-2 の最終スイープ結果と本ベースラインを比較する
- 除外ルール(design/plan の判定基準): `docs/` 配下の履歴記録(spec/ADR/design/plan/task/IR/test-spec/index)と `apm.lock.yaml` 等の生成物内 APM パッケージ由来パス(`assets/templates/`・`references/template-variants.*`・`scripts/frame-template.html` 等)は書き換え対象外

## Implementation

### Changes

- `docs/impl/ir/baseline-placeholder-sweep.md` — 新規(本 IR)。ベースライン一覧を記録
- `docs/tasks/baseline-placeholder-sweep.md` — status `todo` → `done`、checkbox 完了、`implemented-by` に本 IR を追記、`updated` 更新
- `docs/impl/ir/README.md` — index テーブルへ本 IR の行を追記
- `docs/tasks/README.md` — index の本タスク行 status を `done` に更新
- 上記以外の追跡ファイル(`README.md`・`AGENTS.md`・`apm.yml`・`mise.toml`・`.pre-commit-config.yaml` 等)は計測のみで無変更

### Decisions During Implementation

- **`git grep -in "project.template"` は両リテラルを包括する**。パターン中の `.` は正規表現の任意 1 文字であり、`project-template`(ハイフン)と `Project Template`(空白・大文字)の双方に一致することを実測で確認した。
- **catch-all は 2 回実行した**。書き換え対象集合の確定にはタスク指定どおり `docs/` 除外版(`-- . ':(exclude)docs/**'`)を用い、ベースライン記録の完全性のため `docs/` 込み版も別途実行して `docs/` hit を独立集計した。除外は書き換え対象集合の定義であり、記録対象からの除外ではない。
- **`apm.lock.yaml` の 198 hit は全件除外判定**。内訳は `.agents/skills/`・`.claude/skills/` 配下の APM パッケージ由来パスのみで、`assets/templates/` 配下パスが 180 件、`adr-doc/references/template-variants.{ja.,}md` が 12 件、`brainstorming/scripts/frame-template.html` が 6 件。出現形態は `  - <path>` ファイルリスト・`<path>: sha256:<hash>` ハッシュ・`value: <path>` マッピングの 3 種。`apm install`/`apm lock` 由来の生成物であり、T-IDENT が `apm.yml` 更新後に lockfile 再生成する範囲でもあるため書き換え対象外とした。
- **`mise.toml` L8 はリテラル非該当の意味的残存として記録**。`# Infrastructure tools only; add a project language toolchain as needed.` は全 grep パターンに一致しないが、plan/design が指名するテンプレート前提コメントであり T-CONFIG C-1 の comment-edit 対象である。
- **`.pre-commit-config.yaml` L1 はレビュー対象(無変更既定)として分類**。`# Language-agnostic checks powered by prek's built-in hooks.` は `language.agnostic` に一致するが、plan C-3/design 判定基準では「定位は設計どおり・無変更を既定」とされるため、リテラル hit だが書き換え対象ではない。
- **README.md のテンプレート前提記述はリテラル hit を超える範囲に及ぶ**。「新規プロジェクトでの使い方」節(L6–L25)や L39 の汎用説明はパターン非該当だが、design が README L1–L25 の全面改稿を指定済みであり T-README の改稿範囲内に包含される。

## Baseline Sweep

### Environment

- OS/シェル: Windows 11 x64 / Git Bash
- git: `git version 2.54.0.windows.1`
- 実施日: 2026-10-19、スイープ時点の HEAD: `3ceb6e6`(worktree clean)
- grep は `git grep`(追跡ファイルの working tree 内容を検索)で実行。本 IR・更新後の task/index は sweep 時点では未追跡または未変更であり、記録値は `3ceb6e6` 時点の追跡ファイル状態を表す

### Population

`git ls-files` → 55 ファイル(母集団。実行結果 verbatim):

```text
.gitattributes
.github/workflows/ci.yml
.gitignore
.mise/locks/pipx-apm-cli/0.33.0/pyproject.toml
.mise/locks/pipx-apm-cli/0.33.0/uv.lock
.pre-commit-config.yaml
AGENTS.md
Cargo.lock
Cargo.toml
README.md
apm.lock.yaml
apm.yml
crates/ruviform-core/Cargo.toml
crates/ruviform-core/src/lib.rs
docs/adr/README.md
docs/adr/replace-template-derived-docs-metadata-with-ruviform.md
docs/adr/rust-dev-foundation-toolchain-and-ci-profile.md
docs/designs/README.md
docs/designs/dev-foundation.md
docs/designs/overview.md
docs/designs/replace-template-derived-docs-with-ruviform.md
docs/discovery/README.md
docs/ideas/README.md
docs/impl/exp/README.md
docs/impl/ir/README.md
docs/impl/ir/add-manual-ci-workflow.md
docs/impl/ir/create-minimal-workspace-and-smoke-test.md
docs/impl/ir/define-mise-dev-and-ci-tasks.md
docs/impl/ir/pin-rust-toolchain-and-lockfile.md
docs/impl/ir/verify-reproducible-setup-and-work-memo.md
docs/plans/README.md
docs/plans/dev-foundation-bootstrap.md
docs/plans/replace-template-derived-docs-with-ruviform.md
docs/specs/README.md
docs/specs/establish-rust-dev-foundation.md
docs/specs/replace-template-derived-docs-with-ruviform.md
docs/tasks/README.md
docs/tasks/add-manual-ci-workflow.md
docs/tasks/baseline-placeholder-sweep.md
docs/tasks/create-minimal-workspace-and-smoke-test.md
docs/tasks/define-mise-dev-and-ci-tasks.md
docs/tasks/pin-rust-toolchain-and-lockfile.md
docs/tasks/review-config-files-minimal-edits.md
docs/tasks/rewrite-agents-for-ruviform.md
docs/tasks/rewrite-readme-for-ruviform.md
docs/tasks/run-placeholder-sweep-and-verification.md
docs/tasks/update-apm-identity-and-lockfile.md
docs/tasks/verify-reproducible-setup-and-work-memo.md
docs/test-specs/README.md
docs/test-specs/ci-profile-build-behavior.md
docs/test-specs/lockfile-reproducible-setup.md
docs/test-specs/manual-ci-workflow.md
docs/test-specs/mise-verification-tasks.md
mise.lock
mise.toml
```

### Results Overview

| # | コマンド(verbatim) | 総 hit | 非 `docs/` hit | `docs/` hit |
| --- | --- | --- | --- | --- |
| G1 | `git grep -in "project.template"` | 32 | 2 | 30 |
| G2 | `git grep -in "Use this template"` | 24 | 1 | 23 |
| G3 | `git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存\|言語・フレームワークに依存しない"` | 32 | 6 | 26 |
| G4 | `git grep -in "template\|テンプレート" -- . ':(exclude)docs/**'` | 204 | 204 | (除外指定) |
| G5(補助) | `git grep -in "template\|テンプレート"` | 383 | 204 | 179 |
| G6(補助) | `git grep -in "language.agnostic"` | 9 | 2 | 7 |

G4/G5 の非 `docs/` 内訳: `AGENTS.md` 3・`README.md` 2・`apm.yml` 1・`apm.lock.yaml` 198。

### Rewrite Targets(書き換え対象: 非 `docs/`・非生成物)

全 grep のユニオン。file:line:match を記録する。

| file:line | 一致行内容 | 該当 grep | 分類 |
| --- | --- | --- | --- |
| `README.md:1` | `# Project Template` | G1, G4 | 書き換え対象(T-README 全面改稿) |
| `README.md:3` | `言語・フレームワーク非依存の、mise + APM + prek + Git LFS による最小開発基盤です。` | G3 | 書き換え対象(T-README) |
| `README.md:8` | `1. GitHub の **Use this template** から新規リポジトリを作成する。` | G2, G4 | 書き換え対象(T-README) |
| `AGENTS.md:3` | `このリポジトリは、言語・フレームワークに依存しない開発基盤のテンプレートです。` | G3, G4 | 書き換え対象(T-AGENTS G-1) |
| `AGENTS.md:5` | `派生リポジトリでは、プロジェクト固有の目的・制約・検証手順を必要に応じて追記してください。` | G3 | 書き換え対象(T-AGENTS G-1) |
| `AGENTS.md:75` | `- このテンプレートに言語・フレームワーク固有の構成や、不要な \`.github/\`・\`docs/\` ディレクトリを追加しない。派生プロジェクトで必要に応じて定義する。` | G3, G4 | 書き換え対象(T-AGENTS G-2 で削除) |
| `AGENTS.md:80` | `- テンプレートには共通基盤ツールのみを置く。言語・フレームワーク別ツールや \`lint\` / \`test\` / \`check\` / \`build\` タスクは派生リポジトリで必要に応じて追加する。` | G3, G4 | 書き換え対象(T-AGENTS G-3) |
| `AGENTS.md:91` | `- Gitフックでは言語非依存の軽量チェックのみ実行し、言語別の Lint・テスト・ビルドは派生リポジトリのmiseタスクで管理する。` | G3 | 書き換え対象(T-AGENTS G-3) |
| `apm.yml:1` | `name: project-template` | G1, G4 | 書き換え対象(T-IDENT I-1) |
| `apm.yml:3` | `description: Minimal language-agnostic development foundation` | G6 | 書き換え対象(T-IDENT I-1) |
| `.pre-commit-config.yaml:1` | `# Language-agnostic checks powered by prek's built-in hooks.` | G6 | レビュー対象(T-CONFIG C-3。design 判定基準どおり無変更既定) |
| `mise.toml:8` | `# Infrastructure tools only; add a project language toolchain as needed.` | なし(意味的残存) | 書き換え対象(T-CONFIG C-1 comment-edit。plan/design が指名) |

合計: リテラル書き換え対象 11 行(`README.md` 3・`AGENTS.md` 5・`apm.yml` 2)+ `mise.toml` L8 comment-edit 1 箇所 + `.pre-commit-config.yaml` L1 レビュー 1 箇所。

### Excluded: Generated Artifact(`apm.lock.yaml`)

G4/G5 での 198 hit はすべて APM パッケージ由来パスであり、生成物として書き換え対象外(design/plan の判定基準どおり)。パターン別内訳:

- `assets/templates/` 配下パス — 180 hit(スキル配布物のドキュメントテンプレート)
- `.agents/skills/adr-doc/references/template-variants.{ja.,}md`・`.claude/skills/adr-doc/references/template-variants.{ja.,}md` — 12 hit(L44, L45, L233, L234, L422, L423, L593, L594, L1724, L1733, L4298, L4307)
- `.agents/skills/brainstorming/scripts/frame-template.html`・`.claude/skills/brainstorming/scripts/frame-template.html` — 6 hit(L1180, L1189, L1198, L1206, L1931, L4505)

`assets/templates/` 180 hit の行番号(全件):

```text
28, 29, 30, 31, 32, 33, 34, 35, 58, 59, 66, 67, 74, 75, 123, 124,
131, 132, 133, 134, 147, 148, 154, 155, 193, 194, 201, 202, 209, 210,
217, 218, 219, 220, 221, 222, 223, 224, 247, 248, 255, 256, 263, 264,
312, 313, 320, 321, 322, 323, 336, 337, 343, 344, 382, 383, 390, 391,
398, 399, 406, 407, 408, 409, 410, 411, 412, 413, 435, 436, 441, 442,
448, 449, 492, 493, 499, 500, 501, 502, 514, 515, 520, 521, 556, 557,
563, 564, 570, 571, 577, 578, 579, 580, 581, 582, 583, 584, 606, 607,
612, 613, 619, 620, 663, 664, 670, 671, 672, 673, 685, 686, 691, 692,
727, 728, 734, 735, 741, 742, 1580, 1589, 1598, 1607, 1616, 1625,
1634, 1643, 2021, 2030, 2255, 2264, 2327, 2336, 2858, 2867, 2984,
2993, 3002, 3011, 3128, 3137, 3227, 3236, 3722, 3731, 3974, 3983,
4064, 4073, 4154, 4163, 4172, 4181, 4190, 4199, 4208, 4217, 4595,
4604, 4829, 4838, 4901, 4910, 5432, 5441, 5558, 5567, 5576, 5585,
5702, 5711, 5801, 5810, 6296, 6305, 6548, 6557, 6638, 6647
```

### Excluded: `docs/` Historical Records

`docs/` 配下の hit はすべて履歴記録(承認済み spec/ADR/design/plan/task/IR/test-spec とその index)であり、書き換え対象から全件除外。以下は grep 別の `docs/` hit 一覧(file:line)。

**G1 `project.template`(`docs/` 30 hit):**

- `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`: 70, 96, 118, 177, 224
- `docs/designs/dev-foundation.md`: 83
- `docs/designs/replace-template-derived-docs-with-ruviform.md`: 82, 84, 162, 187, 253
- `docs/impl/ir/verify-reproducible-setup-and-work-memo.md`: 281
- `docs/plans/replace-template-derived-docs-with-ruviform.md`: 137, 150, 237, 256
- `docs/specs/establish-rust-dev-foundation.md`: 102, 220
- `docs/specs/replace-template-derived-docs-with-ruviform.md`: 80, 85, 141, 157, 171
- `docs/tasks/baseline-placeholder-sweep.md`: 57, 89, 96
- `docs/tasks/rewrite-readme-for-ruviform.md`: 97
- `docs/tasks/run-placeholder-sweep-and-verification.md`: 104
- `docs/tasks/update-apm-identity-and-lockfile.md`: 57, 59

**G2 `Use this template`(`docs/` 23 hit):**

- `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`: 70, 116, 150, 225
- `docs/designs/replace-template-derived-docs-with-ruviform.md`: 82, 131, 221, 253, 254
- `docs/plans/replace-template-derived-docs-with-ruviform.md`: 116, 137, 146, 237, 238
- `docs/specs/replace-template-derived-docs-with-ruviform.md`: 85, 124
- `docs/tasks/baseline-placeholder-sweep.md`: 58, 89, 97
- `docs/tasks/rewrite-readme-for-ruviform.md`: 54, 61, 96
- `docs/tasks/run-placeholder-sweep-and-verification.md`: 104

**G3 日本語派生表現(`docs/` 26 hit):**

- `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`: 70, 116, 150, 155
- `docs/designs/replace-template-derived-docs-with-ruviform.md`: 83, 149, 152, 188, 253
- `docs/plans/replace-template-derived-docs-with-ruviform.md`: 137, 157, 158, 159, 237
- `docs/specs/replace-template-derived-docs-with-ruviform.md`: 85, 130, 171
- `docs/tasks/baseline-placeholder-sweep.md`: 59, 98
- `docs/tasks/rewrite-agents-for-ruviform.md`: 55, 57, 58, 59, 94, 102
- `docs/tasks/run-placeholder-sweep-and-verification.md`: 104

**G5 catch-all `docs/` 分(179 hit):**

- `docs/adr/README.md`: 9
- `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`: 5, 47, 50, 66, 70, 77, 84, 85, 96, 116, 118, 124, 138, 142, 147, 150, 155, 177, 224, 225, 235, 236
- `docs/designs/README.md`: 11
- `docs/designs/dev-foundation.md`: 83
- `docs/designs/overview.md`: 23, 67
- `docs/designs/replace-template-derived-docs-with-ruviform.md`: 5, 30, 38, 59, 60, 63, 74, 78, 82, 83, 84, 85, 107, 120, 126, 131, 149, 151, 152, 162, 177, 187, 221, 253, 254
- `docs/impl/ir/verify-reproducible-setup-and-work-memo.md`: 281
- `docs/plans/README.md`: 10
- `docs/plans/replace-template-derived-docs-with-ruviform.md`: 5, 31, 39, 63, 67, 68, 80, 84, 96, 102, 103, 109, 116, 120, 126, 137, 146, 150, 157, 158, 159, 170, 177, 214, 237, 238, 239, 256, 259, 267
- `docs/specs/README.md`: 10
- `docs/specs/establish-rust-dev-foundation.md`: 102, 220
- `docs/specs/replace-template-derived-docs-with-ruviform.md`: 5, 56, 57, 58, 69, 73, 78, 80, 85, 90, 104, 112, 124, 130, 135, 141, 147, 157, 171, 192
- `docs/tasks/README.md`: 10
- `docs/tasks/baseline-placeholder-sweep.md`: 5, 26, 29, 50, 57, 58, 60, 61, 69, 70, 71, 72, 89, 96, 97, 99
- `docs/tasks/review-config-files-minimal-edits.md`: 28, 31, 54, 58, 69, 70, 71, 72, 84, 98
- `docs/tasks/rewrite-agents-for-ruviform.md`: 28, 31, 55, 57, 58, 59, 72, 73, 75, 76, 94, 101
- `docs/tasks/rewrite-readme-for-ruviform.md`: 28, 31, 54, 61, 69, 70, 71, 72, 83, 96, 97
- `docs/tasks/run-placeholder-sweep-and-verification.md`: 18, 20, 30, 33, 60, 75, 76, 77, 78, 89, 90, 98, 104, 113
- `docs/tasks/update-apm-identity-and-lockfile.md`: 30, 33, 57, 59, 70, 71, 72, 73

**G6 `language.agnostic`(`docs/` 7 hit):**

- `docs/designs/replace-template-derived-docs-with-ruviform.md`: 84, 163, 177
- `docs/plans/replace-template-derived-docs-with-ruviform.md`: 150, 170
- `docs/tasks/review-config-files-minimal-edits.md`: 58
- `docs/tasks/update-apm-identity-and-lockfile.md`: 59

### Zero-hit Files(全パターン非該当の確認)

以下の追跡ファイルは全 grep で hit なし(テンプレート残存なしを確認): `.gitattributes`、`.github/workflows/ci.yml`、`.gitignore`、`.mise/locks/pipx-apm-cli/0.33.0/pyproject.toml`、`.mise/locks/pipx-apm-cli/0.33.0/uv.lock`、`Cargo.toml`、`Cargo.lock`、`crates/ruviform-core/Cargo.toml`、`crates/ruviform-core/src/lib.rs`、`mise.lock`。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64 / Git Bash / git 2.54.0.windows.1 / node v26.10.0。task の `## Verification` に対応する実測結果:

| Check | コマンド | 結果 |
| --- | --- | --- |
| G1 再現性 | `git grep -in "project.template"` | 本 IR「Results Overview」の hit 一覧と一致(32 hit、非 docs 2 件) |
| G2 再現性 | `git grep -in "Use this template"` | 同上(24 hit、非 docs 1 件) |
| G3 再現性 | `git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存\|言語・フレームワークに依存しない"` | 同上(32 hit、非 docs 6 件) |
| G4 再現性 | `git grep -in "template\|テンプレート" -- . ':(exclude)docs/**'` | 同上(204 hit) |
| IR 存在 | `test -f docs/impl/ir/baseline-placeholder-sweep.md` | exit 0 |
| IR audit | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | findings 0(後述) |
| task audit | `node .agents/skills/doc-status/scripts/audit_docs.js --type task --json` | findings 0(後述) |
| all audit | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` | findings 0(後述) |
| 差分範囲 | `git status --porcelain` | `docs/tasks/baseline-placeholder-sweep.md`・`docs/tasks/README.md`・`docs/impl/ir/README.md` の変更と `docs/impl/ir/baseline-placeholder-sweep.md` の新規のみ。対象ファイル無差分 |

補助確認:

- `git grep -in "language.agnostic"`(G6 補助 grep)→ 9 hit(非 docs 2 件: `.pre-commit-config.yaml:1`・`apm.yml:3`)。
- 全 grep を `git grep` で実行したため、未追跡の新規ファイルは結果に含まれない。ベースライン値はコミット済み追跡ファイルの状態を表す。

## Risks

- **ベースラインの `docs/` hit 数は作業進行で増加する**。本 IR・task status 更新・index 追記はいずれも `docs/` 配下であり、後続スライスが新規 docs を追加するたびに `docs/` 側の catch-all 総数は変動する。T-VERIFY の比較基準は **非 `docs/` 集合(書き換え対象集合)** であり、`docs/` hit の増分は回帰ではない点に注意が必要。
- **`apm.lock.yaml` の hit 数は T-IDENT の lockfile 再生成で変動し得る**。生成物内 APM パッケージ由来パスは除外対象だが、行番号・hit 数は APM 依存解決結果に依存する。T-VERIFY では hit の「種別(APM 由来パスのみ)」を比較し、行番号一致は期待しない。
- **`mise.toml` L8 のような意味的残存はリテラル grep では捕捉不能**。最終スイープの V-1 はリテラル grep が主体のため、comment-edit 完了の確認は T-CONFIG の C-1 個別検証に依存する。
- **`.pre-commit-config.yaml` L1 は最終スイープでも hit し続ける可能性がある**。無変更判定となった場合、`language.agnostic` 系 grep の非 docs 残存として残る。plan C-3/design の「無変更既定」を根拠に除外扱いするかは T-VERIFY の判定事項。

## Follow-ups

- 後続スライス(T-README/T-IDENT/T-AGENTS/T-CONFIG)は本ベースラインの「Rewrite Targets」表を編集範囲の直接入力とする。
- `run-placeholder-sweep-and-verification.md`(T-VERIFY)は F-2/V-1 で最終スイープを実行し、非 `docs/` 集合が `apm.lock.yaml` 等の生成物 hit のみ(可能なら 0)になることを本 IR と比較して確認する。
