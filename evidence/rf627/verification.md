# rf627 — 検証・レビュー ledger

Issue #627 docs-only slice の検証結果を記録する。実行していない検証は Pass と記載しない。

## 1. 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows（MINGW64_NT-10.0-26300、MSYS2/Git Bash 経由） |
| worktree | `G:/orca/workspaces/ruviform/rf627-vtest-contract` |
| branch | `scarecrow173/rf627-vtest-contract` |
| mise | 2026.11.2 windows-x64 |
| node | v26.12.1 |
| toolchain 変更 | なし（docs-only、Rust toolchain 不使用） |

## 2. doc-driven-dev-graph router

- コマンド: `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js`（正規の `.apm` パスは存在せず `.agents/skills/` 配下を使用）
- 結果: `status: "blocked"`, `edgeId: null`, `next: "probe"`
- blocker: `artifact-graph`・`focus-required`・既存の broken relation 多数（Issue #313/#253/#410/#193/#243 等、いずれも本 slice 範囲外の repo 横断の既存状態）
- 判定: **本 slice の文書内容の問題ではなく repo 全体の既存 graph 状態**。doc-chain の整合は `doc_status.js lint` と `doc_maintenance.js index apply` で個別に担保した。

## 3. doc_status lint / impl audit（触れた dir）

- コマンド: `node .agents/skills/doc-status/scripts/doc_status.js lint --dir <dir> --json`（docs/specs, docs/designs, docs/plans, docs/tasks, docs/test-specs）+ `node .agents/skills/impl-doc/scripts/audit_impl_record.js --dir docs/impl/ir`（impl 型は doc_status の lint type 対象外のため専用 audit を使用）
- 結果: 新規 issue-627 文書に対する `broken-relation-link`・`index-missing-entry`・`orphan-index` は、残ファイル作成（`evidence/rf627/verification.md`）と index 再生成で解消。
- 残存: `inconsistent-reciprocal-relation`（severity: warning, blocking: false）が test-spec→plan の2件（`implements`/`derives-from` の逆方向リンク未宣言）で残る。#625 の doc-chain（test-spec625 が plan を implements/derives-from し plan625 が逆方向を持たない）と同型の非ブロッキング警告であり、repo 慣行の範囲内。
- `audit_impl_record.js`: issue-627 IR は finding なし（必須 section 完備）。他 IR の既存 error/warning は repo 横断の既存状態。
- repo 横断の既存 finding: `docs/specs` lint で blocking 106 件（主に他 spec の `traceability-missing-upstream`）。すべて既存文書由来で本 slice 範囲外。

## 4. review-protocol

- 対象: spec・design×2・test-spec・evidence（`policies-baseline-comparison.md`）
- 実施: Worker 共通契約に従い subagent を使わず独立パスで実施。Reviewer A（受入条件との照合）・Reviewer B（反証的レビュー: false green 経路・ポリシー本文との矛盾・schema 網羅性）→ 候補を検証 → judge。
- 指摘と対応:
  1. spec の Audience/Requirements に「承認前の改訂案」前提の陳腐化した表現 → 「着地済み改訂の照合」へ修正（P1 相当、修正済み）
  2. `environment_kind` の説明が EXT-L3 限定に読める表現 → 全 level に適用される旨へ修正
  3. `resolved_ruviform_sha` の Ruviform 自身の Case での扱いが未定義 → 「`sha` と同一値を記録」を明記
  4. front matter relations の非対称（implemented-by/depends-on/derived-by）→ #625 慣行に合わせて修正
- verdict: **ACCEPT**（blocker なし）。spec/design×2/test-spec/plan を `approved`、task を `in-progress`（完了時 `done`）、IR を `draft` に遷移。

## 5. index 再生成

- コマンド: `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index apply --dir <dir>`
- 対象: docs/specs, docs/designs, docs/plans, docs/tasks, docs/test-specs, docs/impl/ir
- 結果: 各 dir で `1 written, 0 blocked`。`docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md` を再生成。

## 6. 衛生チェック（正規検証コマンド）

- コマンド: `mise exec -- prek run --all-files`
- OS: Windows（MINGW64）、toolchain: mise 2026.11.2 / node v26.12.1
- exit code: **0**
- 結果: 全 hook Passed（trim trailing whitespace / fix end of files / check yaml / check toml / mixed line ending / fix utf-8 BOM / detect private key / actionlint）。`check illegal windows names` は no files to check で Skipped。
- Cargo 系タスク（check/lint/test/build/verify）は docs-only slice のため対象外（タスク指定どおり）。`*-msrv`・`*-full` lane も明示的対象外。

## 7. 変更範囲の確認

- `git status --short`: 変更は `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/issue-627-*`、`docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md`（生成 index）、`evidence/rf627/` のみ。
- `POLICIES.md`・`docs/development-environment.md`・`mise.toml`・`.github/workflows/ci.yml`・`ci/summary/`・`crates/`・`Cargo.lock` に差分なし。

## 8. 未検証・NotRun 項目

- 三 repo `POLICIES.md` の**意味論的正しさ**の審査: 本 slice は行単位照合と適合判定まで。最終承認は三 repo のレビュー（外部 gate）。
- 旧物理 Required Case の実状態遷移: 移管台帳の確定まで。各 repo の Issue/PR で実施。
- #628〜#631 の実装物の動作: 本 slice 範囲外（契約のみ）。
- 仮想 Endpoint/Test Plugin/GUI Runner の実動作: 未実装（後続 Issue）。
- 統合検証: docs-only のため CI 証拠は merge 後の対象 SHA の CI 結果に委譲（ローカル検証＝衛生チェックのみ）。
