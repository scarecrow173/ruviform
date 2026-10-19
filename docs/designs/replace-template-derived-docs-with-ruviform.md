---
id: "DESIGN-034IvBNBZvoikAgd6DJxdD"
type: "design"
status: "approved"
title: "テンプレート由来ドキュメント・識別子のRuviform固有化の設計"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added: []
    modified:
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "apm.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".pre-commit-config.yaml"
      - type: "file"
        path: ".gitignore"
      - type: "file"
        path: ".gitattributes"
      - type: "file"
        path: "docs/adr/replace-template-derived-docs-metadata-with-ruviform.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "apm.lock.yaml"
  implements:
    - "../specs/replace-template-derived-docs-with-ruviform.md"
  implemented-by: []
  depends-on:
    - "dev-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../README.md"
    - "../../AGENTS.md"
    - "../../apm.yml"
    - "../../mise.toml"
    - "../../.pre-commit-config.yaml"
    - "../../.gitignore"
    - "../../.gitattributes"
    - "../../Cargo.toml"
    - "dev-foundation.md"
    - "overview.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/replace-template-derived-docs-with-ruviform.md"
    - "../adr/replace-template-derived-docs-metadata-with-ruviform.md"
  derived-by:
    - "overview.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/2"
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "../impl/ir/verify-reproducible-setup-and-work-memo.md"
  defers: []
  deferred-by: []
---

# テンプレート由来ドキュメント・識別子のRuviform固有化の設計

## Context

GitHub Issue #3（`relations.source`）と承認済み spec `SPEC-034Ivc5nj1AE6P2PcZU6Ki`、accepted ADR `ADR-034Ivm746Q38Wa21HeqUiI` に基づき、リポジトリ初期化時に引き継いだ汎用プロジェクトテンプレート由来の説明・識別子・作業規約を Ruviform の現状に合わせて置き換える際の、**各ファイルの具体的な文書構成・差分境界**を定義する。コードアーキテクチャではなくドキュメント設計であり、ADR で決定済みの方針（README/AGENTS の改稿定位、「構想・検討中・未決定」の明示付き記載、`name: ruviform` 化、ライセンス未決定の明示、拡張子 LFS ルール維持＋非メディアフィクスチャ方針）をファイルごとの作業単位へ落とし込む。

前提となる事実関係（変更前診断の結果）:

- `README.md` は `# Project Template` +「Use this template」手順を持つ（L1–L25）。管理対象テーブル（L27–L39）の各ファイル説明は汎用だが Ruviform の構成説明ではない。
- `AGENTS.md` は冒頭（L3–L5）が「言語・フレームワークに依存しない開発基盤のテンプレート」「派生リポジトリ」前提。`リポジトリの作業契約` L75 と `開発環境（mise）` L80・L91 に派生前提条項が残る。`開発・設計原則`（L7–L66）と `Gitと安全性`（L104–L110）は汎用性があり Ruviform にも適合する。
- `apm.yml` L1 `name: project-template`、L3 `description: Minimal language-agnostic development foundation`。
- `mise.toml` L8 `# Infrastructure tools only; add a project language toolchain as needed.` がテンプレート前提のまま（実際には `rust = "1.99.0"` 追加済み）。各 task description は実態に合っている。
- `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` にプレースホルダー的記述はなく、現行設定は Ruviform に適合する（レビューで確認・原則無変更）。
- `Cargo.toml` の `license = "MIT"` / `edition` / `rust-version` は仮置きコメント付きであり、本 Issue では変更しない。
- #2 の実機検証済み手順・環境・未検証項目の一次記録は IR `verify-reproducible-setup-and-work-memo.md`「作業メモ（#3 転記用）」（L195–268）である。本設計の README/AGENTS 文案に使う事実はすべてこの一次記録に限定する。

## Scope

### In Scope

- `README.md` の新構成（タイトル・概要・Status・Quickstart・リポジトリ構成・導線・Contributing・License）の設計
- `AGENTS.md` のセクション単位の再編成設計（保持・改稿・追加の切り分け）
- `apm.yml` の `name`/`description` 差分と `apm.lock.yaml` 再生成手順の設計
- `mise.toml` のコメント・description 適合範囲の設計
- `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` / `Cargo.toml` のレビュー観点と「無変更」を明示する境界
- 表現ルール（検証済み事実 / 構想 / 検討中 / 未確定 / 未検証 / 仮置き）と禁止記載の定義
- 検証設計（構文検証・lockfile 整合・プレースホルダースイープ・文書監査）

