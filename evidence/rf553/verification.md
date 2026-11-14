# RF553 Workflow/CLI orchestration契約 — 検証記録

- Issue: https://github.com/scarecrow173/ruviform/issues/553
- 作業 branch: `docs/553-workflow-orchestration`
- 作成日: 2026-11-14
- 対象スライス: **DESIGN-ONLY**（実装コード・crate・CI・依存変更なし。文書のみ）
- 判定語彙: `POLICIES.md` §2/§3（要件状態 Required/Deferred/NotApplicable、実行結果 Pass/Fail/NotRun/Unsupported）
- 検証レベル: 文書のみの変更のため OS-L1 の Rust build/test は対象外。Markdown/構成ファイル/リンクの軽量チェックが原則（Worker 共通契約・タスク固有「正規の検証コマンド」）。

## 1. 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows 11 x64（worktree `C:\Users\akiyoshi\orca\workspaces\ruviform\rf553-autx1`） |
| Rust toolchain | rustc/cargo（dev toolchain。`mise.toml`/`mise.ci.toml` 管理） |
| Node.js | doc ツール実行用（`doc-maintenance`/`doc-status`/`adr-doc`/`impl-doc` scripts） |
| mise | windows-x64 |
| 対象 revision | branch `docs/553-workflow-orchestration` |

## 2. 実行した検証（実コマンド・結果）

| # | コマンド | 目的 | exit code | 結果 |
| --- | --- | --- | --- | --- |
| V-01 | `mise run fmt-check`（= `cargo fmt --all -- --check`） | Rust fmt 非破壊チェック（文書のみだが正規検証として実行） | 0 | **Pass** |
| V-02 | `mise exec -- prek run --files <変更ファイル>` | Markdown/リンク/末尾空白/EOF/BOM/秘密鍵等の軽量チェック | 0 | **Pass**（trim trailing whitespace・fix end of files・mixed line ending・fix utf-8 BOM・detect private key 全て Passed。check yaml/toml/actionlint/windows-names は対象ファイルなしで Skipped） |
| V-03 | `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir <dir>` → `index apply --dir <dir>` | 触れた docs ディレクトリの管理 index 再生成（手編集禁止のため preview-first） | 0 | **Pass**（docs/specs・designs・adr・test-specs・plans・tasks・impl/ir の 7 dir で `[written]`、docs/impl/exp は `in-sync` で変更不要） |
| V-04 | `node .agents/skills/doc-status/scripts/audit_docs.js --type <type> --dir docs/<dir>` | spec/design/test-spec/plan/task の front matter・relation・構造監査 | 0 | **Pass（自スコープ）** — 自分の文書に起因する error/blocking なし。repo 全体には他 Worker・既存文書の pre-existing finding が多数残存（§3 参照） |
| V-05 | `node .agents/skills/adr-doc/scripts/audit_adr.js --dir docs/adr` | ADR 監査 | 0 | **Pass（自スコープ）** — 自分の ADR に error なし。`audit_adr` は全 relation を ADR 間リンクとして解決するため、spec/design/evidence への正当な cross-type reference が "missing ADR" warning として出るのは repo 全体で既存仕様的警告（自分の分を含む）。blocking ではない |
| V-06 | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --dir docs/impl/ir` | IR 監査 | 0 | **Pass（自スコープ）** — 自分の IR の required section（Summary/Context/Implementation/Related Experiments/Validation/Risks/Follow-ups）を整備済み |
| V-07 | `node .agents/skills/doc-status/scripts/doc_status.js audit --type all` | repo 全体の文書健全性 audit | 0 | **参考記録** — pre-existing finding（他 Issue・他 Worker の文書）が多数残存。自分の文書に起因する blocking は `evidence/rf553/verification.md` 未作成による broken-relation-link と design `defers` の非 path 文字列で、いずれも本記録作成・修正で解消 |

## 3. 実行しなかった検証（NotRun/Unsupported）

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| `mise run check` / `test` / `test-rt` | **NotRun** | コード変更がないため対象外。文書契約の照合のみであり runtime Pass ではない。実 Job/Resume/Cancel/OS の受入は Synvelis #267・AudioLens #127/#189 の各 Gate が Owner |
| `mise run test-ci-consumer-verify` | **NotRun** | 同上。Synvelis/AudioLens 独立 SHA tuple・Candidate Case のレビューは文書照合として実施（`evidence/rf553/consumer-applicability.md`） |
| 実 Runtime 共存・3OS・native FFI | **NotRun** | rf#559/#560 の PoC 範囲。本 Issue は Script Runtime/Analyzer を対象外 |
| Workflow Engine 実装・両 Consumer 移行・OS Native 実走 | **NotRun** | 本設計は DESIGN-ONLY。実装は再評価 Trigger 成立後の別 Issue（ADR 参照） |
| Synvelis AUTX-05/06/03 具象実装 | **NotRun** | Synvelis 側 Owner（synvelis#264/#265/#262）。generic 契約の adapter 適合は各 AUTX Issue の受入で確認 |
| AudioLens orchestration 適用 | **NotRun/Deferred** | 現要件で consumer-only 継続。AudioLens/第三 Consumer の独立した必要性を再評価 Trigger にする |

## 4. 設計レビュー（review-protocol）

文書チェーン完了後、`review-protocol` に従い spec/design/ADR/test-spec をレビューする。blocker が残っていない状態を承認として扱う（Worker 共通契約）。レビュー結果は PR 本文に記録する。

## 5. 補足

- 本記録は文書のみの変更に対する検証であり、ローカル検証＝変更範囲の検証、統合検証は merge 後の対象 SHA の CI 証拠（POLICIES.md / Worker 共通契約）。
- `evidence/rf553/contract-trace.md`・`evidence/rf553/consumer-applicability.md` が契約・判定の正本証拠。
