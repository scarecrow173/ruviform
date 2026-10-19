---
id: "PLAN-034IvX2cFeBjPV4UMfDJ2x"
type: "plan"
status: "approved"
title: "テンプレート由来ドキュメント・識別子のRuviform固有化の実装"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
test-spec-skip: "ドキュメントとメタデータのみの変更であり、外部から観測可能な新規振る舞いを持たない。検証はプレースホルダー grep・構文検証・lockfile 整合・prek/文書監査（V-1〜V-11）で行うため test-spec は作成しない"
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
  implemented-by:
    - "../tasks/baseline-placeholder-sweep.md"
    - "../tasks/rewrite-readme-for-ruviform.md"
    - "../tasks/update-apm-identity-and-lockfile.md"
    - "../tasks/rewrite-agents-for-ruviform.md"
    - "../tasks/review-config-files-minimal-edits.md"
    - "../tasks/run-placeholder-sweep-and-verification.md"
  depends-on:
    - "dev-foundation-bootstrap.md"
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
    - "../designs/dev-foundation.md"
    - "../designs/overview.md"
    - "../adr/replace-template-derived-docs-metadata-with-ruviform.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/replace-template-derived-docs-with-ruviform.md"
    - "../specs/replace-template-derived-docs-with-ruviform.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/2"
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "../impl/ir/verify-reproducible-setup-and-work-memo.md"
  defers: []
  deferred-by: []
---

# テンプレート由来ドキュメント・識別子のRuviform固有化の実装 Implementation Plan

## Goal

Issue #3 の chore 作業として、承認済み design `DESIGN-034IvBNBZvoikAgd6DJxdD` が定義する差分境界に従い、リポジトリ初期化時に引き継いだ汎用プロジェクトテンプレート由来の説明・識別子・作業規約を、Ruviform の現状（Rust を中心とした再利用可能なマルチメディア基盤・設計/初期セットアップ段階）に合わせて置き換える。対象は `README.md` 全面改稿、`AGENTS.md` 節単位改稿と暫定規約節の追加、`apm.yml` の識別子更新と `apm.lock.yaml` 再生成、`mise.toml` コメント適合、`.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` のレビュー（無変更を既定）、全追跡ファイルのプレースホルダースイープと総合検証である。

## Scope

単一の実装ストリームとして扱う（分割しない）。全スライスが同一の小さな追跡ファイル集合（ルートの README/AGENTS/`apm.yml`/`mise.toml`/フック設定）を触り、最終スイープ T-VERIFY という共有のクリティカルパスを持ち、リリース意図も「Issue #3 を #1 の完了を待たずに閉じられる状態にする」で同一のため、design のファイルマップの担当タスク案を 5 つの実装スライスに写像する（`mise.toml` の comment-edit は design では T-IDENT 担当だったが、本 plan では設定ファイル見直しの責務に揃えて T-CONFIG へ意図的に再グループする）。

| スライス | 内容 | 主な触れるファイル |
| --- | --- | --- |
| T-README | `README.md` の全面改稿 | `README.md` |
| T-IDENT | `apm.yml` の `name`/`description` 更新と `apm.lock.yaml` 再生成・整合確認 | `apm.yml`、`apm.lock.yaml`（生成物） |
| T-AGENTS | `AGENTS.md` の節単位改稿と暫定規約節の追加 | `AGENTS.md` |
| T-CONFIG | `mise.toml` コメント適合と `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` のレビュー（無変更既定）・最小修正 | `mise.toml`、`.pre-commit-config.yaml`、`.gitignore`、`.gitattributes` |
| T-VERIFY | プレースホルダースイープ・構文/フック/lockfile/文書監査の総合検証と ADR `derived-by` メタデータ編集 | 全追跡ファイル（検査）、`docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`（front matter のみ） |

境界（Out of Scope）は design / spec と同一: `ruviform-audio` 等のコード・クレート作成、`LICENSE` 作成・`Cargo.toml` の `license` 確定、mise/APM/prek/LFS の仕組み自体の再構成・タスク定義本体・CI ワークフローの変更、`docs/` 配下の既存設計文書の改稿（宣言済みの ADR `derived-by` メタデータ更新を除く）、プラットフォーム・公開 API・フォーマット・RT 契約の決定（#1 の詳細設計）は含めない。

## Upstream Documents

