---
id: "IMPL-034Ix98fGRcsRNYZ3EZnZJ"
type: "impl"
status: "completed"
title: "Rewrite AGENTS for Ruviform"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
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
        path: "docs/tasks/rewrite-agents-for-ruviform.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/rewrite-agents-for-ruviform.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/replace-template-derived-docs-with-ruviform.md"
    - "../../designs/replace-template-derived-docs-with-ruviform.md"
    - "../../designs/dev-foundation.md"
    - "../../specs/replace-template-derived-docs-with-ruviform.md"
    - "../../adr/replace-template-derived-docs-metadata-with-ruviform.md"
    - "verify-reproducible-setup-and-work-memo.md"
    - "baseline-placeholder-sweep.md"
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

# Rewrite AGENTS for Ruviform

## Summary

task `TASK-034IwCiByN74V3hysJ9iTR`（plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-AGENTS、Phase G-1〜G-8）として、`AGENTS.md` を design の再編成表どおり節単位で改稿した。冒頭を Ruviform の作業契約に書き換え、`## 開発・設計原則`（7 サブセクション）は verbatim 保持、`## リポジトリの作業契約`・`## 開発環境（mise）`・`## AIエージェント環境（APM）`・`## Gitと安全性` を現状適合させ、新規節 `## Rust ワークスペースと暫定開発規約`（5 サブセクション）を追加した。テンプレート前提の禁止事項・「派生リポジトリ/派生プロジェクト」表現は除去し、`git grep` で残存ゼロを確認した。

## Context

- 実施タスク: `docs/tasks/rewrite-agents-for-ruviform.md`（`TASK-034IwCiByN74V3hysJ9iTR`、G-1〜G-8 が本タスクの全作業）
- 上流: `docs/plans/replace-template-derived-docs-with-ruviform.md`（Phase G-1〜G-8 の正本）、`docs/designs/replace-template-derived-docs-with-ruviform.md`（AGENTS 再編成表・表現ルール・禁止記載の直接入力）、`docs/specs/replace-template-derived-docs-with-ruviform.md` Req 2（作業契約の Ruviform 固有化要件）、`docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`（作業契約固有化・LFS ポリシー文書化の決定）
- 事実の一次ソース: `docs/impl/ir/verify-reproducible-setup-and-work-memo.md`「作業メモ（#3 転記用）」（検証済みセットアップ手順・lockfile 再生成条件・暫定開発規約・LFS 方針）と `docs/designs/dev-foundation.md` §暫定開発規約
- 前提: `docs/impl/ir/baseline-placeholder-sweep.md`（`AGENTS.md:3,5,75,80,91` が書き換え対象であるベースライン記録）、T-IDENT 完了済み（`apm.yml` の `name: ruviform` 確定）

## Implementation

### Changes

`AGENTS.md`（110 行 → 170 行）の節ごとの編集内容と design 再編成表の keep/rewrite/add 判定:

| 旧節 | 判定 | 編集内容 |
| --- | --- | --- |
| 冒頭（旧 L1–L5） | rewrite | 「言語・フレームワークに依存しない開発基盤のテンプレート」「派生リポジトリでは…追記してください」を除去し、「このリポジトリは Ruviform — Rust を中心とした再利用可能なマルチメディア基盤（設計・初期セットアップ段階）」とエージェント共通の作業契約である旨に置き換え（L3–L4） |
| `## 開発・設計原則`（旧 L7–L66、7 サブセクション） | keep（verbatim） | 無変更。`git diff` の hunk が全て同節の範囲外であることを確認（G-7） |
| `## リポジトリの作業契約` | section-edit | 箇条 5 件（旧 L70–L74）は verbatim 保持。旧 L75「このテンプレートに言語・フレームワーク固有の構成や、不要な `.github/`・`docs/` ディレクトリを追加しない。派生プロジェクトで必要に応じて定義する。」を削除し、L74 に「実装されていない API・クレート・検証を、実在するものまたは実行済みと報告しない。」を追加 |
| `## 開発環境（mise）` | section-edit | 旧 L80 を「`mise.toml` は共通基盤ツール＋ `rust = "1.99.0"` ＋ actionlint を管理し検証タスクも同ファイルが正本」に書き換え。初回セットアップ列を IR 検証済み手順に同期（`mise install --locked` 正本・caveat 注記・フォールバック `mise install`・`mise run setup`・厳格検証 `mise exec -- apm install --frozen`、検証環境限定を明記）。「派生リポジトリのmiseタスク」を「`mise.toml` のタスク」に修正。lockfile 再生成条件の表（`mise.lock`/`apm.lock.yaml`/`Cargo.lock` のトリガと手順）と不整合時の復旧手順を追記（L101–L109） |
| `## Rust ワークスペースと暫定開発規約` | add（新規節、L111–L152） | 5 サブセクション: `### ワークスペース構成`（ルート `Cargo.toml` の `[workspace]`/`workspace.package`/`workspace.lints`/`[profile.dev]`/`[profile.ci]`＋仮置き最小メンバー `crates/ruviform-core`、最終分割・公開 API は Issue #1）、`### 検証入口`（cargo は `mise run` 経由・12 タスクの対応表・`*-ci`/`verify-ci` は CI 用途限定）、`### 暫定規約`（`unsafe` 原則禁止＋SAFETY コメント付き局所許可・`-D warnings` で実質 deny、default feature 最小、依存追加は理由記録、`workspace.lints` 正本＋ `-D warnings` ゲート、公開 API の例は doctest）、`### 暫定原則`（下位基盤クレートは上位に依存しない・依存 DAG は #1、SIMD 前提・不変条件の詳細契約は #1、RT/オフラインの設計意図＝RT 音声経路でブロッキング操作・意図しない allocation を避ける方向・未確定 API を RT-safe と断定しない）、`### ライセンス注記`（`LICENSE` 不在・未決定・`license = "MIT"` は仮置き） |
| `## AIエージェント環境（APM）` | keep（微修正） | 先頭箇条を「`apm.yml`（パッケージ名 `ruviform`）を正本とする」に修正（L156）。他箇条は verbatim 保持 |
| `## Gitと安全性` | keep + 1 件追記 | L169 に LFS フィクスチャ方針の箇条を追加（小さなテストフィクスチャは非メディア形式で Git・CI は必要になるまで LFS オブジェクトを取得しない・現行 `ci.yml` も LFS 取得なし）。併せて lockfile 追跡箇条に `Cargo.lock` を追加（実際に追跡されている事実との整合） |

