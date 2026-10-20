---
id: "TASK-034JfHz5u1uXcPkG8d8t4B"
type: "task"
status: "todo"
title: "PoC 採否 ADR 確定（エビデンス集約・confirm/revise・accepted 遷移）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-28-ui-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/issue-28-ui-poc.md"
    - "poc-render-backends.md"
    - "poc-message-boundary.md"
    - "poc-native-surface.md"
    - "poc-frontend-and-dist.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-28-ui-poc.md"
    - "../designs/issue-28-ui-poc.md"
    - "../adr/ui-tech-selection-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-28-ui-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-28-ui-poc.md"
  references:
    - "https://github.com/tauri-apps/wry/pull/1762"
  defers: []
  deferred-by: []
---

# PoC 採否 ADR 確定（エビデンス集約・confirm/revise・accepted 遷移）

## Work

plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T6。全実験のエビデンスを集約し、[採否 ADR](../adr/ui-tech-selection-poc.md)（`ADR-034Jc9Cguud7nZ2Tt88Ptu`・proposed）の各採否条項を confirm / revise して `accepted` へ遷移させ、ドキュメントグラフの status・記録を同期する（AC7）。

- [ ] `poc/ui/RESULTS.md` を完成させる: 「AC → 実験 → エビデンス行」対応表に全 AC（AC1–AC6）の行を揃え、各行が環境（`env`）・再現コマンド・証跡（`out/*.jsonl` / `docs/impl/exp/*.jsonl`）を持つことを確認
- [ ] 採否 ADR の全条項（WebView 実装 / 高頻度描画 / 合成方式 / メッセージ境界 / フロントエンド FW / 配布 / 採否構造）を、エビデンス行へのリンク付きで confirm または revise する。実機未検証項目（macOS / Linux 実挙動・PR #1762 依存構成）は「文書評価」と明示されていることを監査し、採否はコンポーネント単位・固定フォールバック階層にしない原則を維持する
- [ ] 全条項確定後、ADR status を `proposed` → `accepted` に遷移（遷移は review-protocol で確認）
- [ ] spec の AC チェックボックスを完了し、spec status を `approved` → `implemented` に遷移する（**決定**: 完了済み spec の既存 convention は `implemented` — `SPEC-034J6OCXHiP1izKRzbMmgz` 等と同じ遷移を採用。`completed` ではなく `implemented` を使う）。plan status を `completed`、全 task status を `done` に更新し、各 `changes` 記録を実施結果に同期する
- [ ] 各 task に impl 記録（`docs/impl/ir/`）が紐付いていること・`docs/impl/exp/` の実験ログが全実験分あることを監査する
- [ ] wgpu MSRV 1.87 と宣言 `rust-version = "1.85"` の衝突事実を、MSRV 見直し専用タスクへの入力として連携記録する（本タスクでは引き上げ自体を決めない）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-28-ui-poc.md](../plans/issue-28-ui-poc.md) — `PLAN-034JfDEU8mhCDj4ldYFZUG`（approved）。T6 の正本
- [../adr/ui-tech-selection-poc.md](../adr/ui-tech-selection-poc.md) — `ADR-034Jc9Cguud7nZ2Tt88Ptu`（proposed）。confirm / revise と `accepted` 遷移の対象
- [../specs/issue-28-ui-poc.md](../specs/issue-28-ui-poc.md) — AC7（ADR 確定）・AC8（隔離確認）の正本
- [../test-specs/issue-28-ui-poc.md](../test-specs/issue-28-ui-poc.md) — 全条項 confirm 条件（リンク付きエビデンス・「文書評価」ラベル）の判定規則

## Dependencies

- 前提: `poc-render-backends.md`・`poc-message-boundary.md`・`poc-native-surface.md`・`poc-frontend-and-dist.md`（AC2–AC6 の全エビデンスが揃っていること。T1 はそれらの経由で前提済み）
- 後続をブロック: なし（本タスクが plan の最終フェーズ）
- `wxp` パッチ差分の精査は参照材料としての整理に留める（プラグイン GUI Issue への引き渡し材料。採用判断の差分レビュー自体は別 Issue）

## Affected Files

- `docs/adr/ui-tech-selection-poc.md` — 各条項の confirm/revise・status `accepted`・`changes` 記録
- `poc/ui/RESULTS.md` — 対応表完成・全証跡行の確定
- `docs/specs/issue-28-ui-poc.md` — AC チェックボックス・status `implemented`・`changes` 記録
- `docs/plans/issue-28-ui-poc.md` — status `completed`・`changes` 記録
- `docs/tasks/poc-*.md` — status `done`・`changes`・`implemented-by` → impl 記録の紐付け
- `docs/impl/ir/`・`docs/impl/exp/` — impl 記録・実験ログの監査

## Done When

- [ ] 採否 ADR の全条項がリンク付きエビデンスで confirm/revise され、status=`accepted`・「文書評価」ラベル監査済み
- [ ] `RESULTS.md` の AC → 実験 → エビデンス対応表が完結し、全行に環境・再現コマンドがある
- [ ] spec の AC1–AC8 がチェック済みで status=`implemented`、plan=`completed`、全 task=`done` に同期されている
- [ ] 全 task に `docs/impl/ir/` 記録が紐付き、`docs/impl/exp/` ログが揃っている
- [ ] wgpu MSRV 衝突の連携記録がある

## Verification

- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type adr --json` — ADR の status・relation・index が整合し findings なし
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — 全ドキュメント種の audit が通る（status 同期後）
- [ ] `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` — 出力が空（AC8 最終確認）
- [ ] `mise run check` && `mise run lint` && `mise run test` — ルート検証が PoC 前と同一に成功
- [ ] Manual: `RESULTS.md` 対応表の全行に再現コマンドがあり、文書評価ラベルが必要な行すべてに付いている（補助確認）