- [../specs/replace-template-derived-docs-with-ruviform.md](../specs/replace-template-derived-docs-with-ruviform.md) — `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（approved）。Requirements 1–7 と Acceptance Criteria 7 件が本計画の検証マトリクスの正本
- [../adr/replace-template-derived-docs-metadata-with-ruviform.md](../adr/replace-template-derived-docs-metadata-with-ruviform.md) — `ADR-034Ivm746Q38Wa21HeqUiI`（accepted）。README/AGENTS の改稿定位、「構想・検討中・未決定」の明示付き記載、`name: ruviform` 化、ライセンス未決定の明示、拡張子 LFS ルール維持＋非メディアフィクスチャ方針の決定に従う
- 一次ソース: GitHub Issue [#3](https://github.com/scarecrow173/ruviform/issues/3)（`relations.source`）
- 前提: Issue #2 の完了成果として計画 [dev-foundation-bootstrap.md](./dev-foundation-bootstrap.md)（`PLAN-034IdrispOUms1c32192B6`、`depends-on`）。その実機検証の一次記録が README/AGENTS の事実記述の基盤

## Design Inputs

- [../designs/replace-template-derived-docs-with-ruviform.md](../designs/replace-template-derived-docs-with-ruviform.md) — `DESIGN-034IvBNBZvoikAgd6DJxdD`（approved）。ファイルマップ（変更種別）、README 新構成、AGENTS 再編成表、`apm.yml`/`mise.toml` 差分設計、レビュー対象ファイルの判定基準、表現ルール（検証済み事実/構想/検討中/未確定/未検証/仮置き）・禁止記載、Verification Notes を実装の直接入力とする
- 事実の一次ソース: IR [../impl/ir/verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md)「作業メモ（#3 転記用）」。実施済み環境・セットアップ手順・前提条件・lockfile 再生成条件・暫定開発規約・未検証 OS/未実施検証はすべてこの一次記録に限定し、そこに無い性能数値・互換性主張を創作しない

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `README.md` | Ruviform 固有の顔: タイトル・概要・Status・Quickstart・Repository layout・Documentation・Contributing・License | T-README。rewrite（全面改稿）。「Use this template」系の記述は残さない |
| `AGENTS.md` | エージェント共通の Ruviform 作業契約 | T-AGENTS。section-edit。冒頭・作業契約・mise/APM/Git 節を改稿し、`開発・設計原則`（L7–L66）は保持、暫定規約節を追加 |
| `apm.yml` | APM 正本の `name`/`description` | T-IDENT。field-edit。L1 `name` と L3 `description` の 2 フィールドのみ |
| `apm.lock.yaml` | APM 依存解決の lockfile | T-IDENT。generated（再生成）。`apm install` → `apm lock` で再解決し整合確認。手編集しない |
| `mise.toml` | ツール pin・検証タスクの正本 | T-CONFIG。comment-edit。L8 のテンプレート前提コメントのみ現状適合化。task description・定義本体は原則無変更 |
| `.pre-commit-config.yaml` | prek の言語非依存フック定義 | T-CONFIG。review / 無変更見込み。L1 コメント定位と builtin フック群の実態適合を確認 |
| `.gitignore` | 秘密情報・生成物の除外 | T-CONFIG。review / 無変更見込み。`target/`・APM/Serena 生成物・`.env*` の除外は適合済み |
| `.gitattributes` | 改行・バイナリ・拡張子 LFS ルール | T-CONFIG。review / 無変更。拡張子 LFS ルール維持（ADR 決定）。フィクスチャ方針は AGENTS/README に文書化し属性行は変えない |
| `Cargo.toml` | workspace ルート（`license = "MIT"` 等は仮置き） | no-change。README/AGENTS のライセンス注記と整合させるのみ |
| `mise.lock` / `.mise/locks/` | mise ツール解決の確定値 | no-change 見込み。`mise.toml` はコメントのみの変更でロック結果に影響しない想定。V-6 で実測確認 |
| `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md` | accepted ADR | T-VERIFY。metadata-edit。`relations.derived-by` に本 plan へのバックリンクを追加し design バックリンクの存在を確認（本文・`status: "accepted"` は無変更） |
| その他追跡ファイル | プレースホルダー残存の母集団 | T-VERIFY。sweep。`git ls-files` ベース。`docs/` 配下の履歴記録（spec/ADR/design/plan/task/IR/test-spec 等）は書き換え対象から全て除外 |

未確定要素: レビュー対象 3 ファイル（`.pre-commit-config.yaml`/`.gitignore`/`.gitattributes`）の最終判定は実装時のレビュー結果に委ねる（無変更見込み・Open Questions / Gaps 参照）。

## Implementation Sequence

実装は S-1（ベースラインスイープ）→ T-README ∥ T-IDENT ∥ T-CONFIG（並行可）→ T-AGENTS（T-IDENT 後）→ T-VERIFY の順がクリティカルパス。各ステップは 1 アクション。

### Phase S: ベースラインスイープ（共通前提）

- [ ] S-1. `git ls-files` を母集団にプレースホルダー所在を確定して記録する: `git grep -in "project.template"`、`git grep -in "Use this template"`、`git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存\|言語・フレームワークに依存しない"`、catch-all として `git grep -in "template\|テンプレート"`。`docs/` 配下の履歴記録と `apm.lock.yaml` 等の生成物内 APM パッケージ由来パス（`assets/templates/` 等）は書き換え対象外と明示し、T-VERIFY の最終スイープと比較できるベースライン一覧を残す（現状: `README.md`・`AGENTS.md`・`apm.yml`・`mise.toml` L8 コメントが主な残存箇所）