- `docs/tasks/rewrite-agents-for-ruviform.md` — status `todo` → `done`、G-1〜G-8・Done When・Verification の全 checkbox を `[x]`＋結果注記、`implemented-by` に本 IR を追記。`updated` は同日（2026-10-19）のため据え置き
- `docs/impl/ir/rewrite-agents-for-ruviform.md` — 新規（本 IR。`new_impl_record.js` で in-progress 作成後に completed へ更新）
- `docs/impl/ir/README.md`・`docs/tasks/README.md` の index 更新は orchestrator 側の責務として委譲（本 IR の変更対象外）

### Decisions During Implementation

- **新規節の挿入位置を `## 開発環境（mise）` と `## AIエージェント環境（APM）` の間にした**。design 再編成表は位置を指定しないが、新規節の検証入口対応表が mise 節の「タスクの正本は `mise.toml`」を前提とするため、mise 節の直後に置くことで参照が自然になると判断した。
- **「初期化後は `mise.lock` を生成・確認してコミットし…」の記述を削除した**。`mise.lock` は既に生成・コミット済みで実在するため、現行の復元手順は `mise install --locked` が正本であり、再生成は lockfile 表のトリガ条件側に集約した。
- **lockfile 追跡箇条（`## Gitと安全性`）に `Cargo.lock` を追加した**。追跡方針の記述を一箇所（Git 節）に保ち、再生成条件を別箇所（mise 節の表）に集約する分担とし、`Cargo.lock` が追跡済みの事実と整合させた。
- **SIMD 暫定原則は spec の文言範囲に留めた**。spec Req 2 は「SIMD の前提・不変条件は暫定的な設計原則として示し詳細契約は #1 で確定」とのみ定めるため、具体的な不変条件を創作せず「暫定原則として示し詳細契約は #1 で確定」と転記した。
- **`fmt` タスクも対応表に含めた**。`mise.toml` に `[tasks.fmt]` が実在し、README には未記載だが AGENTS は作業契約としてタスク全容の対応表を持つ design 指示のため、`fmt`（ソース書き換え）であることを用途列で明記した。
- **Issue #1 は初出のみリンク化した**。README と同じ導線形式（`Issue [#1](...)`）を採用し、2 回目以降は `#1` 表記に統一した。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64 / Git Bash / node v26.10.0 / git（`git grep` は追跡ファイルの working tree 内容を検索）。task の `## Verification` に対応する実測結果:

