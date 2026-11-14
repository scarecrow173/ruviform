# RF558 Script/Analyzer 契約 — 検証記録

- Issue: https://github.com/scarecrow173/ruviform/issues/558
- 作業 branch: `docs/558-script-contract`
- 作成日: 2026-11-14
- 対象スライス: **DESIGN-ONLY**（実装コード・crate・CI・依存変更なし。文書のみ）
- 判定語彙: `POLICIES.md` §2/§3（要件状態 Required/Deferred/NotApplicable、実行結果 Pass/Fail/NotRun/Unsupported）
- 検証レベル: 文書のみの変更のため OS-L1 の Rust build/test は対象外。Markdown/構成ファイル/リンクの軽量チェックが原則（Worker 共通契約・タスク固有「正規の検証コマンド」）。

## 1. 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows 11 x64（worktree `C:\Users\akiyoshi\orca\workspaces\ruviform\rf558-script0`） |
| Rust toolchain | rustc/cargo 1.99.1（dev toolchain。`mise.toml`/`mise.ci.toml` 管理） |
| Node.js | v26.12.1（doc ツール実行用） |
| mise | 2026.11.2 windows-x64 |
| 対象 revision | branch `docs/558-script-contract`（base main `2613b2388e0413af15913032421c57857e393682`） |

## 2. 実行した検証（実コマンド・結果）

| # | コマンド | 目的 | exit code | 結果 |
| --- | --- | --- | --- | --- |
| V-01 | `mise run fmt-check`（= `cargo fmt --all -- --check`） | Rust fmt 非破壊チェック（文書のみだが正規検証として実行） | 0 | **Pass** |
| V-02 | `mise exec -- prek run --files <変更ファイル>` | Markdown/リンク/末尾空白/EOF/BOM 等の軽量チェック | 0 | **Pass**（trim trailing whitespace・fix end of files・mixed line ending・fix utf-8 BOM・detect private key 全て Passed。check yaml/toml/actionlint は対象ファイルなしで Skipped） |
| V-03 | `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir <dir>` → `index apply --dir <dir>` | 触れた docs ディレクトリの管理 index 再生成（手編集禁止のため preview-first） | 0 | **Pass**（docs/specs・designs・adr・test-specs・plans・tasks・impl/ir・impl/exp の 8 dir で `index apply` が `[written]`。docs/impl コンテナは管理 index なしで `rejected`=仕様どおり） |
| V-04 | `node .agents/skills/doc-status/scripts/audit_docs.js --type <type> --dir docs/<dir>` | spec/design/test-spec/plan/task の front matter・relation・構造監査 | 0 | **Pass（自スコープ）** — 自分の文書に起因する error/blocking なし。repo 全体には他 Worker・既存文書の pre-existing finding が多数残存（§3 参照） |
| V-05 | `node .agents/skills/adr-doc/scripts/audit_adr.js --dir docs/adr` | ADR 監査 | 0 | **Pass（自スコープ）** — 自分の ADR に error なし。`audit_adr` は全 relation を ADR 間リンクとして解決するため、spec/design/evidence への正当な cross-type reference が "missing ADR" warning として出るのは repo 全体で 396 件の既存仕様的警告（自分の分を含む）。blocking ではない |
| V-06 | `node .agents/skills/doc-status/scripts/doc_status.js audit --type all` | repo 全体の文書健全性 audit | 0 | **参考記録** — 1468 文書・761 finding・405 blocking は全て pre-existing（他 Issue・他 Worker の文書）。自分の文書に起因する blocking は `evidence/rf558/verification.md` 未作成による broken-relation-link と task の `changes.generated` 形式で、いずれも本記録作成・修正で解消済み（§4 で再監査） |

## 3. repo 全体の pre-existing finding（自スコープ外・報告のみ）

`doc_status.js audit --type all` は repo 全体で 405 blocking finding を報告するが、これらは本タスク着手前から存在する他文書の問題であり、本スライスの編集範囲外のため修正しない。例:

- `docs/adr/*`・`docs/test-specs/*` の Artifact ID 形式違反（`ADR-2026...`/`TESTSPEC-...`/`TSPEC-<22文字でない>` 等）。
- `docs/tasks/task-312-*.md` の `relations.changes` が array（object 期待）・ID prefix 違反。
- `docs/plans/issue-565-*.md` の PLAN ID prefix 違反。
- spec/test-spec の `verifies`/`derives-from` の双方向 link 不足 warning が多数。
- `route_graph.js` が返した repository-wide broken relation（Issue #313/#253/#410/#193/#243/#464 周辺）は doc-driven-dev-graph の `blocked` 結果として記録済み。本タスクは自分の文書鎖の整合のみを保証し、repo 全体の graph 修復は範囲外。

これらは Coordinator/別タスクの管轄であり、本 PR では「自分の文書が新規 blocking を増やさない」ことのみを保証する。

## 4. 自スコープ文書の最終状態

