---
id: "PLAN-034XPQz5nuJOYW06YM1ygQ"
type: "plan"
status: "completed"
title: "Issue #419 後継ADR verdict逆反映 docs-only slice 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/419"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-419-adr-backreflection.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-419-adr-backreflection.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-419-adr-backreflection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-419-adr-backreflection.md"
    - "docs/tasks/issue-419-adr-backreflection.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-419-adr-backreflection.md"
    - "docs/designs/issue-419-adr-backreflection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-419-adr-backreflection.md"
  references:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
  defers: []
  deferred-by: []
---

# Issue #419 後継ADR verdict逆反映 docs-only slice 実装計画 Implementation Plan

## Goal

Issue #419 の「Coordinator 決裁済み verdict（outcome = CONDITIONAL）を後継 ADR へ逆反映する」docs-only slice として、対象 ADR の More Information 逆反映欄・Confirmation・frontmatter を更新し、slug `issue-419-adr-backreflection` の文書チェーン 7 ファイルと管理 index 再生成を完成させて検証を通す。

## Scope

**1 実装ストリーム**: 文書のみ。`crates/**`・`Cargo.toml`/`Cargo.lock`/`mise.toml`/CI workflow・`evidence/`・他 ADR・POLICIES/AGENTS・Consumer コードは対象外。実装コード・新規 CI・ADR status 変更・Issue 投稿は生成しない。

## Upstream Documents

- `docs/specs/issue-419-adr-backreflection.md`
- `docs/designs/issue-419-adr-backreflection.md`
- `docs/test-specs/issue-419-adr-backreflection.md`

## Design Inputs

- `docs/designs/issue-419-adr-backreflection.md`（approved）
- `evidence/rf262-gpui-verdict/handoff-419.md`（逆反映内容の起草元）
- `evidence/rf262-gpui-verdict/verdict-matrix.md`（判定語彙・Case 正本）

## File Map

| ファイル | 種別 | 内容 |
| --- | --- | --- |
| `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md` | 既存 ADR（変更） | More Information 逆反映欄の dated 記入・Confirmation dated 実施記録・frontmatter `updated`/`references` 更新。`status: "proposed"` 不変 |
| `docs/specs/issue-419-adr-backreflection.md` | spec | 問題・Audience・Scope・Non-goals・受入条件・Evidence |
| `docs/designs/issue-419-adr-backreflection.md` | design | 逆反映の記録構造（D1）・判定語彙/誠実性規則（D2）・文書チェーン（D3） |
| `docs/test-specs/issue-419-adr-backreflection.md` | test-spec | 逆反映記述の不変条件を Rule（R-RECORD/R-VOCAB/R-SCOPE/R-HONESTY）と Example（EX-1〜4）へ固定 |
| `docs/plans/issue-419-adr-backreflection.md` | plan | 本計画 |
| `docs/tasks/issue-419-adr-backreflection.md` | task | 文書チェーン完成・検証タスク |
| `docs/impl/ir/issue-419-adr-backreflection.md` | impl/ir | 実装記録 |
| `docs/impl/exp/issue-419-adr-backreflection.md` | impl/exp | verdict 転記・語彙適用・文書規約適用の試行記録（`.md` 自由形式、issue-262 先例） |
| `docs/adr/README.md`・`docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/README.md` | 生成 index | `doc-maintenance index apply`・`sync_impl_index.js`・`docs/adr` index 手順で再生成 |

## Dependency Graph

`spec → design → test-spec → plan → task → ir/exp`。design は spec を implements、plan は spec を implements かつ design に depends-on、task は plan を implements、ir は task を implements。test-spec は spec/design を verifies。

## Risk Register