### Out of Scope

- README/AGENTS の最終的な日本語文案の逐語確定（実装タスクで本設計の構成・用語規則に従って執筆する）
- `ruviform-audio` 等のコード・クレート作成、`LICENSE` 作成、`Cargo.toml` の `license` 確定（#1 以降）
- mise/APM/prek/LFS の仕組み自体の再構成、タスク定義本体・CI ワークフローの変更
- `docs/` 配下の既存設計文書の改稿（`overview.md` の一覧追記と索引再生成、および ADR `replace-template-derived-docs-metadata-with-ruviform.md` の `relations.derived-by` front matter への本設計バックリンク追記は除く）
- プラットフォーム・公開 API・フォーマット・RT 契約の決定（#1 の詳細設計）

## Components and Boundaries

### ファイルマップ（変更種別とタスク案）

| ファイル | 変更種別 | 内容の要約 | 担当タスク案 |
| --- | --- | --- | --- |
| `README.md` | rewrite | 全文を Ruviform 固有構成へ再作成（後述のセクション構成） | T-README |
| `AGENTS.md` | section-edit | 冒頭・作業契約・mise/APM/Git 節を改稿、`開発・設計原則` は保持、暫定規約節を追加 | T-AGENTS |
| `apm.yml` | field-edit | `name`/`description` のみ変更、その他フィールドは無変更 | T-IDENT |
| `apm.lock.yaml` | generated（再生成） | `apm install` → `apm lock` で再解決し整合確認（手編集しない） | T-IDENT |
| `mise.toml` | comment-edit | L8 のテンプレート前提コメントのみ現状適合化。task description・定義本体は原則無変更 | T-IDENT |
| `.pre-commit-config.yaml` | review / 無変更見込み | L1 コメントと言語非依存フック群は実態適合を確認 | T-CONFIG |
| `.gitignore` | review / 無変更見込み | `target/`・APM 生成物・秘密情報の除外は適合済み | T-CONFIG |
| `.gitattributes` | review / 無変更見込み | 拡張子 LFS ルール維持（ADR 決定）。フィクスチャ方針は AGENTS/README に文書化し属性ファイル自体は変えない | T-CONFIG |
| `Cargo.toml` | no-change | `license = "MIT"` は仮置きのまま。README/AGENTS 側に注記するのみ | — |
| `mise.lock` / `.mise/locks/` | no-change 見込み | `mise.toml` はコメントのみの変更でロック結果に影響しない想定。検証で確認 | T-VERIFY |
| `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md` | metadata-edit | `relations.derived-by` に本設計へのバックリンクを front matter に追加（本文・`status: "accepted"` は無変更） | T-VERIFY |
| その他追跡ファイル | sweep | `git ls-files` ベースのプレースホルダー洗い出し。`docs/` 配下の履歴記録（spec/ADR/design/plan/task/IR/test-spec 等の承認済み・記録済み文書）はプレースホルダー書き換え対象から全て除外（上記の宣言済み `overview.md` 索引・ADR `derived-by` バックリンクのメタデータ更新は別枠） | T-VERIFY |

### `README.md` の新構成

全面改稿。「Use this template」系の記述は残さない。セクション案と各節の意図:

