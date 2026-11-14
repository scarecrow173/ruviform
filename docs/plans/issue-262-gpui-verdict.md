---
id: "PLAN-034Xzw4vyjdTI0ZYgqUJr1"
type: "plan"
status: "completed"
title: "Issue #262 GPUI採用Gate verdict 文書・証拠スライス実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-262-gpui-verdict.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-262-gpui-verdict.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-262-gpui-verdict.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-262-gpui-verdict.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-262-gpui-verdict.md"
  references:
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
    - "evidence/rf262-gpui-verdict/verification.md"
  defers: []
  deferred-by: []
---

# Issue #262 GPUI採用Gate verdict 文書・証拠スライス実装計画 Implementation Plan

## Goal

Issue #262 の採用 Gate 評価スライスとして、A〜E Gate の verdict matrix・証拠 report・#419 引き渡し文面・検証記録を `evidence/rf262-gpui-verdict/` に作成し、spec/design/test-spec/plan/task/IR/exp の文書チェーンを完成させて検証を通す。

## Scope

**1 実装ストリーム**: 文書・証拠のみ。`crates/**`・`Cargo.toml`/`Cargo.lock`/`mise.toml`/CI workflow・Consumer コードは対象外。実装コード・新規 CI・ADR status 変更・Issue 投稿は生成しない。

## Upstream Documents

- `docs/specs/issue-262-gpui-verdict.md`
- `docs/designs/issue-262-gpui-verdict.md`
- `docs/test-specs/issue-262-gpui-verdict.md`

## Design Inputs

- `docs/designs/issue-262-gpui-verdict.md`（approved）

## File Map

| ファイル | 種別 | 内容 |
| --- | --- | --- |
| `docs/specs/issue-262-gpui-verdict.md` | spec | 問題・Audience・Scope・Non-goals・受入条件・Evidence |
| `docs/designs/issue-262-gpui-verdict.md` | design | 判定方法論（D1）・Gate A〜E 評価構造（D2〜D6）・推奨 verdict と引き渡し境界（D7） |
| `docs/test-specs/issue-262-gpui-verdict.md` | test-spec | verdict 記録の Rule/Guarantee と反例 Example |
| `docs/plans/issue-262-gpui-verdict.md` | plan | 本計画 |
| `docs/tasks/issue-262-gpui-verdict.md` | task | 文書チェーン完成・検証タスク |
| `docs/impl/ir/issue-262-gpui-verdict.md` | impl/ir | 実装記録 |
| `docs/impl/exp/issue-262-gpui-verdict.md` | impl/exp | 証拠収集・判定手法の試行記録 |
| `evidence/rf262-gpui-verdict/verdict-matrix.md` | evidence | A〜E Gate verdict matrix 正本（CaseID/OS/driver/plugin/expected/observed/verdict/証拠 URL） |
| `evidence/rf262-gpui-verdict/report.md` | evidence | 証拠収集方法・SHA/Run 照合・判定まとめ・既知ギャップ |
| `evidence/rf262-gpui-verdict/handoff-419.md` | evidence | #419/#465 向け引き渡し文面 |
| `evidence/rf262-gpui-verdict/verification.md` | evidence | 検証実行記録（コマンド・OS・toolchain・exit code・結果） |

## Dependency Graph

`spec → design → test-spec → plan → task → ir/exp`。design は spec を implements、plan は spec を implements かつ design に depends-on、task は plan を implements、ir は task を implements。test-spec は spec/design を verifies。

## Risk Register

| リスク | 影響 | 対応 |
| --- | --- | --- |
| ancestor SHA の CI 証拠を最新 SHA の Pass と誤認 | 統合受入の偽装 | design D1.2 の SHA 照合規則。HEAD run 未実施は NotRun |
| 機構層 Pass を Dock Pass に昇格 | Gate D 過大評価 | verdict-matrix で機構/画素を別 Case に分離。各 report の「不成立」明記を転記 |
| `Unsupported`/`NotRun` 混同 | Capability 欠落の見落とし | design D1.3 の語彙規則を test-spec R-VERDICT で固定 |
| Consumer Oracle 不在の Pass 化 | Gate E 虚偽 | consumer-verify 対象 SHA run なしを明記 |
| 証拠の取捨選択バイアス | 判定の恣意性 | verdict-matrix に CaseID/expected/observed/evidence を網羅記録し、Fail/NotRun も同列に残す |

## Rollback Strategy

文書・証拠のみの変更であり、`git revert` で文書チェーン・evidence・index を戻せる。コード・依存・CI への影響はない。

## Verification Matrix

| 検証 | 方法 | 結果 |
| --- | --- | --- |
| 文書 front matter・relation・構造 | `audit_docs`/`doc_status` | 自スコープ error なし |
| impl 記録・実験ログ | `audit_impl_record.js`/`audit_experiment_log.js --json` | error なし |
| 管理 index 整合 | `doc-maintenance index apply` + `sync_impl_index.js` | 触れた dir で `in-sync` |
| Rust fmt | `mise run fmt-check` | 文書のみだが正規検証として実行 |
| Markdown/リンク軽量チェック | `mise exec -- prek run --files <変更ファイル>` | Pass |
| 証拠再実走（同一 SHA 補強） | `mise run test-ui-scenario-contract` `check-ui-driver-boundary` `test-ui-native-slot` `test-ui-parity` `test-ui-gpui` | HEAD `7aab6302`・Windows で全 Pass（`evidence/rf262-gpui-verdict/verification.md`） |
| Runtime/Consumer/OS 実機の新規検証 | — | **NotRun**（文書・証拠 slice。既存証拠の照合と正規 lane 再実走のみ） |

## Implementation Sequence

- [x] spec/design/test-spec/plan/task の文書チェーン作成
- [x] CI run・先行 evidence・宣言的 Capability の証拠収集と SHA 照合
- [x] `evidence/rf262-gpui-verdict/` の verdict-matrix/report/handoff/verification 作成
- [x] impl/ir・impl/exp 作成と audit
- [x] 管理 index 再生成と軽量検証（fmt-check・prek・audit_docs・audit_impl_record）
- [x] review-protocol によるレビューと blocker 解消

## Task Breakdown

単一タスク（`docs/tasks/issue-262-gpui-verdict.md`）。文書チェーンと証拠は一体の成果物であり分割しない。

## Open Questions / Gaps

- HEAD `7aab6302` の CI run が未実施 — 統合受入は merge 後の対象 SHA CI 証拠待ち（Coordinator 確認事項）。
- verdict の最終決裁（accepted/conditional/rejected）— Coordinator 権限。本 slice は推奨 verdict と根拠表のみ。
- Windows L2 の「利用機能」確定範囲 — Consumer 側の GUI 機能確定待ち。現時点では Window/Input/Focus の実測済み能力と IME Unsupported・a11y/DPI/物理入力 NotRun を能力別に記録。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 worker・文書 slice）
- Independent work streams: none
- User approval required before dispatch: yes → review-protocol レビューで blocker なしを承認として扱う

## Review Handoff

- verdict-matrix の Case 網羅性（A〜E × OS × capability）と Fail/NotRun/Unsupported の正直さを重点確認
- 「推奨 verdict」と「決裁」の分離が全ファイルで一貫しているか
- #419/#465 引き渡し文面が「L1 のみ評価済み、L2/L3 非保証」と独立判断境界を正確に伝えているか
