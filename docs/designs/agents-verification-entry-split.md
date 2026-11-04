---
id: "DESIGN-L8xeY9I5pmnb61GPOcn3D0"
type: "design"
status: "approved"
title: "AGENTS.md 旧版復元 + `### 検証入口` の docs/development-environment.md 分離方式"
created: "2026-11-05"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/pull/350"
  changes:
    added: []
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
  implements:
    - "../specs/agents-verification-entry-split.md"
  implemented-by:
    - "../plans/agents-verification-entry-split.md"
  depends-on:
    - "../specs/agents-verification-entry-split.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../development-environment.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/agents-verification-entry-split.md"
  derived-by:
    - "../plans/agents-verification-entry-split.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AGENTS.md 旧版復元 + `### 検証入口` の docs/development-environment.md 分離方式

## 目的と前提

spec `SPEC-R6DgKRt3JJqwgUhZfDMOhr`（PR #350 フォローアップ・owner 是正指示）を実現するための編集方式を定める。前提: `origin/main` には #350 squash merge（`2ee375d`）後の縮小版 AGENTS.md（約 14KB）と、#350 で新設された `docs/development-environment.md`（約 19KB）が存在する。

## 決定事項

### D1: AGENTS.md の復元ベースと編集単位

- 作業ベースは `git show 2ee375d^:AGENTS.md` の出力（旧版・全文）。`origin/main` 上の縮小版を手で再拡張するのではなく、旧版を正として置き換える。
- 復元後の唯一の構造的変更: `### 検証入口` セクション（旧版で `### ワークスペース構成` と `### 暫定規約` の間。見出し行から次の `###` 直前まで。先頭の導入 bullet・mise タスク対応表・`*-ci` 注記・ci.yml 記述 bullet・Consumer 影響分類 bullet を含む）を除去し、見出し `### 検証入口` の直下に 1〜2 行の参照を残す。参照には「cargo を直接実行せず `mise run <task>` を使う」の核心規約と `docs/development-environment.md` へのリンクを含める。
- 他のセクション（開発・設計原則全文を含む）は一切削除・言い換え・要約しない。

### D2: `docs/development-environment.md` の編集方式

- 冒頭に「この文書は AGENTS.md の `### 検証入口` から分離した詳細リファレンス」である旨 1 行を記す（既存の正本優先注記は保持）。
- `## 検証入口`（`##` レベルに昇格。内容は旧版セクションを verbatim 収録 — 導入 bullet・表全行・`*-ci` 注記・ci.yml 記述・Consumer 影響分類の 3 末尾 bullet）を文書前半に配置する。
- #350 で追加され他に存在しない有益な記述は保持する: 「mise タスク体系」（freshness・lazy・`test-ofx-msrv`・webview Linux 依存などの運用詳細）、「CI ワークフロー構成」（構造化版・Issue #346 選択 CI・macOS 分離の整理記述）、「Consumer 影響分類の運用」（`.github/PULL_REQUEST_TEMPLATE.md` が記入形式の正本である旨を含む）。
- 復元により AGENTS.md と両方に存在することになる記述は doc 側から除去する（正本は AGENTS.md）: 「lockfile・環境内部の詳細」「MSRV clippy の既知ノイズ除外」「附録: 開発・設計原則（詳細版）」。
- 「mise タスク体系」の `test-ofx-msrv` 記述を、同じく表に未収録の `test-plugin-core`/`lint-plugin-core`/`test-ofx`/`test-ofx-bridge`/`lint-ofx`/`test-video-proc-icc`/`lint-video-proc-icc` を含む opt-in タスク群の補足に拡張し、表未収録分が `mise tasks` に実在することを読者が確認できるようにする。

### D3: 整合性修正（削減ではなく事実矛盾のみ・実測確認済み）

復元した旧版全文を現行基盤ファイルと照合し、矛盾が確認できた箇所のみを最小修正する。

| # | 箇所 | 旧版記述 | 現状 | 修正 |
| --- | --- | --- | --- | --- |
| F1 | `## 開発環境（mise）` ツール列挙 | `uv / prek / git-lfs / pipx 系の共通基盤ツール` | `mise.toml` `[tools]` に `"github:microsoft/apm" = "0.33.0"` が存在（親コミット時点でも存在） | 列挙に `apm` を追加 |
| F2 | 同上・対応表ポインタ | `対応表は ## Rust ワークスペースと暫定開発規約 節を参照` | 対応表は本変更で `docs/development-environment.md` へ移動 | ポインタを当該 doc へ変更 |
| F3 | `### ワークスペース構成` クレート数 | `層別の計33クレート一覧` | `Cargo.toml` `members` = 34、`crates/README.md` の索引表 = 34 行（ヘッダ記述 `計33クレート` 自体が stale） | `計34クレート` に修正（`crates/README.md` ヘッダも 33→34 に修正） |

矛盾なしと確認した候補: 「Shadow Mode」表現（旧版に非含有・`Issue #346 で選択実行を有効化した` と既に現行表現）、`mise.lock`/`.mise/locks/` sidecar 記述（`.gitignore` の `!.mise/locks/` 例外と一致・`pipx:serena-agent` は `lazy = true` のため現行 sidecar 非生成と一致）、タスク表の全タスク名（`mise tasks` で全件実在を確認）、CI lane・required checks 記述（`ci.yml` の job 構成・`ci-gate` needs・9 context ruleset 移行中記述と一致）。

### D4: コミット分割

- 文書チェーン（本 spec/design/plan/task）と AGENTS.md 復元+分離は論理単位で分割したコミットとする。`crates/README.md` の数値修正は F3 の整合性修正の一部として AGENTS.md 修正と同一論理単位で扱う。

## 代替案

- **revert commit で #350 を戻してから分離**: 2 段階の中間状態を履歴に残すだけで結果は同一。ファイル単位復元の方が単純（KISS）。
- **縮小版 AGENTS.md を起点に旧版の差分を再適用**: 復元漏れのリスクがある。旧版を正として置き換える方が検証可能。
- **doc 側の構造化 CI 記述を削除して verbatim のみ残す**: spec が #350 追加記述の保持を許可しており、構造化版は可読性が高い。二重記述の drift リスクは Deferred Design Concerns として記録済み。

## 検証方式

- `wc -c` による AGENTS.md の期待サイズ確認（旧版から `### 検証入口` 分のみ減・大幅減なし）
- `### 検証入口` の表が doc 側に全文あること・AGENTS.md 内に残っていないことの grep 確認
- `mise tasks` による表内タスク実在確認、`mise run check` の pass
