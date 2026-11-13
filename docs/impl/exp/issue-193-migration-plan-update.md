---
id: "EXP-193-migration-plan-update"
title: "Issue #193 migration plan update — Issue 状態 census と Snapshot 記述規則"
created: "2026-11-14"
updated: "2026-11-14"
---

# Issue #193 migration plan update — Issue 状態 census と Snapshot 記述規則（2026-11-14）

- 対象タスク: `docs/tasks/issue-193-migration-plan-update-task.md`
- 設計根拠: `docs/designs/issue-193-migration-plan-update-design.md`
- 証拠記録: `evidence/rf193/issue-state-census-2026-11-14.md`
- 実装記録: `docs/impl/ir/issue-193-migration-plan-update.md`

## 1. 目的

版付き Snapshot の Issue 状態記述を `gh` 実測に基づかせ、
Blocked by の誤記（Closed/Deferred/NotRun の着手 Blocker
化）を防ぐための census 手順と記述規則を記録する。

## 2. 試行

1. `gh issue list -R scarecrow173/ruviform --state all
   --limit 1000 --json number,title,state`（SV/AL も同様、
   limit 500）で 3 repo の Issue 状態を取得。
2. 計画関連の Open Issue を抽出し、`gh issue view <N>
   --json number,title,state,body` で `### 着手条件
   （Blocked by）` 節を照合。
3. 照合結果を `evidence/rf193/
   issue-state-census-2026-11-14.md` に記録。
4. 確認した全 Open Issue で `着手条件（Blocked by）` が
   「なし」であることを確認し、実 OS GUI/Device/Plugin
   Gate は `統合・受入で関係する Issue（着手は止めない）`
   側に分離済みであることを検証。

## 3. 結果

- 照合対象の全 Open Issue（RF 25 件超・SV・AL 各件）で
  `着手条件（Blocked by）: なし` を確認。
  SV #23/#28・AL #127 は Stage 3 分離方針を本文に明記。
- M0〜M7 実装 Issue（#184〜#198）・M1 着手 Gate
  （#199/#201）・IO/OBS/GUI Test/GUI lane 基盤・
  3OS OS-L1 CI（#598）は全て CLOSED。
- Script・Creative Branch・Future capability の新規
  Issue 群（RF #315〜#318・#553・#558〜#562、SV
  #119/#124/#130/#138・#259/#264/#266/#271〜#296）は
  Open であり、M0〜M7 の Blocked by ではなく並行 Lane
  として接続する規則を採用。

## 4. 採用した決定

- Snapshot 版識別子は基準日 `Snapshot-2026-11-14`。
- 計画正本 2 文書へ同一内容の Snapshot 節を追記し、
  設計正本側を詳細正本・実行正本側を同内容の実行記録
  として相互参照。
- Blocked by は全 Milestone「なし」、実 OS/実機 Gate は
  Integration Gate/独立 Owner Issue/Deferred-Activation
  表にのみ記述。
- `docs/impl/ir`・`docs/impl/exp` の README は
  doc-maintenance index 対象外のため手動追記。

## 5. 結論

採用。Snapshot 記述の根拠として census 証拠を
`evidence/rf193/issue-state-census-2026-11-14.md` に
固定した。