| セクション | 内容の意図 |
| --- | --- |
| `# Ruviform` + 概要段落 | 「Rust を中心とした再利用可能なマルチメディア基盤」という**現時点の方向性**を 1–2 段落で述べる。音声・動画とその下位の計算基盤（DSP・解析・codec・container・SIMD）を同じ基盤上で再利用可能にする意図に触れ、詳細設計は Issue #1 の後続タスクである旨を案内する。概要段落内に対象読者（リポジトリ訪問者・将来の利用者、コントリビューター、AI エージェント、#1 の設計者）を一文で示す枠を確保する。 |
| `## Status`（開発状況） | 開発ステータス = **設計・初期セットアップ段階**を明示し、実装済み/構想/検討中/未確定の境界を表または箇条書きで示す。<br>・実在・検証済み: Rust 1.99.0 ツールチェーン（mise pin + `mise.lock`）、最小ワークスペース（ルート `Cargo.toml` + `crates/ruviform-core` の smoke test）、mise 検証タスク群、`workflow_dispatch` 専用 CI（静的検証のみ済み・実ディスパッチ未実施）。<br>・構想（未実装）: `ruviform-audio` / `ruviform-video` と下位の DSP・解析・codec・container・SIMD。<br>・検討中: `wgpu`、WebView UI、ネイティブ UI、プラグイン連携。<br>・未確定: 対応プラットフォーム、公開 API、サポート形式、ライセンス、edition/MSRV、最終クレート分割。 |
| `## Quickstart` | このリポジトリ向けの再現手順。#2 実機検証済みコマンドのみを記載: `mise install --locked`（正本・固定復元。global mise config が repo 外ツールを要求する環境ではその分のみ exit 1 となり得る caveat 注記）、`mise run setup`（= `apm install` + `prek install` + `git lfs install --local`）、`mise run fmt-check` / `check` / `lint` / `test` / `build`、`mise run verify-ci`（CI 同等・`target/ci/` 並存の注意）、厳格検証として `mise exec -- apm install --frozen`。前提条件（mise 導入済み・`mise trust`・ネットワーク到達先・git 認証・書き込み権限［mise による `~/.local`/`AppData` への書き込み、repo 内 `.git/hooks` 更新（prek）・`.git/config` 更新（git lfs --local）］）と、**実機検証は Windows 11 x64（Git Bash）のみ・macOS/Linux は未検証**の明記。 |
| `## Repository layout` | 実在する構成のみのツリー/表: `Cargo.toml` / `Cargo.lock` / `crates/ruviform-core/` / `mise.toml` / `mise.lock` / `.mise/locks/` / `apm.yml` / `apm.lock.yaml` / `.pre-commit-config.yaml` / `.gitattributes` / `.gitignore` / `.github/workflows/ci.yml` / `docs/` / `AGENTS.md`。存在しないディレクトリ・文書を案内しない。本節末尾に LFS フィクスチャ方針の注記枠を確保する（`.gitattributes` の音声/動画拡張子ルール維持、小さなテストフィクスチャは非メディア形式で Git に置く、CI は必要になるまで LFS オブジェクトを取得しない）。 |
| `## Documentation`（導線） | `docs/designs/overview.md`・`docs/designs/dev-foundation.md`・`docs/specs/`・`docs/adr/` へのリンクと、Issue #1（詳細設計）・#2（開発基盤・完了）・#3（本 Issue）への導線。 |
| `## Contributing` | `AGENTS.md` の作業契約への導線、Conventional Commits・Issue 駆動の流れ、コミットフック（prek）の存在。大げさなコントリビューション規約は作らない。 |
| `## License` | **ライセンス未決定**を明示。`LICENSE` ファイルは存在せず、`Cargo.toml` の `license = "MIT"` は仮置き（配布方針の確定ではない）と注記する。 |

記述上の切り分け: `Status` と `Quickstart` は検証済み事実を書く領域、概要段落の方向性と構想/検討中リストは**意図**を書く領域。両者を同一セクション内で混在させず、Status 内でラベル分けする。

### `AGENTS.md` の再編成

