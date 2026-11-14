# 内部 Plan — Issue #275 Virtual-first Owner/受入整合（Deferred Owner/Trigger 再指向）

- 文書種別: 内部 plan（`obs-virtualfirst-design.md` の実施計画相当）
- 日付: 2026-11-15
- 対象 Revision: `d5aa1b4b`（origin/main HEAD、clean tree）

## 実施手順

1. **Issue 実在照合（実測）**
   - `gh issue view` で RF #627/#628/#629/#630/#631/#281/
     #294/#313/#314 と AL #127・SV #28 の状態を取得し、
     番号・open/closed・役割を evidence に記録する。
   - POLICIES.md §2.1・Issue #275 本文「2026-11-15
     Virtual-first正式受入Owner整理」・#628〜631 の設計正本
     題名を突合し、役割記述の正本を確認する。
2. **内部 doc chain 作成**: `evidence/rf275/
   obs-virtualfirst-{spec,design,plan,task}.md` を作成。
3. **canonical doc 差分**:
   - `docs/designs/issue-275-observability-integration-
     design.md` の front-matter `updated` を 2026-11-15 に
     更新。
   - 「Deferred 範囲の Owner/Trigger」節の実 OS GUI・実
     Device 行の再開 Owner/Trigger を #628〜631 Harness＋
     #627 へ再指向し、表直後に dated note を追加。
   - 「Consumer Gate 割当（要約）」の Windows L2/L3 行を
     整合させ、dated note を追加。
4. **ledger 追記**: `evidence/rf275/obs-acceptance-ledger.md`
   に §7（追記専用）を追加。既存 §1〜§6 は改変しない。
5. **evidence 作成**: `evidence/rf275/obs-virtualfirst-
   evidence.md` に検証コマンド・exit code・照合結果・
   差分要約を記録。
6. **検証**: `mise run fmt-check`、`mise exec -- prek run
   --all-files`、`git diff`/`git status` で変更範囲確認、
   review-protocol レビュー。
7. **統合**: `git fetch origin main && git merge origin/main`
   → commit（Conventional Commits）→ push → `gh pr create`
   （本文に `Refs #275`）→ worker_done 報告。

## ファイルマップ

| ファイル | 操作 |
| --- | --- |
| `docs/designs/issue-275-observability-integration-design.md` | 参照差し替え＋dated note＋`updated` 更新のみ |
| `evidence/rf275/obs-acceptance-ledger.md` | §7 追記のみ（既存節は不変） |
| `evidence/rf275/obs-virtualfirst-{spec,design,plan,task,evidence}.md` | 新規（内部 doc chain＋検証記録） |

## 依存・順序

- 手順 1 の照合結果が手順 3〜5 の記述の正本。照合前に
  Issue 番号・状態を書かない。
- canonical doc 差分と ledger §7 は同一の再指向内容で
  整合させる（ledger は索引、design doc が詳細正本）。

## 検証マトリクス

| 検証 | コマンド | 期待 |
| --- | --- | --- |
| 書式 | `mise run fmt-check` | exit 0 |
| 軽量 hook | `mise exec -- prek run --all-files` | exit 0 |
| 変更範囲 | `git status`/`git diff --stat` | 指定ファイルのみ |
| 追記のみ（ledger） | `git diff evidence/rf275/obs-acceptance-ledger.md` | 追加行のみ |
| 参照実在 | `gh issue view` 出力と evidence 照合 | 全参照が実在・状態一致 |

## Rollback

文書のみの変更。問題時は当該 commit の revert で戻せる。
コード・CI・manifest への影響なし。
