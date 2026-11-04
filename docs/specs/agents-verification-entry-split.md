---
id: "SPEC-R6DgKRt3JJqwgUhZfDMOhr"
type: "spec"
status: "implemented"
title: "AGENTS.md の #350 過剰縮小の是正 — `### 検証入口` 等の参照詳細を Progressive Disclosure で docs/ へ分離"
created: "2026-11-05"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/pull/350"
  changes:
    added:
      - type: "file"
        path: "docs/development-guidelines.md"
    modified:
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/agents-verification-entry-split.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../development-environment.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../plans/agents-verification-entry-split.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/pull/350"
  defers: []
  deferred-by: []
---

# AGENTS.md の #350 過剰縮小の是正 — `### 検証入口` 等の参照詳細を Progressive Disclosure で docs/ へ分離

## What Should Be Built

PR #350（squash merge `2ee375dd6c9adfada68eb0a6706b2a096948e7ea`）は AGENTS.md を 37KB 級から 14KB 級へ縮小し、詳細を `docs/development-environment.md` へ移動したが、縮小が過剰だった。owner 指示の正しい要求は「`### 検証入口` セクション（mise タスク詳細表・CI lane 記述・Consumer 影響分類運用）だけを別文書に分離して参照化し、他の記述は削らない」である。

さらに追補指示: 復元 + `### 検証入口` 分離後でも AGENTS.md が注入閾値（16,384B）を超える場合、内容量を落とさず移動のみで以下も分離し Progressive Disclosure 構成に再編する — (1) `## Rust ワークスペースと暫定開発規約` の暫定規約詳細部分（検証入口以外・常時参照不要な詳細）、(2) アーキテクチャ確定契約（設計ベースライン）関連記述、(3) 共通基盤・Observability・Diagnostics・Debug の設計方針系記述（必要時参照のもの）。分離先は docs/ 配下の適切なファイル（新規可）で、AGENTS.md には各 1 行程度の参照ポインタを残す。目標は AGENTS.md < 16,384B（多少超える程度なら構造上許容）。

本 spec の成果物は以下のとおり。

- AGENTS.md を #350 マージコミットの親コミット（`2ee375d^`）時点の内容へ復元し、`### 検証入口` セクションのみを除去して 1〜2 行の参照を残す
- 除去した `### 検証入口` セクション全文（表を含め省略なし）を `docs/development-environment.md` に収録し、冒頭に「AGENTS.md の `### 検証入口` から分離した詳細リファレンス」である旨を 1 行記す
- 追補対象の詳細部分を `docs/development-guidelines.md`（新規）へ移動し、AGENTS.md の該当 `###` 見出しに各 1 行程度の参照ポインタを残す
- 復元した AGENTS.md のうち現状と事実として矛盾する記述のみを最小修正する（整合性修正。削減・言い換え・要約ではない）

## Why Now

- #350 の縮小は「always-on context 注入の 16,384B 切り捨て対策」を理由に規約本文自体を圧縮したが、owner 裁定で「過剰」と確定しやり直しが指示された。規約文はエージェントの作業契約の正本であり、消化コピーへの差し替えは契約内容の散逸を招く。
- 縮小版が main に既にマージ済みのため、縮小差分を revert するのではなく「旧版復元 + `### 検証入口` 分離」として再適用する必要がある。

## Users and Value

- **エージェント・作業者**: AGENTS.md が完全な規約本文を保持しつつ、常時注入のボリューム最大要因である mise タスク表のみを外部参照化できる。
- **レビュアー**: `docs/development-environment.md` に分離セクション全文が収録され、情報損失なしを検証できる。

## Scope

### In Scope