| 文書 | front matter | relation | index |
| --- | --- | --- | --- |
| `docs/specs/issue-558-script-contract.md` | OK | `implemented-by`→design+plan、`verified-by`→test-spec、`derives-from`→Issue #558 | `docs/specs/README.md` 再生成済み |
| `docs/designs/issue-558-script-contract-design.md` | OK | `implements`→spec、`depends-on`/`derives-from`→ADR+spec、`verified-by`→test-spec | `docs/designs/README.md` 再生成済み |
| `docs/adr/issue-558-script-backend-contract.md` | OK | `derives-from`→Issue #558、`derived-by`→design、`related`→public-enum ADR | `docs/adr/README.md` 再生成済み |
| `docs/test-specs/issue-558-script-contract.md` | OK | `verifies`→spec+design+ADR、`derives-from`→design | `docs/test-specs/README.md` 再生成済み |
| `docs/plans/issue-558-script-contract-plan.md` | OK | `implements`→spec、`verified-by`→test-spec | `docs/plans/README.md` 再生成済み |
| `docs/tasks/issue-558-script-contract-task.md` | OK | `implements`→plan、`verified-by`→test-spec、`changes.generated` を object 形式に修正済み | `docs/tasks/README.md` 再生成済み |
| `docs/impl/ir/issue-558-script-contract.md` | OK | `implements`→task、`metadata.experiments.adopted`→exp | `docs/impl/ir/README.md` 再生成済み |
| `docs/impl/exp/issue-558-script-contract.jsonl` | OK（JSONL v1） | `task`→task、`experiment`→自身のパス | `docs/impl/exp/README.md` 再生成済み |

## 5. 未検証・未実行項目（NotRun / Deferred / Unsupported）

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| 実 Runtime 共存（Luau+Rhai 同一バイナリ） | **NotRun** | #559 PoC の受入。契約は FX-P-07・FX-N-01〜N-08 を割当済みだが実測なし |
| 3OS native build/link/run | **NotRun** | #559/#560 の受入。OS-L1 実機証拠は未取得 |
| Luau.Analysis C++ native FFI・配布 | **NotRun** | #560 の受入。契約は Analyzer Port のみ規定 |
| Rhai 本番採用 | **Deferred** | #559 PoC + Consumer 受入後に判断 |
| 共通 API の製品昇格・実装 crate | **Deferred** | #559/#560 PoC + #553 判定後 |
| AudioLens への Script 適用 | **Deferred** | AL #127 は現時点 Script 非要求。Ruviform 昇格で AL 依存木に影響が出た時点で再評価 |
| `Send`/`Sync` 可能 Session 形態 | **Deferred** | #559 PoC で配置特性を実測後 |
| Synvelis/AudioLens 独立受入 | **NotRun** | #294・SV #28・AL #127 の後続ゲート |
| GUI-L2/EXT-L3（実 GUI/Plugin/デバイス） | **NotApplicable** | 本スライスは文書のみで実機能を使わない。fake/headless 証拠は実受入の代替にしない |
| Workspace Build/Test（`check`/`lint`/`test`/`build`） | **NotApplicable** | コード変更なし（POLICIES §5: 文書のみは軽量チェックが原則） |

## 6. review-protocol 結果

- 実施: 本スライスの文書（spec/design/ADR/test-spec/plan/task/IR/exp/evidence）に対し、客観的チェック（audit_docs/audit_adr/doc_status/fmt-check/prek）を先に実行し、続いて協調的レビュー（Reviewer A 相当: spec 受入条件→design 契約条項→test-spec Rule/Example→evidence 対応の写像確認）と敵対的レビュー（Reviewer B 相当: Fail-Closed 網羅性・製品型非依存・未実測の Pass 偽装・relation 双方向性の点検）を実施。
- 確認した Issue Candidate と処置:
  - `evidence/rf558/verification.md` 未作成による broken-relation-link（test-spec/plan/task）→ **CONFIRMED P1** → 本ファイル作成で解消。
  - task `changes.generated` が string 配列で object 期待違反 → **CONFIRMED P1** → object 形式に修正。
  - plan が test-spec を `verifies` に置き `verified-by` でない → **CONFIRMED P2** → `verified-by` へ修正。
  - test-spec が design を `verifies` に含めず design の `verified-by` と非双方向 → **CONFIRMED P2** → design を `verifies` に追加。
  - spec `implemented-by` に plan がなく plan `implements`→spec と非双方向 → **CONFIRMED P2** → spec `implemented-by` に plan を追加。
  - ADR `derived-by`→design と design `derives-from` 非双方向 → **CONFIRMED P2** → design `derives-from` に ADR を追加。
  - Issue 要求の「Analyzer unavailable」障害が判定表になし → **CONFIRMED P2** → F-18/FX-N-24 を追加。
  - Issue 要求の「Plugin SDK 具象型」「OS handle」非露出が禁止越境に不十分 → **CONFIRMED P2** → design C3・test-spec Guarantees に追加。
  - Issue 要求の「OS-L1/GUI-L2/EXT-L3 区別」「fake/headless 非代替」の明記不足 → **CONFIRMED P2** → design Non-Functional・本 verification.md §1/§5 に追加。
  - Issue 要求の「Existing Ruviform Exec/Job/IO/CLI 非重複」で IO が未記載 → **CONFIRMED P3** → consumer-applicability §4 に `ruviform-io` NotApplicable を追加。
  - test-spec に Fail-Closed/Fault Matrix の literal 写像表がなく Issue の「test-specs に固定」要求を弱く満たす可能性 → **CONFIRMED P2** → test-spec に「Fail-Closed / Fault Matrix 写像」節を追加。
- 結果: **blocker なし**（CONFIRMED P0/P1 は全て解消、P2/P3 も本スライス範囲内で対応）。レビュー回数 1 巡（修正が前提・構造・責務境界を変えないため再レビュー不要と判定）。

## 7. 結論

- 文書チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と `evidence/rf558/` が完成し、管理 index が `doc-maintenance` で再生成され、front matter・relation・構造監査で自スコープの blocker がない。
- `mise run fmt-check`・`prek run --files` が exit 0 で Pass。
- 実装・Runtime・3OS・FFI・Consumer 受入は全て NotRun/Deferred であり、Pass に偽装していない。#559/#560 の PoC が本契約を前提に着手可能。