| Check | コマンド | 結果 |
| --- | --- | --- |
| テンプレート文言なし | `git grep -in "template\|テンプレート" -- AGENTS.md` | exit 1（0 hit） |
| 派生表現なし | `git grep -n "派生リポジトリ\|派生プロジェクト" -- AGENTS.md` | exit 1（0 hit） |
| 暫定規約の記載 | `grep -n "SAFETY\|workspace.lints\|doctest" AGENTS.md` | L115（`workspace.lints`）・L138（SAFETY・`workspace.lints`・`-D warnings`）・L141（`workspace.lints` 正本）・L142（doctest）に一致 |
| セットアップ・厳格検証の記載 | `grep -n "mise install --locked\|mise run setup\|apm install --frozen" AGENTS.md` | L84/87/90/93/97 に一致 |
| prek フック | `mise exec -- prek run --all-files` | exit 0（trim trailing whitespace / fix end of files / check yaml / check toml / mixed line ending / fix utf-8 bom / detect private key: Passed、check illegal windows names: Skipped） |
| 原則節の保持（G-7） | `git diff AGENTS.md` | hunk は旧 L1–8・L72–100・L107–110 のみ。原則節（旧 L7–66）は全 hunk 外で verbatim 保持 |
| mise タスク名の実在 | `for t in setup fmt fmt-check check lint test build check-ci lint-ci test-ci build-ci verify-ci; do grep -q "^\[tasks.$t\]" mise.toml ...` | 全 12 タスクが `mise.toml` L18–L86 の `[tasks.*]` に実在 |
| 節構成 | `grep -n "^## " AGENTS.md` | `## 開発・設計原則`(L6)・`## リポジトリの作業契約`(L67)・`## 開発環境（mise）`(L76)・`## Rust ワークスペースと暫定開発規約`(L111)・`## AIエージェント環境（APM）`(L154)・`## Gitと安全性`(L163) の 6 節 |
| `ci.yml` LFS 未取得の実測 | `grep -in "lfs" .github/workflows/ci.yml` | exit 1（0 hit。G-5 箇条の記載と一致） |
| `LICENSE` 不在 | `test ! -f LICENSE` | exit 0（ライセンス注記の記載と一致） |
| task audit | `node .agents/skills/doc-status/scripts/audit_docs.js --type task --json` | blocking findings 0。warning は `index-metadata-mismatch`（`docs/tasks/README.md` が本タスクを `todo` のまま記載。index 更新は orchestrator 責務のため本 IR の変更対象外） |
| IR audit | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | findings 0 |

補助確認（G-8 自己監査・design 表現ルール 6 区分と禁止記載 7 項目）:

- ラベル適用: 検証済み＝「Windows 11 x64・Git Bash で検証済み」＋環境限定（L80）、未検証＝「macOS / Linux は未検証」（L80）、未確定＝「#1 で確定するまで未確定」（L148）・「未決定」（L152）、仮置き＝「仮置き」（L115・L116・L152）。AGENTS は構想/検討中のプロダクト項目を列挙しない（README が担当）ため該当ラベルの対象なし
- 禁止記載 7 項目: いずれもなし。`LICENSE` 不在は実測、未作成クレートへの言及なし（`crates/ruviform-core` のみ・仮置き明記）、RT-safe の断定なし（L148「断定しない」と明記）、GitHub Actions 実ディスパッチ成功の記載なし（`ci.yml` への言及は LFS 未取得の実測事実のみ）、macOS/Linux 検証済みの記載なし、「Use this template」系の手順なし（上記 grep で 0 hit）、存在しない文書へのリンク・言及なし（リンクは Issue #1 のみで実在）
- `cargo` 直接実行の案内がないこと: コマンド列は全て `mise install`/`mise run`/`mise exec` 経由で記載（lockfile 表の `cargo build`/`cargo generate-lockfile` は再生成手順の説明として許容）

未実行の検証: `mise run fmt-check`/`check`/`lint`/`test`/`build`/`verify-ci` の再実行（本タスクは `AGENTS.md`・task 文書の変更のみで Rust コード・`mise.toml` への影響なし。最終スモークは T-VERIFY の検証範囲）

## Risks

- **暫定原則（依存方向・SIMD・RT/オフライン意図）が確定契約と誤読される残存リスク**。節名を「暫定原則（詳細契約は #1 で確定）」とし各箇条でも #1 確定を明記して抑止。文言の最終担保は PR レビュー観点（design Risks と同じ留保）
- **#1 の決定後に暫定規約・未確定の記述が陳腐化する**。各所に「#1 で確定」の導線を残したため追従先は明確
- **`mise install --locked` の環境 caveat は environment-dependent**。global mise config を持たない環境では発生しないが、フォールバックを併記済み（IR メモ・README と同じ記述）

## Follow-ups

- `docs/impl/ir/README.md`（IR index）と `docs/tasks/README.md`（task index）への本 IR・本タスクの反映は orchestrator が行う
- `run-placeholder-sweep-and-verification.md`（T-VERIFY）が最終スイープで `AGENTS.md` のベースライン 5 hit（旧 L3/L5/L75/L80/L91）消滅と V-2/V-3/V-9 の内容監査を確認する。本 IR の grep 実測値が比較入力となる
- #1 の詳細設計確定時に暫定規約・暫定原則・ライセンス注記・検証入口の記述を追従更新する