- `git show 2ee375d^:AGENTS.md` による旧版復元（最新 `origin/main` 上で、ファイルとして復元した上で編集）
- `### 検証入口` セクション（見出しから次の `###` 直前まで）の除去と参照行の残置
- `docs/development-environment.md` へのセクション全文収録（表・箇条書きを含め省略なし）と冒頭の分離由来注記
- `docs/development-environment.md` 側で、復元により AGENTS.md と両方に存在することになる記述の整理（lockfile 内部詳細・MSRV clippy コマンド・開発原則詳細版附録。正本は AGENTS.md）
- 追補分離: `docs/development-guidelines.md`（新規）への以下移動 — 暫定規約詳細部分（MSRV clippy 検証・RT 検証レーンの詳細）、`### アーキテクチャ確定契約（設計ベースライン）` 本文、`### 共通基盤・Observability・Diagnostics・Debug の設計方針（必要時参照）` 本文。AGENTS.md 側は各 `###` 見出しを残し参照ポインタを置く
- 現状と事実として矛盾する記述のみの最小修正（mise `[tools]` の `apm` 未列挙、クレート数 `計33` → 実数、`## Rust ワークスペースと暫定開発規約` 節への対応表ポインタの追随、`crates/README.md` のクレート数記述）

### Out of Scope

- `### 検証入口` および追補列挙項目以外の AGENTS.md 本文の削減・言い換え・要約（暫定規約の常時参照ルール本文・ワークスペース構成・開発環境節等は AGENTS.md に残す）
- 新規規約の追加・既存規約の変更（整合性修正以外）
- `#350` の revert commit（復元はファイル単位で行い squash merge を歴史書き換えしない）
- `docs/development-environment.md` の全面的な再構成（#350 で追加された独自記述は保持）

## Requirements

- 分離先 doc には旧版 `### 検証入口` セクション全文を省略なく収録する（表を含む）
- AGENTS.md の見出し構造は旧版と同一を維持する（`### 検証入口` → 参照行のみ変更）
- 整合性修正は「現状と事実として矛盾」が確認できた箇所のみとし、修正一覧を PR 本文に区別して列挙する
- `docs/development-environment.md` に #350 で追加され他に存在しない有益な記述（Issue #346 選択 CI・mise freshness 等の運用詳細）は保持してよい

## Acceptance Criteria

- [ ] **AC1**: AGENTS.md が旧版の全見出し構造を保持し、`### 検証入口` は参照行に、暫定規約詳細・アーキテクチャ確定契約・共通基盤方針の各本文は参照ポインタに置き換わっている。移動していない全記述は旧版どおり残っている
- [ ] **AC2**: `docs/development-environment.md` に旧 `### 検証入口` セクション全文（mise タスク表・3 つの末尾 bullet を含む）が収録され、冒頭に分離由来の注記がある。AGENTS.md 内に当該表が残っていない
- [ ] **AC3**: 整合性修正が「実際に確認した事実矛盾」のみで構成され、PR 本文に列挙される（`mise.toml` に存在しない列挙ツールの未記載、クレート数、対応表ポインタ、`crates/README.md` のクレート数）
- [ ] **AC4**: `mise tasks` で表内タスク名の実在を確認し、`mise run check`（または環境許容の軽量タスク）が pass する
- [ ] **AC5**: 追補対象の詳細が `docs/development-guidelines.md` に省略なく収録され、AGENTS.md が 16,384B 以下または「多少超える程度」に収まっている。移動した節・移動先・最終バイト数が PR 本文に列挙される

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `docs/development-environment.md` 内の構造化版 CI 記述と分離 verbatim セクションの二重記述 | 両方保持を許容（#350 追加分の保持を owner 指示が許可。正本は分離セクション） | 将来の doc 整理タスク | 同一内容の 2 箇所管理による drift |
| mise タスク表に未収録の opt-in タスク（ofx/plugin-core/icc 系） | 表の verbatim 移動を優先し、欠落分は doc 側補足で補完 | 将来の表更新タスク | 表が全タスク網羅でない点の読み違い |

## Sources and Relations

- 一次ソース: owner 指示（PR #350 フォローアップの是正タスク・対応 Issue なし）と [PR #350](https://github.com/scarecrow173/ruviform/pull/350)（過剰縮小が判明した先行変更）。
- 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（復元・分離対象）、[`docs/development-environment.md`](../development-environment.md)（分離先）。
- 派生（`relations.derived-by`）: [実装 plan](../plans/agents-verification-entry-split.md)。
