---
id: "TSPEC-034WvK9mNqR2sTxYcAbEfG"
type: "test-spec"
status: "draft"
title: "consumer-impact-check workflow の PR 分類機械検査（記入検査・unknown 解消先・draft 除外）"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md"
    - "docs/test-specs/consumer-pr-issue-template.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# consumer-impact-check workflow の PR 分類機械検査（記入検査・unknown 解消先・draft 除外）

## Purpose

`.github/workflows/consumer-impact-check.yml` が、PR 本文の Consumer 影響分類欄（`PULL_REQUEST_TEMPLATE.md` の `## Consumer 影響分類` 表）の記入を機械的に強制し、「分類欄の未記入が human gate 頼みで見落とされる」状態を防ぐことを保証する。検査するのは記入の存在のみであり、分類内容の正しさは機械判定しない（レビュアー human gate の責務を lane が代替しない）。

## Feature

Ruviform PR の Consumer 影響分類の機械的記入検査（情報 lane・非 required）。

## Rules

- `pull_request`（`opened`/`edited`/`synchronize`/`reopened`/`ready_for_review`・main 向け）で起動し、draft PR（`github.event.pull_request.draft == false` の job `if`）は対象外とする
- `## Consumer 影響分類` セクションが PR 本文に存在しなければ fail
- `| Synvelis |` と `| AudioLens |` の各行の分類セルが `impacted` / `no impact` / `unknown` のいずれか（大小文字・空白・太字 `**` は許容）に記入されている。未記入・語彙外は fail
- `unknown` が 1 行でもあれば `### unknown の解消先` に記述が必要。HTML コメント（複数行含む）と空白のみは未記入とみなす
- PR 本文は `env` 経由でスクリプトへ渡す（`run:` 内への式展開はしない — 本文中の文字による式注入を防ぐ）
- 本 lane は required checks に含めず、`unknown` の merge 許否はレビュアー判断のまま（spec R1/R9）

## Examples

- Given PR 本文が未記入テンプレートのまま、When check、Then 両 Consumer 行の未記入を指摘して job fail
- Given `Synvelis=no impact`・`AudioLens=impacted`+リンク、When check、Then pass
- Given `Synvelis=unknown` かつ解消先が HTML コメントのみ、When check、Then 解消先未記入で job fail
- Given `Synvelis=unknown` かつ解消先に追跡 Issue 記述、When check、Then pass
- Given `## Consumer 影響分類` セクション自体が無い、When check、Then セクション欠落で job fail

## Guarantees

- 分類未記入・語彙外・解消先なしの `unknown` が緑の check として報告されない
- draft PR に未完成分類の fail が付かない
- lane は記入の存在のみ検査し、分類の妥当性（根拠・リンクの内容）は判定しない
- PR 本文由来の任意文字列が shell 式として評価されない

## Non-goals

- 分類内容（impacted/no impact の選択）の正しさの検証 — レビュアー責務
- `## 検証証拠` 表の内容検証 — 機械判定に耐える記述形式でないため記入検査の対象外
- required check 化・分類に応じた label 付与等の自動化 — spec R9/Deferred の別判断

## Risk

- これを外すと未記入 PR が「reviewer の見落とし」経路で merge され、Issue ルール2 の分類記録が欠落する。逆に過剰な検査（検証証拠表の内容強制・label 連鎖）は記入コストだけを増やして式注入や誤判定の面を広げる