### Phase T-README: `README.md` 全面改稿

- [ ] R-1. `README.md` を design の新構成どおり全面改稿する: `# Ruviform`＋概要段落（Rust を中心とした再利用可能なマルチメディア基盤という現時点の方向性、音声・動画とその下位の計算基盤（DSP・解析・codec・container・SIMD）を同じ基盤上で再利用可能にする意図、対象読者を一文で、詳細設計は Issue #1 の後続タスクである旨の案内）、`## Status`、`## Quickstart`、`## Repository layout`、`## Documentation`、`## Contributing`、`## License`
- [ ] R-2. `## Status` でラベル分けを行う: 実在・検証済み（Rust 1.99.0 ツールチェーンの mise pin＋`mise.lock`、ルート `Cargo.toml`＋`crates/ruviform-core` の最小ワークスペースと smoke test、mise 検証タスク群、`workflow_dispatch` 専用 CI＝静的検証のみ済み・実ディスパッチ未実施）/ 構想（`ruviform-audio`/`ruviform-video` と下位の DSP・解析・codec・container・SIMD）/ 検討中（`wgpu`・WebView UI・ネイティブ UI・プラグイン連携）/ 未確定（対応プラットフォーム・公開 API・サポート形式・ライセンス・edition/MSRV・最終クレート分割）
- [ ] R-3. `## Quickstart` に IR 一次記録の検証済みコマンドのみ記載する: `mise install --locked`（正本・固定復元。global mise config が repo lockfile 外ツールを要求する環境ではその分のみ exit 1 となり得る caveat とフォールバック `mise install` を併記）、`mise run setup`（= `apm install`＋`prek install`＋`git lfs install --local`）、`mise run fmt-check`/`check`/`lint`/`test`/`build`、`mise run verify-ci`（CI 同等・`target/ci/` 並存の注意）、厳格検証 `mise exec -- apm install --frozen`。前提条件（mise 導入済み・`mise trust`・ネットワーク到達先・git 認証・書き込み権限）と、実機検証は Windows 11 x64（Git Bash）のみ・macOS/Linux は未検証である旨を明記する
- [ ] R-4. `## Repository layout` に実在構成のみのツリー/表を書く: `Cargo.toml`/`Cargo.lock`/`crates/ruviform-core/`/`mise.toml`/`mise.lock`/`.mise/locks/`/`apm.yml`/`apm.lock.yaml`/`.pre-commit-config.yaml`/`.gitattributes`/`.gitignore`/`.github/workflows/ci.yml`/`docs/`/`AGENTS.md`。存在しないディレクトリ・文書を案内しない。節末尾に LFS フィクスチャ方針の注記枠を設ける（`.gitattributes` の音声/動画拡張子ルール維持・小さなテストフィクスチャは非メディア形式で Git に置く・CI は必要になるまで LFS オブジェクトを取得しない）
- [ ] R-5. `## Documentation`・`## Contributing`・`## License` を書く: 存在する文書へのリンクのみ（`docs/designs/overview.md`・`docs/designs/dev-foundation.md`・`docs/specs/`・`docs/adr/`・Issue #1/#2/#3 への導線）、`AGENTS.md` の作業契約・Conventional Commits・Issue 駆動・prek フックへの導線、**ライセンス未決定の明示**（`LICENSE` ファイルは存在せず、`Cargo.toml` の `license = "MIT"` は仮置きで配布方針の確定ではない旨の注記）
- [ ] R-6. 禁止記載の自己監査を行う: `LICENSE` が存在する/ライセンス決定済み、未作成クレート・API が実在する、RT-safe・リアルタイム保証の断定、GitHub Actions 実ディスパッチ成功済み、macOS/Linux 検証済み、「Use this template」等のテンプレート再利用手順、存在しない文書・テスト・ベンチマークへのリンク・言及——のいずれもないことを確認する

### Phase T-IDENT: `apm.yml` 識別子更新と APM 再生成