| リスク | 影響 | 対応 |
| --- | --- | --- |
| 「推奨」を「決裁」と読み替える記述 | verdict 権限の偽装 | design D2 の決裁/推奨分離。全 chain 文書で「Coordinator 決裁済みの記録」と明記 |
| `NotRun`/`Deferred`/`Unsupported`/`Fail` の `Pass` 化 | L2/L3 非保証の隠蔽 | design D2 の語彙分離。test-spec R-VOCAB で固定 |
| ADR status の誤変更 | 決裁権限の侵害 | `status: "proposed"` 不変を spec/Acceptance・task Done When・review で確認 |
| 既存本文の不用意な書き換え | ADR 契約の破壊 | design D1.4 の触れない範囲。Decision Outcome・判定フロー・縮退順序・責務マップ・Capability Matrix・Verification は不変 |
| index の手編集・生成漏れ | 生成物ドリフト | `index plan`/`index apply`・`sync_impl_index` のみを mutator とし全 dir `in-sync` を確認 |
| Issue #419 の auto-close | 残 DoD の隠蔽 | PR 本文は `Refs #419`（`Closes` 禁止） |

## Rollback Strategy

文書のみの変更であり、`git revert` で ADR 変更・文書チェーン・index を戻せる。コード・依存・CI・evidence への影響はない。

## Verification Matrix

| 検証 | 方法 | 結果 |
| --- | --- | --- |
| 文書 front matter・relation・構造 | `audit_docs`（type 別） | 自スコープ error なし |
| impl 記録・実験ログ | `audit_impl_record.js`/`audit_experiment_log.js --json` | error なし |
| 管理 index 整合 | `doc-maintenance index plan`（`docs/adr`+触れた全 dir）→ `index apply` → 再 plan で `in-sync` | 全 dir `in-sync` |
| impl index | `sync_impl_index.js --kind ir`・`--kind exp`（preview → `--apply`） | `in-sync` |
| Markdown/リンク軽量チェック | `mise exec -- prek run --files <変更ファイル>` | Pass |
| Rust fmt | `mise run fmt-check` | docs-only だが正規軽量チェックとして実行 |
| Runtime/Consumer/OS 実機・CI lane | — | **NotRun**（docs-only slice。POLICIES §5。統合検証は merge 後対象 SHA の CI 証拠） |

## Implementation Sequence

- [x] spec/design/test-spec/plan/task の文書チェーン作成
- [x] ADR More Information 逆反映欄の dated 記入（outcome = CONDITIONAL・判定根拠・証拠リンク）
- [x] ADR Confirmation への dated 実施記録追加・frontmatter `updated`/`references` 更新
- [x] impl/ir・impl/exp 作成と audit
- [x] 管理 index 再生成と軽量検証（fmt-check・prek・audit_docs・audit_impl_record・index plan in-sync）
- [x] review-protocol によるレビューと blocker 解消

## Task Breakdown

単一タスク（`docs/tasks/issue-419-adr-backreflection.md`）。ADR 逆反映と文書チェーン・index は一体の成果物であり分割しない。

## Open Questions / Gaps

- ADR `status` の正式決裁（`proposed`→`accepted`/`conditional`/`rejected`）— design authority 権限。Issue #419 残 DoD 完了後。
- 仮想優先 Gate（`#630`/`#631`）の証拠深度記載・`#465` Legacy 撤去の独立判断 — Issue #419 残 DoD。
- `#532` DComp `topmost=true` 未解消 — Windows Native Dock 画素の根本 Blocker（open）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 worker・文書 slice）
- Independent work streams: none
- User approval required before dispatch: yes → review-protocol レビューで blocker なしを承認として扱う

## Review Handoff

- ADR 逆反映欄が「未記入」を含まず、outcome = CONDITIONAL・「L1 のみ評価済み、L2/L3 非保証」・標準 Driver 方針維持・Native Dock 無条件採用不可・宣言 Capability・既知制約/将来 Owner・証拠リンクを日付付きで記録しているか
- 「決裁済みの記録」と「推奨/Worker 決定」の分離、`NotRun`/`Fail`/`Unsupported`/`Deferred` の `Pass` 化がないか、`#532` open Blocker が解消済みに見えないか
- status `proposed`・`id`/`created`/既存 `relations` 不変、Decision Outcome・判定フロー・縮退順序・責務マップ・Capability Matrix・Verification の既存記述が不変か
- 変更範囲が対象 ADR + 新規 7 文書 + 生成 index に限定され、`evidence/`・他 ADR・POLICIES・コードに差分がないか