| 現行箇所 | 扱い | 内容 |
| --- | --- | --- |
| L1–L5 冒頭（テンプレート説明・派生リポジトリへの言及） | rewrite | 「このリポジトリは Ruviform — Rust を中心とした再利用可能なマルチメディア基盤（設計・初期セットアップ段階）である」旨に置き換え、以下がエージェント共通の作業契約であることを述べる。派生プロジェクトへの言及は消す。 |
| `## 開発・設計原則`（L7–L66、7 サブセクション） | keep（verbatim またはほぼ verbatim） | 変更前の診断・境界での修正・整合的変更・構造的問題の兆候・実装原則・検証の区別・スコープ比例性は Ruviform にも通用する汎用原則であり保持する。 |
| `## リポジトリの作業契約`（L68–L75） | section-edit | L70–L74 の箇条は保持。L75「このテンプレートに言語・フレームワーク固有の構成や、不要な `.github/`・`docs/` ディレクトリを追加しない」は削除。spec 要求の明文化として「実装されていない API・クレート・検証を実在/実行済みと報告しない」を追加する。 |
| `## 開発環境（mise）`（L77–L93） | section-edit | L80（テンプレートには共通基盤ツールのみ…）を現状に書き換え: `mise.toml` は共通基盤ツール + `rust = "1.99.0"` を管理し、検証タスクも同ファイルが正本。初回セットアップのコマンド列を #2 検証済み手順に同期（`mise install --locked` が正本、`mise install` はフォールバック、`mise run setup`、厳格検証 `mise exec -- apm install --frozen`）。L91「派生リポジトリのmiseタスク」→「`mise.toml` のタスク」に修正。lockfile 再生成条件（`mise.lock`/`apm.lock.yaml`/`Cargo.lock` の各トリガと復旧手順）を IR の表に基づき追記する。 |
| `## AIエージェント環境（APM）`（L95–L102） | keep（微修正） | 運用規約は実態どおり保持。`apm.yml` の `name` が `ruviform` になること以外に変更理由はない。 |
| `## Gitと安全性`（L104–L110） | keep + 1 件追記 | 現行箇条を保持しつつ、LFS フィクスチャ方針（小さなテストフィクスチャは非メディア形式で Git に置き、CI は必要になるまで LFS オブジェクトを取得しない）を 1 箇条追加して ADR 決定を文書化する。 |
| 新規節 `## Rust ワークスペースと暫定開発規約`（仮題） | add | ・構成: ルート `Cargo.toml`（`[workspace]`・`workspace.package`・`workspace.lints`・`[profile.dev]`/`[profile.ci]`）+ 最小メンバー `crates/ruviform-core`（仮置き）。最終分割・公開 API は #1。<br>・検証入口: #2 の mise タスク対応表（`fmt`/`fmt-check`/`check`/`lint`/`test`/`build`、`*-ci` 群、`verify-ci`、`setup`）と「cargo コマンドは `mise run` 経由で呼ぶ」規約。`*-ci` は CI 用途限定。<br>・暫定規約（IR「暫定開発規約」転記）: `unsafe` 原則禁止 + 局所許可と SAFETY コメント、default feature 最小、依存追加は理由記録、`workspace.lints` が正本で `-D warnings` ゲート、公開 API の例は doctest。加えて spec Req 2 の暫定原則として、クレート依存方向（下位基盤クレートは上位クレートに依存しない。詳細な依存 DAG は #1 で確定）と SIMD の前提・不変条件（詳細契約は #1 で確定）を暫定原則として示す。<br>・ライセンス注記: `LICENSE` 不在・ライセンス未決定の明示と、`Cargo.toml` の `license = "MIT"` が仮置きである旨の注記枠を本節に設ける（README `## License` と整合させる）。<br>・RT/オフラインの**設計意図**: リアルタイム音声経路ではブロッキング操作・意図しない allocation を避ける方向で設計する、を暫定原則として記載。API ごとの保証は #1 で確定し、未確定 API を RT-safe と断定しないことを明記。 |

### `apm.yml` の差分設計

変更するのは 2 フィールドのみ:

```yaml
name: ruviform                                   # was: project-template
description: Reusable multimedia foundation centered on Rust   # was: Minimal language-agnostic development foundation
```

- `version` / `targets`（copilot/codex/claude/cursor/windsurf）/ `dependencies.apm` / `devDependencies.mcp`（serena・mise）/ `includes` / `scripts` は ADR 決定どおり**無変更**。
- `name` 変更は APM 配布生成物へ波及するため、同一タスク内で `apm install` → `apm lock` を実行して `apm.lock.yaml` を再生成し、生成物（`apm_modules/`・各エージェント配布物は非追跡）との整合を確認する。厳格確認は `mise exec -- apm install --frozen`。
- APM 0.33.0 に専用 validate コマンドはないため、命名制約の適合は install/lock 成功で確認する（ADR の確認方法をそのまま採用）。

### `mise.toml` の差分設計

- L8 コメント `# Infrastructure tools only; add a project language toolchain as needed.` を、Rust ツールチェーン追加済みの現状に合わせて書き換える（例: `# Shared infrastructure tools plus this repository's pinned Rust toolchain.`）。コメントは英語のまま維持する。
- 各 task description・`run`/`env` 定義・`[settings]`・`[env]`・`[tools]` のバージョン指定は現状適合済みのため**無変更**。レビューで食い違いが見つかった場合のみ description を修正する。

### レビュー対象ファイルの判定基準