- [ ] I-1. `apm.yml` の 2 フィールドのみ変更する: `name: project-template` → `name: ruviform`、`description: Minimal language-agnostic development foundation` → `description: Reusable multimedia foundation centered on Rust`。`version`/`targets`/`dependencies.apm`/`devDependencies.mcp`（serena・mise）/`includes`/`scripts` は ADR 決定どおり無変更
- [ ] I-2. `apm install` → `apm lock` を実行して `apm.lock.yaml` を再生成し、`apm.yml` との整合と生成物（`apm_modules/`・各エージェント配布物は非追跡）との差分を確認する。lockfile・生成物は手編集しない
- [ ] I-3. `mise exec -- apm install --frozen` を実行し exit 0 を確認する（`apm.yml` ↔ `apm.lock.yaml` が非同期なら失敗する厳格検証。apm-cli 0.33.0 は `--frozen` 対応済み・IR 実測済み）
- [ ] I-4. `git status --ignored` で `apm_modules/`・各エージェント配布生成物が非追跡のままであることを確認する。APM 0.33.0 に専用 validate コマンドはないため、命名制約の適合は I-2/I-3 の成功で確認した旨を実施記録に残す（ADR の確認方法をそのまま採用）

### Phase T-AGENTS: `AGENTS.md` 節単位改稿

- [ ] G-1. 冒頭（L1–L5）を書き換える: 「このリポジトリは Ruviform — Rust を中心とした再利用可能なマルチメディア基盤（設計・初期セットアップ段階）である」旨と、以下がエージェント共通の作業契約であることを述べる。「言語・フレームワークに依存しない開発基盤のテンプレート」「派生リポジトリ」への言及は消す
- [ ] G-2. `## リポジトリの作業契約` を編集する: 作業契約の箇条は保持し、「このテンプレートに言語・フレームワーク固有の構成や、不要な `.github/`・`docs/` ディレクトリを追加しない。派生プロジェクトで必要に応じて定義する。」（L75）を削除し、「実装されていない API・クレート・検証を実在/実行済みと報告しない」を明文化する
- [ ] G-3. `## 開発環境（mise）` を編集する: 「テンプレートには共通基盤ツールのみを置く…」（L80）を現状に書き換え（`mise.toml` は共通基盤ツール＋`rust = "1.99.0"`・actionlint を管理し、検証タスクも同ファイルが正本）、初回セットアップのコマンド列を IR 検証済み手順に同期（`mise install --locked` が正本・`mise install` はフォールバック・`mise run setup`・厳格検証 `mise exec -- apm install --frozen`）、「派生リポジトリのmiseタスク」（L91）を「`mise.toml` のタスク」に修正し、lockfile 再生成条件（`mise.lock`/`apm.lock.yaml`/`Cargo.lock` の各トリガと復旧手順）を IR の表に基づき追記する
- [ ] G-4. `## AIエージェント環境（APM）` を微修正する: 運用規約は実態どおり保持しつつ、`apm.yml` の `name` が `ruviform` である現状に記述を合わせる
- [ ] G-5. `## Gitと安全性` に LFS フィクスチャ方針を 1 箇条追加する: 小さなテストフィクスチャは非メディア形式（JSON/YAML 等）で Git に置き、CI はフィクスチャが必要になるまで LFS オブジェクトを取得しない（現行 `ci.yml` も LFS 取得なし・ADR 決定の文書化）
- [ ] G-6. 新規節 `## Rust ワークスペースと暫定開発規約`（仮題）を追加する: 構成（ルート `Cargo.toml` の `[workspace]`/`workspace.package`/`workspace.lints`/`[profile.dev]`/`[profile.ci]`＋仮置きの最小メンバー `crates/ruviform-core`。最終分割・公開 API は #1）、検証入口（#2 の mise タスク対応表と「cargo コマンドは `mise run` 経由で呼ぶ」規約。`*-ci` は CI 用途限定）、暫定規約（`unsafe` 原則禁止＋SAFETY コメント付き局所許可・default feature 最小・依存追加は理由記録・`workspace.lints` が正本で `-D warnings` ゲート・公開 API の例は doctest）、暫定原則（下位基盤クレートは上位クレートに依存しない・SIMD の前提/不変条件。詳細契約は #1 で確定）、ライセンス注記（`LICENSE` 不在・未決定・`license = "MIT"` は仮置き）、RT/オフラインの設計意図（リアルタイム音声経路ではブロッキング操作・意図しない allocation を避ける方向で設計する暫定原則。API ごとの保証は #1 で確定し未確定 API を RT-safe と断定しない旨を明記）
- [ ] G-7. `## 開発・設計原則`（7 サブセクション）が保持されていることを `git diff` で確認する（verbatim またはほぼ verbatim）
- [ ] G-8. 表現ルールの自己監査を行う: 構想/検討中/未確定/未検証/仮置きの各区分がラベル明示され、design の禁止記載に該当する表現がないことを確認する

### Phase T-CONFIG: 設定ファイルのレビューと最小修正