- `.pre-commit-config.yaml`: L1「Language-agnostic checks」という定位は設計どおり（軽量・言語非依存のフックのみ）。フック構成は Ruviform に適合 → 無変更を既定とし、コメントの文言が「テンプレート前提」と読める場合のみ微修正。
- `.gitignore`: `target/`・APM/Serena 生成物・秘密情報の除外は適合済み → 無変更見込み。
- `.gitattributes`: 音声・動画・大型ソースの拡張子 LFS ルールを維持（ADR 決定）。フィクスチャ方針は AGENTS.md「Gitと安全性」の追記箇条と README「Repository layout」節の注記枠に文書化し、属性行は変更しない。
- `Cargo.toml`: `license`/`edition`/`rust-version` の仮置きコメントは #1 まで保持 → 無変更。

## Data and Control Flow

ドキュメント更新の作業フロー（実装タスクでの手順設計）:

```text
1. sweep     : git ls-files を母集団に project-template / Project Template /
               派生リポジトリ前提語の所在を確定（docs/ 配下の履歴記録は全て除外）
2. edit      : README 全面改稿 → AGENTS 節単位改稿 → apm.yml 2 フィールド
               → mise.toml コメント → レビュー対象ファイルの判定・修正
3. regen     : apm install → apm lock（apm.lock.yaml 再生成・整合確認）
4. validate  : prek 全ファイル実行（YAML/TOML 構文・フック）→
               mise exec -- apm install --frozen → mise install --locked →
               mise run fmt-check / verify-ci のスモーク →
               プレースホルダー再 grep → 文書表現ルールの自己監査
5. report    : 実行した検証と未検証条件（他 OS・CI 実ディスパッチ等）を分けて報告
```

読者側の導線フロー（README が担う制御フロー）: 訪問者 → 概要・Status で実態と未決定の境界を把握 → Quickstart で環境再現 → Documentation 節から `docs/designs/`・Issue #1/#2/#3 へ、コントリビューター・エージェント → `AGENTS.md` の作業契約へ誘導する。

## Non-Functional Constraints

### 表現ルール（verified fact vs direction の境界）

| 状態区分 | 必須の表現ラベル | 適用対象 | 書き方の例 |
| --- | --- | --- | --- |
| 検証済み事実 | 断定可。実施環境を添える | mise タスク・ツール pin・最小ワークスペース・各コマンドの成功 | 「Windows 11 x64（Git Bash）で検証済み」 |
| 構想（方向性） | 「構想」「計画」「方向性」 | `ruviform-audio`/`ruviform-video`、DSP・解析・codec・container・SIMD | 「〜を構想している」 |
| 検討中 | 「検討中」「採否は #1 で決定」 | `wgpu`・WebView UI・ネイティブ UI・プラグイン連携 | 「検討中の項目」 |
| 未確定 | 「未確定」「#1 で決定」 | 対応プラットフォーム・公開 API・サポート形式・ライセンス・最終クレート分割・edition/MSRV | 「未確定」 |
| 未検証 | 「未検証」＋実施範囲の限定 | macOS/Linux 手順・GitHub Actions 実ディスパッチ・mise 未導入の素の Windows 環境 | 「実施は Windows 11 x64 のみ」 |
| 仮置き | 「仮置き」 | `Cargo.toml` の `license`/`edition`/`rust-version`、`crates/ruviform-core` 名 | 「仮置き（#1 で再評価）」 |

### 禁止記載（虚偽とみなす表現）

- `LICENSE` が存在する、または MIT 等に決定済みであるという記載
- `ruviform-audio`/`ruviform-video` 等の未作成クレート・API が実在するという記載
- RT-safe・リアルタイム保証を API 契約として断定する記載
- GitHub Actions の実ディスパッチが成功済みという記載（静的検証 pass のみ記載可）
- macOS/Linux で手順が検証済みという記載
- 「Use this template」等、テンプレート再利用のための手順
- 存在しない文書・テスト・ベンチマークへのリンク・言及

### その他の制約

- 事実の一次ソースは IR `verify-reproducible-setup-and-work-memo.md`「作業メモ（#3 転記用）」とし、そこに無い性能数値・互換性主張を創作しない
- `apm.yml`/`mise.toml` の無関係な再構成をしない（ADR の「正当な理由がない限り維持」に従う）
- 生成物（`apm.lock.yaml`、`.mise/locks/`、APM 配布物）は手編集せずコマンドで再生成する
- README/AGENTS への転記は「実施済み環境」「未検証 OS / 未実施検証」の両方を含め、片方だけの引用にしない

## Risks and Trade-offs