- [ ] C-1. `mise.toml` L8 コメント `# Infrastructure tools only; add a project language toolchain as needed.` を Rust ツールチェーン追加済みの現状に合わせて書き換える（例: `# Shared infrastructure tools plus this repository's pinned Rust toolchain.`）。コメントは英語のまま維持。`[tools]`・task description・`run`/`env` 定義・`[settings]`/`[env]` は無変更
- [ ] C-2. `mise.toml` の全 task description をレビューし実態適合を確認する（食い違いが見つかった場合のみ description を修正。タスク定義本体は変更しない）
- [ ] C-3. `.pre-commit-config.yaml` をレビューする: L1「Language-agnostic checks」という定位と builtin フック群（trailing-whitespace/end-of-file-fixer/check-yaml/check-toml/mixed-line-ending/fix-byte-order-marker/detect-private-key/check-illegal-windows-names）が Ruviform に適合することを確認。既定は無変更とし、コメント文言が「テンプレート前提」と読める場合のみ微修正
- [ ] C-4. `.gitignore` をレビューする: `target/`・APM/Serena 生成物（`apm_modules/`・`.agents/`・各エージェント配布物）・秘密情報（`.env*`）・ローカル状態の除外が適合済みであることを確認。既定は無変更
- [ ] C-5. `.gitattributes` をレビューする: 音声・動画・大型編集ソースの拡張子 LFS ルールが維持されていることを確認する（属性行は変更しない。フィクスチャ方針は G-5・R-4 で文書化済みであることと整合確認）
- [ ] C-6. `Cargo.toml` が無変更であることを確認し、C-1〜C-5 のレビュー結果（変更/無変更とその理由）を実施記録に残す

### Phase T-VERIFY: 総合検証と ADR メタデータ編集

- [ ] F-1. ADR `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md` の front matter `relations.derived-by` に本 plan（`../plans/replace-template-derived-docs-with-ruviform.md`）へのバックリンクを追加し、design へのバックリンクが存在することを確認する。あわせて本 plan の `relations.derives-from` に当該 ADR を追加して双方向リンクを一致させ、design の先例に倣い重複を避けるため `relations.related` からは当該 ADR を除去する（plan の relation 更新は規約上許容される in-place edit。ADR の本文・`status: "accepted"` は無変更）
- [ ] F-2. プレースホルダー最終スイープを実行し S-1 のベースラインと比較する（V-1）
- [ ] F-3. `mise exec -- prek run --all-files` を実行する（V-5。auto-fix 系フックの修正が出たら取り込んでから再実行）
- [ ] F-4. `mise install --locked` と `mise exec -- apm install --frozen` を実行する（V-4・V-6 の厳格確認。global-config caveat は Risk 8 の判定基準に従う）
- [ ] F-5. `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run verify-ci` を実行する（V-6 の挙動不変スモーク）
- [ ] F-6. README/AGENTS の内容監査を行う（V-2・V-3・V-7・V-8・V-9）
- [ ] F-7. `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` を実行し docs グラフの不整合がないことを確認する（V-10。`node` は repo 管理の mise ツールではなく global mise config 等に由来する環境前提のため、導入済みであることを先に確認する）
- [ ] F-8. 実行した検証と未検証条件（macOS/Linux・mise 未導入の素の Windows・GitHub Actions 実ディスパッチ）を区別して実施記録に報告する（V-11）

## Dependencies and Risks

### Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| S-1（ベースラインスイープ） | — | T-README, T-IDENT, T-AGENTS, T-CONFIG, T-VERIFY |
| T-README | S-1 | T-VERIFY |
| T-IDENT | S-1 | T-AGENTS, T-VERIFY |
| T-AGENTS | S-1, T-IDENT | T-VERIFY |
| T-CONFIG | S-1 | T-VERIFY |
| T-VERIFY | T-README, T-IDENT, T-AGENTS, T-CONFIG | — |

- クリティカルパス: S-1 → T-IDENT → T-AGENTS → T-VERIFY。`apm.yml` の `name` 確定が AGENTS の APM 節記述の前提になるため T-IDENT を T-AGENTS に先行させる
- 並行可能な作業: T-README ∥ T-IDENT ∥ T-CONFIG。T-README と T-AGENTS は互いに独立だが、両方の完了が最終スイープ T-VERIFY の前提。T-IDENT がトリガする `apm install`/`apm lock` の整合確認は T-VERIFY の検証（F-4）でも再確認する

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | README/AGENTS の記述が未検証領域に踏み込み虚偽記載となる（構想を実装済み、未実施検証を実行済みと誤記等） | med | high | design の表現ルール表を強制適用し事実は IR 一次記録に限定。R-6/G-8 の自己監査に加え V-3 のラベル監査・V-9 のコマンド実在性確認を実施。表現の最終担保は実装 PR のレビュー観点とする |
| 2 | `apm.yml` の `name` 変更で `apm.lock.yaml`・APM 配布生成物が陳腐化し `apm install --frozen` が壊れる | med | med | 変更と再生成・`--frozen` 検証を T-IDENT 単一タスクに閉じ込める。不整合時は `git checkout -- apm.lock.yaml` → `apm install`/`apm lock` 再実行で復旧 |
| 3 | 編集ファイルが prek フック（trailing-whitespace/end-of-file-fixer/mixed-line-ending/fix-byte-order-marker 等）に抵触 | med | low | F-3 で `prek run --all-files` を実行。auto-fix 系フックの修正は取り込んでから再実行する |
| 4 | `docs/` 配下の履歴記録（spec/ADR/design/plan/task/IR/test-spec 等の承認済み・記録済み文書）を誤って書き換える | low | med | 書き換え対象を `relations.changes.modified` の宣言ファイルに限定し `docs/` は全て除外。`git diff --name-only` で宣言外の逸脱を検出する |
| 5 | lockfile 再生成の churn（`apm.lock.yaml`/`mise.lock`/`.mise/locks/` に無関係な差分が混入） | med | low | 生成物はコマンドでのみ再生成し手編集禁止。差分レビューで無関係な変更を切り分け、IR「lockfile 再生成条件」表の復旧手順（`git checkout -- <lockfile>` → lock コマンド再実行）を適用 |
| 6 | 外部依存の可用性: APM 依存 git repo・PyPI・crates.io・static.rust-lang.org へのネットワーク障害で `apm install`/`mise install` が失敗 | low | med | IR 記録の前提条件（到達先・git 認証・非対話時 `GIT_TERMINAL_PROMPT=0` 推奨）に従う。再試行または lockfile 復旧手順を適用 |
| 7 | データ損失: README/AGENTS 全面改稿で Ruviform に有効な既存記述を消失 | low | med | design が保持箇所（`開発・設計原則` は verbatim 保持等）を明示済み。ドキュメントのみの変更で `git revert`/`checkout` 可能。S-1 のベースライン記録で変更前状態を保全 |
| 8 | `mise install --locked` の環境 caveat（global mise config が repo lockfile 外ツールを要求する環境ではその分のみ exit 1 となり得る）を検証時に誤判定 | med | low | 既知の実測事実として README Quickstart に注記し `mise install` フォールバックを併記。検証では repo 管理の全 7 ツールが解決することを以て合否判定とする |
| 9 | レビュー対象 3 ファイルの「無変更」判定漏れでテンプレート前提の文言が残存 | low | low | C-3〜C-5 で design の判定基準どおりレビューし結果を C-6 で記録。V-1 の catch-all grep が最終防御 |
| 10 | front matter の双方向不整合（plan/spec/design/ADR の derives-from/derived-by）で docs 監査が warning を出す | low | low | F-1 で ADR `derived-by` と plan `derives-from` を同時に更新し、F-7 の audit_docs で確認する |

## Task Breakdown

task-doc 化の単位はスライスと一致させ、1 task = 1 実装まとまり・複数 commit 可とする。推奨分割:

| 候補 task | 含むステップ | 完了条件（検証エントリ） |
| --- | --- | --- |
| T-README: `README.md` 全面改稿（task-doc 案 `docs/tasks/rewrite-readme-for-ruviform.md`） | R-1〜R-6 | V-2, V-3 |
| T-IDENT: `apm.yml` 識別子更新と APM 再生成（task-doc 案 `docs/tasks/update-apm-identity-and-lockfile.md`） | I-1〜I-4 | V-4 |
| T-AGENTS: `AGENTS.md` 節単位改稿（task-doc 案 `docs/tasks/rewrite-agents-for-ruviform.md`） | G-1〜G-8 | V-2, V-3, V-9 |
| T-CONFIG: 設定ファイルのレビューと最小修正（task-doc 案 `docs/tasks/review-config-files-minimal-edits.md`） | C-1〜C-6 | V-5, V-6, V-8 |
| T-VERIFY: 総合検証と ADR メタデータ編集（task-doc 案 `docs/tasks/run-placeholder-sweep-and-verification.md`） | S-1 のベースライン共有＋F-1〜F-8 | V-1〜V-11 |

前提ステップ: S-1（ベースラインスイープ）はいずれのタスクにも属さない共有の pre-dispatch ステップであり、スライス dispatch 前に一度だけ実行する。T-VERIFY 行の「S-1 のベースライン共有」は、そのベースラインを消費して最終スイープと比較する意味であり、S-1 自体の実行は T-VERIFY の担当範囲外である。

task-doc の作成は plan 承認後の planning-flow 後段で行う。本計画はドキュメント・メタデータのみの変更で test-spec を作成しない（front matter `test-spec-skip` 参照）。

## Verification