- 構想記述が確定方針と誤読されるリスク → Status 内のラベル分けと上記表現ルールで抑止するが、文言の最終確認は実装 PR のレビュー観点とする（表現の担保は機械検証しにくい）。
- `apm.yml` の `name` 変更で APM 生成物・`apm.lock.yaml` が陳腐化するリスク → 変更と再生成・`--frozen` 検証を同一タスク（T-IDENT）に閉じ込める。
- #1 の決定後に README/AGENTS の構想記述が陳腐化するリスク → Deferred Design Concerns で追従更新を明示し、README 側にも「詳細設計は #1」の導線を残す。
- `mise install --locked` の環境 caveat（global mise config が repo 外ツールを要求する環境ではその分のみ exit 1 となり得る）→ 実測事実として README の Quickstart に注記し、フォールバック `mise install` を併記する。
- 過剰改訂によるインフラ破壊リスク → ファイルマップで change type と「無変更」を事前に固定し、レビュー対象は判定基準に従って必要箇所のみ触る。

## Deferred Design Concerns

| 据え置く項目 | 理由 | 再着手トリガ | 無視した場合のリスク |
| --- | --- | --- | --- |
| ライセンス決定（`LICENSE` 作成・`Cargo.toml` 仮置き確定） | 配布方針は #1 以降の判断 | 公開・配布方針の決定 | 仮置き値と実ライセンスの不整合が続く |
| 最終クレート分割・公開 API | #1 の詳細設計対象 | #1 着手 | README の構想記述と実装の乖離 |
| `wgpu`・WebView・ネイティブ UI・プラグイン採否 | 同上 | #1 の詳細設計 | 同上 |
| 対応プラットフォーム確定・macOS/Linux 手順検証 | #2 実機検証は Windows 11 x64 のみ | #1 のプラットフォーム設計・他 OS 検証機会 | 未検証差異が文書上見えなくなる |
| RT/オフライン API 契約・`unsafe`/SIMD 規約の確定 | 本設計は意図・暫定原則のみ | #1 の詳細設計 | 暫定原則が未検証の保証として固定化 |
| GitHub Actions 実ディスパッチ検証 | default branch マージ後のみ可能 | #3 マージ後の初回ディスパッチ | CI 手順記述と実挙動の乖離 |
| README/AGENTS のアーキテクチャ記述追従 | 現時点の意図を記す設計のため | #1 完了時 | 古い構想記述の残存 |

## Verification Notes

- [ ] `git grep -in "project.template"`・`git grep -in "Use this template"`・`git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存"` を追跡ファイルに実行し、catch-all として `git grep -in "template\|テンプレート"` も実行して変種（「言語・フレームワークに依存しない」や裸の「テンプレート」等）の残存を確認する。`docs/` 配下の履歴記録（spec/ADR/design/plan/task/IR/test-spec 等の承認済み・記録済み文書）はプレースホルダー書き換え対象から全て除外し、それ以外の追跡ファイルに残存がないこと
- [ ] README が「Use this template」手順を持たず、検証済みコマンド（`mise install --locked`・`mise run setup`・`mise run fmt-check`/`check`/`lint`/`test`/`build`/`verify-ci`）と検証環境の限定・未検証 OS が記載されている
- [ ] README/AGENTS で構想・検討中・未確定・未検証・仮置きの各区分がラベル明示され、禁止記載の表現がない（目視監査）
- [ ] `mise exec -- prek run --all-files` が成功する（`apm.yml`/`mise.toml`/`.pre-commit-config.yaml` の YAML/TOML 構文を含む）
- [ ] `apm install` → `apm lock` が成功し `apm.lock.yaml` が再生成・整合し、`mise exec -- apm install --frozen` が成功する
- [ ] `mise.toml` 変更がコメントのみで `mise install --locked` の復元結果に影響しない（実測で確認）
- [ ] `mise run fmt-check` / `mise run verify-ci` が成功する（ドキュメント変更でコード系検証が壊れていないことのスモーク）
- [ ] `LICENSE` が作成されておらず、`Cargo.toml` が無変更で、README/AGENTS がライセンス未決定・仮置き注記を持つ
- [ ] `.gitattributes` の LFS 拡張子ルールが維持され、非メディアフィクスチャ方針が AGENTS/README に文書化されている
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` が docs グラフの不整合を報告しない（`node` は repo 管理の mise ツールではなく global mise config 等に由来する環境前提。検証環境で導入済みであることを確認する）