| Step | Verification command or criteria | Pass condition |
| --- | --- | --- |
| V-1（AC: テンプレート残存なし） | `git grep -in "project.template"`・`git grep -in "Use this template"`・`git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存\|言語・フレームワークに依存しない"` を追跡ファイルに実行し、catch-all として `git grep -in "template\|テンプレート" -- . ':(exclude)docs/**'` も実行して変種の残存を確認 | `docs/` 配下の履歴記録と `apm.lock.yaml` 等の生成物内 APM パッケージ由来パス（`assets/templates/` 等）を除き、追跡ファイルに残存がないこと |
| V-2（AC: README/AGENTS が実態適合・#2 手順反映・未検証明示） | README が「Use this template」手順を持たず、design のセクション構成どおりで、検証済みコマンド（`mise install --locked`・`mise run setup`・`mise run fmt-check`/`check`/`lint`/`test`/`build`/`verify-ci`・`mise exec -- apm install --frozen`）と実施環境の限定（Windows 11 x64 Git Bash のみ・macOS/Linux 未検証）が記載されていることを目視確認 | 全セクション・全コマンド・環境限定・未検証条件が記載済み |
| V-3（AC: 境界明示・禁止記載なし） | README/AGENTS で構想・検討中・未確定・未検証・仮置きの各区分がラベル明示され、禁止記載（`LICENSE` 存在/ライセンス決定済み・未作成クレートや API の実在・RT-safe 断定・CI 実ディスパッチ成功・macOS/Linux 検証済み・テンプレート再利用手順・存在しない文書リンク）がないことを目視監査 | ラベル分けが全該当箇所に適用され、禁止記載がない |
| V-4（AC: `apm.yml` 識別子・lockfile 整合） | `git diff apm.yml` で `name`/`description` の 2 行のみの変更を確認、`apm install` → `apm lock` で `apm.lock.yaml` を再生成し整合確認、`mise exec -- apm install --frozen` を実行 | diff が 2 フィールドのみ・`targets`/依存/MCP が維持・再生成成功・`--frozen` exit 0 |
| V-5（AC: 構文有効・prek 実行可） | `mise exec -- prek run --all-files` | exit 0（check-yaml が `apm.yml`/`.pre-commit-config.yaml`、check-toml が `mise.toml`/`Cargo.toml` の構文を検証） |
| V-6（AC: 無関係な再構成なし・挙動不変） | `mise install --locked`、`mise run fmt-check`/`check`/`lint`/`test`/`verify-ci`、`git diff mise.toml` | 全コマンド exit 0（インフラ無変更のため変更前と同じ挙動であることのスモーク）。`mise.toml` の diff がコメントのみで `mise.lock`/`.mise/locks/` の復元結果に影響しないことを実測確認。global-config caveat は Risk 8 の判定基準に従う |
| V-7（AC: LICENSE 不在・ライセンス未決定の整合） | `test ! -f LICENSE`、`git diff --stat Cargo.toml` が空であること、README `## License` と AGENTS 暫定規約節がライセンス未決定・`license = "MIT"` 仮置き注記を持つことを確認 | `LICENSE` 未作成・`Cargo.toml` 無変更・両文書に注記あり |
| V-8（AC: LFS 方針整合） | `git diff .gitattributes` が空であること（拡張子 LFS ルール維持）、AGENTS `Gitと安全性` の追記箇条と README `Repository layout` の注記が非メディアフィクスチャ方針を文書化していることを目視確認 | 属性ファイル無変更・方針が両文書に記載 |
| V-9（AC: 記載コマンドの実在性） | README/AGENTS 中の `mise run <name>`・`mise exec -- <cmd>` 記載を抽出し、`mise.toml` の `[tasks.<name>]` 定義と `mise tasks` 出力と突合 | 記載コマンド名がすべて `mise.toml` に存在（`setup`/`fmt`/`fmt-check`/`check`/`lint`/`test`/`build`/`check-ci`/`lint-ci`/`test-ci`/`build-ci`/`verify-ci`） |
| V-10（docs グラフ監査） | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` | docs グラフの不整合を新規に報告しない（`node` は環境前提。導入確認結果を実施記録に残す） |
| V-11（AC: メタデータ編集・検証報告） | ADR `relations.derived-by` に design・plan のバックリンクがあり本文/status が無変更、本 plan の `derives-from` に ADR が追加済み、実施記録が実行済み検証と未検証条件（macOS/Linux・mise 未導入の素の Windows・CI 実ディスパッチ）を区別していることを確認 | front matter の双方向リンクが整合し、報告が両区分を記載 |

## Rollback Strategy

本計画の変更はすべて Git 追跡ドキュメント・設定の編集と生成物のコマンド再生成であり、DB マイグレーションや公開 API 変更のような不可逆操作は含まない。各対象のロールバック:

| 対象 | Trigger | Procedure | Verification |
| --- | --- | --- | --- |
| `README.md`/`AGENTS.md` | 内容欠落・禁止記載混入がレビューで発覚 | `git checkout -- README.md AGENTS.md` で revert（ドキュメントのみの変更） | S-1 ベースラインの grep ヒットが元に戻り V-1〜V-3 が変更前状態を示す |
| `apm.yml`＋`apm.lock.yaml` | `apm install`/`--frozen` 失敗・命名制約不適合 | `apm.yml` の `name`/`description` を元値（`project-template`/元 description）に戻して `apm install` → `apm lock` を再実行、または `git checkout -- apm.yml apm.lock.yaml` | `mise exec -- apm install --frozen` exit 0・`apm.yml` ↔ lockfile 整合 |
| `mise.toml` | 誤ってタスク定義・`[tools]` を変更 | `git checkout -- mise.toml`（コメントのみの変更のため副作用なし） | `mise tasks` が既存タスクを表示・`mise install --locked` 成功 |
| `.pre-commit-config.yaml`/`.gitignore`/`.gitattributes` | レビュー後の微修正がフックや無視設定を壊す | `git checkout -- <file>` | `prek run --all-files` exit 0・`git status --ignored` が期待どおり |
| ADR front matter | `derived-by` 編集の誤り | `git checkout -- docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`（front matter のみの変更） | `status: "accepted"`・本文が無変更・audit_docs で不整合なし |
| 全体 | 任意の失敗 | `git revert <commit>`（全変更が Git 追跡ファイルの編集のみ） | 変更前 HEAD で V-1〜V-11 を再実行 |

## Open Questions / Gaps

- **README/AGENTS の最終文案**: design が逐語確定を Out of Scope としているため、実装タスクで design の構成・表現ルールに従って執筆する。表現の担保は機械検証しにくく、最終確認は実装 PR のレビュー観点
- **`apm.yml` `description` の文案**: design 提案 `Reusable multimedia foundation centered on Rust` を既定値とし、実装時に README 概要段落と語彙を揃えてよい
- **`apm.lock.yaml` の差分内容**: `name` 変更が lockfile のどのフィールドに波及するかは実機実行でのみ確定する。I-2 で差分を確認して記録する
- **レビュー対象 3 ファイルの最終判定**: 無変更見込みだが、レビューで「テンプレート前提」と読める文言が見つかった場合のみ微修正（design のレビュー対象ファイルの判定基準に従い C-6 で結果を記録）
- **`mise.lock`/`.mise/locks/` への影響**: コメントのみ変更のため無影響の想定。V-6 で実測確認し、差分が出れば IR の lockfile 再生成条件に従って復旧する

## Self-Review

- [x] spec Requirements 1–7 と Acceptance Criteria 7 件が T-README/T-IDENT/T-AGENTS/T-CONFIG/T-VERIFY のステップおよび V-1〜V-11 に対応している
- [x] design のファイルマップ・README 新構成・AGENTS 再編成表・`apm.yml`/`mise.toml` 差分設計・レビュー判定基準・表現ルール・禁止記載・Verification Notes が実装順序と検証に反映されている
- [x] `TBD`/`TODO`/`implement later` 等の placeholder は残っていない（未確定事項は Open Questions / Gaps に明示）
- [x] file path・relation・status・コマンド・用語が design/spec/ADR/IR と整合している（`mise install --locked`・`apm install --frozen`・`verify-ci`・`workflow_dispatch`・`name: ruviform` 等）

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（task-doc 化後、review checkpoint 付きで task ごとに実行）
- Independent work streams: T-README ∥ T-IDENT ∥ T-CONFIG は S-1 後に並行可能。T-AGENTS は T-IDENT 後、T-VERIFY は全タスク完了後の直列スイープ。共有ファイルが少なく依存も浅いため、単一 implementer が依存順に実行してもよい
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 仮置きと判断が残る点: README/AGENTS の最終文案（design は構成・表現ルールのみ確定）、レビュー対象 3 ファイルの無変更/微修正判定、`apm.lock.yaml` の実機差分確認は実装タスクに委ねている
- `relations.derives-from` は design・spec をリンク済み。ADR への derives-from は T-VERIFY（F-1）で ADR `derived-by` への plan バックリンクと同時に追加し双方向を一致させる（design が宣言する ADR metadata-edit と #2 の下流バックリンク慣行に整合）
- レビュー重点: README/AGENTS が design の表現ルール（構想・検討中・未確定・未検証・仮置きのラベル明示）と禁止記載を逸脱していないか、`apm.yml` の変更が 2 フィールドのみか、T-VERIFY の sweep が `docs/` 履歴記録と生成物を書き換え対象外にしているか
- 上流参照: spec `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（Requirements/Acceptance Criteria）、design `DESIGN-034IvBNBZvoikAgd6DJxdD`（Components and Boundaries・表現ルール・Verification Notes）、ADR `ADR-034Ivm746Q38Wa21HeqUiI`（Decision Outcome）、IR `verify-reproducible-setup-and-work-memo.md`「作業メモ（#3 転記用）」、Issue #3
