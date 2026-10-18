---
id: "PLAN-034IdrispOUms1c32192B6"
type: "plan"
status: "approved"
title: "Rust 開発基盤のブートストラップ（toolchain pin・最小ワークスペース・mise タスク・手動CI）"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/establish-rust-dev-foundation.md"
  implemented-by:
    - "../tasks/pin-rust-toolchain-and-lockfile.md"
    - "../tasks/create-minimal-workspace-and-smoke-test.md"
    - "../tasks/define-mise-dev-and-ci-tasks.md"
    - "../tasks/add-manual-ci-workflow.md"
    - "../tasks/verify-reproducible-setup-and-work-memo.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/establish-rust-dev-foundation.md"
    - "../designs/dev-foundation.md"
    - "../adr/rust-dev-foundation-toolchain-and-ci-profile.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/lockfile-reproducible-setup.md"
    - "../test-specs/mise-verification-tasks.md"
    - "../test-specs/ci-profile-build-behavior.md"
    - "../test-specs/manual-ci-workflow.md"
  references:
    - "https://doc.rust-lang.org/cargo/reference/profiles.html"
    - "https://doc.rust-lang.org/cargo/guide/build-performance.html"
    - "https://doc.rust-lang.org/cargo/reference/config.html#buildincremental"
    - "https://github.com/scarecrow173/cheaper/pull/109"
  defers: []
  deferred-by: []
---

# Rust 開発基盤のブートストラップ（toolchain pin・最小ワークスペース・mise タスク・手動CI） Implementation Plan

## Goal

Issue #2 の chore 作業として、承認済み design `DESIGN-034IddXFCRKDIW01WXkSla` が定義する最小 Rust 開発基盤（mise への Rust toolchain pin、workspace ルート `Cargo.toml` + 最小クレート `crates/ruviform-core`、ローカル／CI 共通の mise 検証タスク、`workflow_dispatch` 専用の手動 CI、再現インストール検証と #3 転記用の作業メモ）を 1 回の実装ストリームで構築する。

## Scope

単一の実装ストリームとして扱う（分割しない）。全スライスが `mise.toml` / `Cargo.toml` / `.github/workflows/ci.yml` という共有のクリティカルパス上にあり、リリース意図も「Issue #2 を #1 完了を待たずに閉じられる状態にする」で同一のため、design の Scope をそのまま 5 つの実装スライスに写像する。

| スライス | 内容 | 主な触れるファイル |
| --- | --- | --- |
| A | Rust toolchain pin と lockfile 確定 | `mise.toml`、`mise.lock` |
| B | 最小ワークスペースとメンバークレート | `Cargo.toml`、`Cargo.lock`、`crates/ruviform-core/`、`.gitignore` |
| C | mise 開発・検証タスク | `mise.toml`（`[tasks.*]`） |
| D | 手動 GitHub Actions ワークフロー | `.github/workflows/ci.yml` |
| E | 再現インストール検証と #3 転記用作業メモ | `docs/impl/ir/`（既存ディレクトリ。実装時に impl-doc で IR を生成）、実施 PR 本文 |

境界（Out of Scope）は design / spec と同一: DSP・SIMD・WebView 等の実装、最終クレート分割・公開 API・edition/MSRV の確定、`release` チューニング、macOS/Windows CI matrix、システム依存導入、README/AGENTS 本文改稿と `apm.yml` リネーム（#3）、自動トリガー導入（別 Issue）はすべて含めない。

## Upstream Documents

- [../specs/establish-rust-dev-foundation.md](../specs/establish-rust-dev-foundation.md) — `SPEC-034IcwhwIFSNUNKtX7KK0Q`（approved）。Requirements §1–§6 と Acceptance Criteria が本計画の検証マトリクスの正本
- [../adr/rust-dev-foundation-toolchain-and-ci-profile.md](../adr/rust-dev-foundation-toolchain-and-ci-profile.md) — `ADR-034IcJdtqm6TP6MylyP5J7`（accepted）。toolchain pin 方法・`[profile.ci]`・`workflow_dispatch` 限定・mise タスク一本化・`Cargo.lock` 追跡の決定に従う
- 一次ソース: GitHub Issue [#2](https://github.com/scarecrow173/ruviform/issues/2)（`relations.source`）

## Design Inputs

- [../designs/dev-foundation.md](../designs/dev-foundation.md) — `DESIGN-034IddXFCRKDIW01WXkSla`（approved）。ディレクトリ構成、workspace ルート `Cargo.toml` の verbatim ブロック定義（`[workspace]`・`[workspace.package]`・`[workspace.lints]`・プロファイル）、mise タスク表、ワークフローのステップ構成、暫定開発規約、Verification Notes を実装の直接入力とする

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `mise.toml` | `[tools]` に `rust`（固定 stable バージョン）を追加し、`[tasks.*]` に開発・CI 検証タスクを定義する唯一の入口 | 既存の `[settings]`/`[env]`/`[tools]`（uv/prek/git-lfs/apm-cli/serena-agent）と `[tasks.setup]` を破壊しない。`lockfile = true` を維持 |
| `mise.lock` | Rust toolchain を含む解決済みツールバージョンの確定値 | `mise install` → `mise lock` で再生成し追跡継続（既に追跡済み） |
| `Cargo.toml` | workspace ルート。`resolver = "2"`、`members`、`[workspace.package]`、`[workspace.lints]`、`[profile.dev]`/`[profile.ci]`/`[profile.ci.package."*"]` の唯一の置き場 | `[workspace]`・`[workspace.package]`・`[workspace.lints]`・プロファイルブロックを含む全体を design の定義どおり verbatim で採用（仮置きと明記された値を除く） |
| `Cargo.lock` | 依存解決の再現性を保証する lockfile | cargo が生成。ADR の決定どおり Git 追跡（ライブラリ慣習の非追跡は不採用） |
| `crates/ruviform-core/Cargo.toml` | 最小メンバークレートのマニフェスト。`edition.workspace` / `rust-version.workspace` / `[lints] workspace = true` で共通ポリシーを継承 | 仮置き名。`[profile.*]` は書かない（Cargo がメンバー側 profile を警告対象とするため） |
| `crates/ruviform-core/src/lib.rs` | 最小の公開 API（例: `pub fn version() -> &'static str`）と `#[cfg(test)]` smoke test | デバイス非依存の純粋関数のみ。`cargo new/init` は使わず手書きで作成（雛形コマンドは対象未作成時のみの契約に合わせ生成物を完全制御する） |
| `.gitignore` | `target/` と Rust 関連一時生成物の除外 | 既存ルール（`.env`、`apm_modules/`、`.agents/` 等）を保持したまま追記 |
| `.gitattributes` | audio/video 等の既存 LFS ルール | 変更なし。小さいテストフィクスチャ方針の確認結果のみ作業メモへ記録 |
| `.github/workflows/ci.yml` | `workflow_dispatch` 専用の検証ワークフロー（`contents: read`、`ubuntu-latest` 単一 job、`CARGO_INCREMENTAL=0`、前後のディスク計測） | `.github/` 直下は既存（agents/hooks/mcp.json）。`workflows/` は新設 |
| `docs/impl/ir/`（既存ディレクトリ） | impl-doc 規約の Implementation Record を収めるディレクトリ。実装時に生成する IR に、暫定開発規約・LFS フィクスチャ方針・検証済み OS・lockfile 再生成条件等の #3 転記用作業メモを記録する | ディレクトリ自体は新規作成しない。IR ドキュメントをタスク開始時に作成し随時更新する |
| `apm.yml` / `apm.lock.yaml` | 既存 APM 正本と lockfile | 変更なし。`apm lock` の整合と frozen install 相当の再現性確認のみ |

未確定要素: Rust の pin バージョンと actionlint 採否は実装時に実機確認して決定する（Open Questions / Gaps 参照）。これ以外のファイルは追加しない。

## Implementation Sequence

実装はスライス A → B → C → D → E の順がクリティカルパス。各ステップは 1 アクション。

### Phase A: Rust toolchain pin と lockfile 確定

- [ ] A-1. `mise ls-remote rust` で利用可能な stable を確認し、`minimum_release_age = "7d"` を考慮した最新の具体バージョン（例 `1.9x`）を選定する。`stable`/`latest` の浮動指定は使わない
- [ ] A-2. `mise.toml` の `[tools]` に `rust = "<選定バージョン>"` を追記する（既存エントリは変更しない）
- [ ] A-3. `mise install` を実行して toolchain を導入する
- [ ] A-4. `mise exec -- cargo --version`、`mise exec -- cargo fmt --version`、`mise exec -- cargo clippy --version` の疎通を確認する。component が欠ける場合は rustup component 追加手順を `setup` タスクに明示する
- [ ] A-5. `mise lock` で `mise.lock` を更新し、rust エントリが記録されたことを確認する
- [ ] A-6. `mise run setup` を再実行し、既存 APM/prek/Git LFS セットアップが壊れていないことを確認する（`apm install` / `prek install` / `git lfs install --local` が成功）
- [ ] A-7. `mise exec -- apm lock` で `apm.lock.yaml` の整合を確認し、`apm.yml` との差分がないことを確認する
- [ ] A-8. 選定した Rust バージョン・根拠（stable 最新、design の仮置き MSRV `1.85` 以上であること）を作業メモ（IR/PR）に記録する

### Phase B: 最小ワークスペースとメンバークレート

- [ ] B-1. workspace ルート `Cargo.toml` を新規作成する。design が定義するブロック全体を verbatim で転記する: `[workspace]`（`resolver = "2"`、`members = ["crates/ruviform-core"]`）、`[workspace.package]`（`edition = "2021"`、`rust-version = "1.85"`、`license = "MIT"`。いずれも仮置きとコメントで明記）、`[workspace.lints.rust]`（`unsafe_code`・`unused_qualifications`）、`[workspace.lints.clippy]`（`pedantic`・`missing_errors_doc`）、および `[profile.dev]`/`[profile.ci]`/`[profile.ci.package."*"]`
- [ ] B-2. `crates/ruviform-core/Cargo.toml` を新規作成する。`package.name = "ruviform-core"`、`edition.workspace = true`、`rust-version.workspace = true`、`license.workspace = true`、`[lints] workspace = true` を設定し、profile・独自 lint 重複は書かない
- [ ] B-3. `crates/ruviform-core/src/lib.rs` を新規作成する。`pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }` 程度の最小 API と、`#[cfg(test)]` 内に値が空でないことを確認する smoke test を書く
- [ ] B-4. `mise exec -- cargo test --workspace` を実行し smoke test が成功することを確認する（red 確認は不要: 新規追加のため）
- [ ] B-5. `Cargo.lock` が生成されていることを確認し、`git check-ignore Cargo.lock` で除外されていないことを確認して追跡対象にする
- [ ] B-6. `.gitignore` に `target/` を追記する（既存エントリは変更しない）
- [ ] B-7. `git check-ignore target/` で除外を確認し、`git status` で `target/`・`Cargo.lock` 以外に意図しない生成物が出ていないことを確認する

### Phase C: mise 開発・検証タスク

- [ ] C-1. `mise.toml` に `[tasks.fmt]`（`cargo fmt --all`）、`[tasks.fmt-check]`（`cargo fmt --all -- --check`）、`[tasks.check]`（`cargo check --workspace --all-targets`）、`[tasks.lint]`（`cargo clippy --workspace --all-targets -- -D warnings`）、`[tasks.test]`（`cargo test --workspace`）、`[tasks.build]`（`cargo build --workspace`）を定義する。各タスクに `description` を付け、実行コマンドと副作用を明記する（`fmt` はソースファイルを書き換える整形タスク、`fmt-check`/`check`/`lint`/`test`/`build` は非破壊で `target/` への成果物生成のみ、という区別が読み取れること）
- [ ] C-2. `[tasks.check-ci]`（`cargo check --workspace --all-targets --profile ci`）、`[tasks.lint-ci]`（`cargo clippy --workspace --all-targets --profile ci -- -D warnings`）、`[tasks.test-ci]`（`cargo test --workspace --profile ci`）、`[tasks.build-ci]`（`cargo build --workspace --profile ci`）を定義し、すべてに `env = { CARGO_INCREMENTAL = "0" }` を付与する
- [ ] C-3. `[tasks.verify-ci]` を `run = ["mise run fmt-check", "mise run check-ci", "mise run lint-ci", "mise run test-ci"]`（順次実行）と `env = { CARGO_INCREMENTAL = "0" }` で定義する
- [ ] C-4. `mise run fmt` → `mise run fmt-check` → `mise run check` → `mise run lint` → `mise run test` → `mise run build` を順に実行し、すべて成功することを確認する
- [ ] C-5. `mise run verify-ci` を実行し `--profile ci` 配下の一連の検証が成功することと、`target/ci/` が生成されることを確認する
- [ ] C-6. `mise.lock` / `mise.toml` の差分を確認し、既存ツール・タスク定義が維持されていることを確認する。あわせて `.pre-commit-config.yaml` が未変更であることを確認し、prek の軽量フック（YAML/TOML/改行/秘密鍵チェック）と cargo の重い検証の分担を維持する（コミット毎の重いビルド強制を避けるため、コミットフックに cargo 系検証を追加しない）

### Phase D: 手動 GitHub Actions ワークフロー

- [ ] D-1. `.github/workflows/ci.yml` を新規作成する。`on: workflow_dispatch` のみ（入力なし・push/PR/schedule なし）、`permissions: contents: read`、`ubuntu-latest` 単一 job、job env に `CARGO_INCREMENTAL: "0"` を設定する
- [ ] D-2. ステップを design の構成どおり記述する: checkout → `df -h`（事前計測）→ `jdx/mise-action`（フルバージョン pin）+ `mise install --locked` → `mise exec -- prek run --all-files` → `mise run fmt-check` → `mise run check-ci` → `mise run lint-ci` → `mise run test-ci` → 事後計測（`df -h`、`du -sh target`、`du -sh target/ci/deps target/ci/build` は存在する範囲で）。各ステップに明示的な `name` を付ける
- [ ] D-3. cargo cache ステップを追加するか、追加しない場合はその理由を作業メモに記録する。追加する場合は `~/.cargo` の registry/git index のみを `Cargo.lock` ハッシュでキー付けしてキャッシュし（例: `actions/cache` に `path: ~/.cargo/registry` 等、`key: ${{ runner.os }}-cargo-${{ hashFiles('Cargo.lock') }}`）、`target/` 全体・中間成果物はキャッシュしない
- [ ] D-4. `mise exec -- prek run --all-files` で `check-yaml` がワークフローを構文検証して通ることを確認する
- [ ] D-5. actionlint を採用する場合は `mise.toml` `[tools]` に `actionlint = "<pin>"` を追加して `mise install` → `mise lock` → `mise exec -- actionlint .github/workflows/ci.yml` を実行する。採用しない場合はその判断を作業メモに記録する（推奨: 採用。手動ディスパッチ前にブランチ上で検出可能な唯一の意味論検証のため）
- [ ] D-6. ワークフローが default branch に存在しないと `workflow_dispatch` できない制約を作業メモに記録し、本 PR マージ後に Actions UI から手動ディスパッチして実測ログ（`df -h`/`du -sh target`）を取得する手順を明記する

### Phase E: 再現インストール検証と #3 転記用作業メモ

- [ ] E-1. `mise install --locked` を実行し、lockfile から toolchain（cargo/rustfmt/clippy 含む）が復元できることを確認する
- [ ] E-2. `mise exec -- apm install --frozen` の成否を実機確認する。未対応なら `apm install` の再現性を確認する同等手順（`apm.lock.yaml` 差分なしの確認等）を定義して記録する
- [ ] E-3. `git status --ignored` と `git check-ignore` で、`apm_modules/`・`.agents/`・`.env*`・`target/`・Serena ローカル状態等が意図どおり非追跡であり、`Cargo.lock`・`mise.lock`・`apm.lock.yaml` が追跡対象であることを確認する
- [ ] E-4. `.gitattributes` の audio/video 一律 LFS を確認し、「小さいテストフィクスチャは非メディア形式（JSON/YAML 等）で Git に置く」「CI ではフィクスチャが必要になるまで `git lfs pull` を省略する」方針の確認結果を記録する
- [ ] E-5. 暫定開発規約（`unsafe` 原則禁止＋SAFETY コメント付き局所許可、default feature 最小、依存追加は理由記録、`workspace.lints` 正本＋`-D warnings` ゲート、公開 API の例は doctest）を design どおり作業メモへ転記する
- [ ] E-6. 実施した OS・実行したコマンド・検証結果・前提条件（ネットワーク/権限/認証）・lockfile 再生成条件・未検証 OS を `docs/impl/ir/` の Implementation Record と実施 PR 本文に記録し、#3 が転記可能な状態にする

## Dependencies and Risks

### Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| A（toolchain pin + lockfile） | — | B, C, D, E |
| B（workspace + crate） | A | C, E |
| C（mise タスク） | A, B | D, E |
| D（CI ワークフロー） | C（呼び出すタスク名の確定） | E |
| E（再現検証 + 作業メモ） | A–D | — |

- クリティカルパス: A → B → C → D → E（直列）。`mise.toml` と `Cargo.toml` を共有するため実質単一ストリーム
- 並行可能な作業: D-1〜D-3 のワークフロー記述は、C で定義されるタスク名が design で確定済みのため、C の検証実行と並行してファイルを作成できる（検証完了は C の後）

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `apm install --frozen` が apm-cli 0.33.0 で未対応 | med | low | E-2 で実機確認し、未対応なら同等の再現性検証を定義して記録（design 既定のフォールバック） |
| 2 | `workflow_dispatch` は default branch にワークフローが必要で、ブランチ上で CI 実動作を検証できない | high | med | D-4/D-5 の静的検証（check-yaml・actionlint）で代替し、マージ後の手動ディスパッチを受入条件として明記 |
| 3 | mise の rust backend で rustfmt/clippy component が同梱されない | low | med | A-4 で `cargo fmt/clippy --version` 疎通を確認し、欠ける場合は `setup` タスクへ component 追加を明示 |
| 4 | 外部依存の可用性: GitHub Actions・jdx/mise-action・rustup/crates.io のネットワーク障害で toolchain/依存取得に失敗 | low | med | action はフルバージョン pin、必要なネットワーク・権限・認証を前提条件として作業メモに記録 |
| 5 | `mise.toml` 編集が既存ツール（uv/prek/git-lfs/apm-cli/serena-agent）や `setup` タスクを壊す | low | med | 追記のみで既存エントリを変更せず、A-6/C-6 で `mise run setup` 成功と差分確認を検証する |
| 6 | 破壊的操作による喪失: `apm init --yes` や `cargo init` 等が既存ファイルを上書き | low | high | 初期化・雛形コマンドを使わず全ファイルを手書きで作成し、`apm.yml`/`apm.lock.yaml`/`mise.toml` 既存部分に触れない（spec 契約） |
| 7 | 仮置き値（クレート名 `ruviform-core`、edition 2021、MSRV 1.85、pin Rust バージョン）が #1 設計を過度に拘束 | med | low | すべて仮置きと明記し、根拠を作業メモ/PR に残して #1 で再評価可能にする |
| 8 | actionlint の mise registry 名・pin バージョンが解決できない | low | low | D-5 のフォールバックとして prek `check-yaml` のみに留め、判断を記録する |
| 9 | ローカルで `*-ci` 常用による `target/ci/` + `target/debug/` 並存の容量増 | med | low | `*-ci` タスクは CI 用途に限定する旨を作業メモに明記。CI では同一 job で `dev`/`ci` を二重ビルドしない |
| 10 | 実施 OS（Windows）と CI（ubuntu-latest）の差異（改行・パス・ツール導入）を未検出のまま記録漏れ | med | low | E-6 で実施 OS を明記し、macOS/Linux ローカル手順は未検証と明記する |

## Task Breakdown

task-doc 化の単位はスライスと一致させ、1 task = 1 実装まとまり・複数 commit 可とする。推奨分割:

| 候補 task | 含むステップ | 完了条件（検証エントリ） |
| --- | --- | --- |
| T1: [Rust toolchain pin と lockfile 確定](../tasks/pin-rust-toolchain-and-lockfile.md) | A-1〜A-8 | V-1, V-2 |
| T2: [最小ワークスペースと smoke test の構築](../tasks/create-minimal-workspace-and-smoke-test.md) | B-1〜B-7 | V-3, V-4, V-10 |
| T3: [mise 開発・CI 検証タスクの定義](../tasks/define-mise-dev-and-ci-tasks.md) | C-1〜C-6 | V-5, V-6 |
| T4: [手動 CI ワークフローの追加](../tasks/add-manual-ci-workflow.md) | D-1〜D-6 | V-7, V-8 |
| T5: [再現インストール検証と #3 転記用作業メモ](../tasks/verify-reproducible-setup-and-work-memo.md) | E-1〜E-6 | V-1, V-2, V-9, V-11 |

task-doc / test-spec の作成は plan 承認後の planning-flow 後段で行う。test-spec は `../test-specs/` に作成済みで、本 plan の `relations.verified-by` にリンク済み（approved 化の監査条件を充足）。

## Verification

| Step | Verification command or criteria | Pass condition |
| --- | --- | --- |
| V-1（AC: 破壊的再初期化なし・lockfile 追跡） | `git ls-files mise.lock apm.lock.yaml`、`git diff --stat apm.yml`、`mise run setup` | lockfile 2 件が追跡済み、`apm.yml` に diff なし、setup が exit 0 |
| V-2（AC: 再現インストール） | `mise install --locked`、`mise exec -- apm install --frozen`（未対応なら記録した同等手順） | 両者 exit 0、同等手順の場合は手順と結果が作業メモに記録済み |
| V-3（AC: workspace/プロファイル構成） | workspace ルート `Cargo.toml` の `[workspace]`/`[workspace.package]`/`[workspace.lints.rust]`/`[workspace.lints.clippy]`/`[profile.dev]`/`[profile.ci]`/`[profile.ci.package."*"]` が design verbatim で存在し、`crates/ruviform-core/Cargo.toml` に `profile`・独自 lint の重複記述がなく `[lints] workspace = true` で継承していることを目視確認；`mise exec -- cargo build --workspace --profile ci` と `mise exec -- cargo build --workspace`（dev, debug=full）の両方を実行 | grep/目視で verbatim 一致・メンバー側重複なし、両 build が exit 0、`target/debug/` と `target/ci/` が分離生成される |
| V-4（AC: Cargo.lock 追跡・target 除外） | `git ls-files Cargo.lock` が空でない、`git check-ignore target/` が `target/` を出力 | 両条件成立 |
| V-5（AC: mise タスク共通化） | `mise run fmt-check && mise run check && mise run lint && mise run test && mise run build`、`mise run verify-ci` | すべて exit 0。`verify-ci` が `fmt-check`→`check-ci`→`lint-ci`→`test-ci` の順で `--profile ci`・`CARGO_INCREMENTAL=0` を適用 |
| V-6（AC: ローカル/CI 同一コマンド） | `.github/workflows/ci.yml` のステップが `mise run fmt-check|check-ci|lint-ci|test-ci` を呼び、生の cargo コマンドを直接記述していないことを目視確認 | CI 手順が mise タスク経由で一致 |
| V-7（AC: workflow_dispatch のみ・最小権限） | `ci.yml` の `on:` が `workflow_dispatch` のみで `push`/`pull_request`/`schedule` を含まない、`permissions` が `contents: read` のみであることを目視確認；`mise exec -- actionlint .github/workflows/ci.yml`（採用時）または prek `check-yaml` | 設定が条件を満たし、静的検証 exit 0 |
| V-8（AC: CI ディスク計測・増分無効） | `ci.yml` に `CARGO_INCREMENTAL: "0"`、事前/事後の `df -h`、`du -sh target` と `target/ci/deps`・`build` の計測ステップがあり、`target/` キャッシュがないことを目視確認；マージ後 Actions UI から手動ディスパッチ | 設定確認＋手動実行のログに計測値が記録される（ブランチ上は静的確認のみで可） |
| V-9（AC: 追跡/非追跡の分離） | `git status --ignored`、`git check-ignore apm_modules .env target/` | 生成物・秘密情報が非追跡、lockfile 群が追跡 |
| V-10（AC: smoke test 成功） | `mise run test` | smoke test を含め exit 0 |
| V-11（AC: #3 転記可能・未検証 OS 明記） | `docs/impl/ir/` の IR と実施 PR 本文に、セットアップ手順・検証コマンド・前提条件・lockfile 再生成条件・実施 OS/未検証 OS が記録されていることをレビュー | 記録が揃い #3 が転記可能 |

新規 clone 再現性（AC: 新規 clone で build/check/lint/test が再現）は V-1/V-2/V-5 を `git worktree` 等のクリーンな作業ツリーで再実行して確認する。

## Rollback Strategy

本計画の変更はすべて新規ファイル追加と既存ファイルへの追記であり、DB マイグレーションや公開 API 変更のような不可逆操作は含まない。各スライスのロールバック:

| 対象 | Trigger | Procedure | Verification |
| --- | --- | --- | --- |
| スライス A（mise.toml tools/lockfile） | `mise install`/`setup` が既存ツールを壊す、`cargo` 疎通不可 | `git checkout -- mise.toml mise.lock` で追記を revert し `mise install` で復元 | `mise run setup` と既存ツールのバージョン確認が成功 |
| スライス B（workspace/crate/.gitignore） | ビルド・test が解決不能に失敗 | `Cargo.toml`、`Cargo.lock`、`crates/` を削除し `.gitignore` の追記を revert | `git status` で対象ファイルが消え、既存運用に影響なし |
| スライス C（mise tasks） | タスク定義の構文・動作不備 | スライス A を維持する場合は `[tasks.*]` の追記分のみを手編集で revert する。`git checkout -- mise.toml` はスライス A の `[tools] rust` 追記も同時に revert するため、その場合は A-2〜A-5（`rust` 追記・`mise install`・`mise lock`）を再実行する | `mise tasks` が既存 `setup`（および維持したい場合は rust toolchain の疎通）を表示・利用できる |
| スライス D（ci.yml） | ワークフローが手動ディスパッチで恒常失敗・誤起動 | マージ前ならファイル削除。マージ後は `ci.yml` 削除の revert PR、または Actions UI で workflow を disable | default branch に `ci.yml` がなく、Actions に当該 workflow が active でない |
| lockfile 再生成全般 | `mise.lock`/`apm.lock.yaml`/`Cargo.lock` が不整合 | `git checkout -- <lockfile>` で追跡済み lockfile に戻し、`mise lock`/`apm lock`/`cargo generate-lockfile` を再実行 | `mise install --locked` と各検証タスクが exit 0 |

## Open Questions / Gaps

- **Rust pin バージョン**: 実装時に `mise ls-remote rust` で選定する（`minimum_release_age = "7d"` を尊重）。本 plan では具体値を仮置きに留め、選定値と根拠を A-8 で記録する
- **`apm install --frozen` 対応状況**: apm-cli 0.33.0 の対応は実機確認が必要（E-2）。未対応時の同等手順はその場で定義して記録する
- **actionlint 採否**: 推奨は採用（D-5）。mise registry で解決不能なら prek `check-yaml` のみにフォールバックし、判断を記録する
- **作業メモの正本**: 暫定開発規約・LFS 方針・検証済み OS 等の #3 転記材料は `docs/impl/ir/` の Implementation Record と実施 PR 本文に記録する（README/AGENTS 本文への反映は #3 の責務）
- **CI の実動作確認**: `workflow_dispatch` は default branch マージ後にのみ可能。ブランチ上は静的検証に留め、手動ディスパッチ結果はマージ後に確認する運用を前提とする

## Self-Review

- [x] spec の Requirements §1–§6 と Acceptance Criteria 全 11 件が Phase A–E のステップおよび V-1〜V-11 に対応している
- [x] design の verbatim ブロック（`Cargo.toml` 全体: workspace/package/lints/プロファイル）・タスク表・ワークフロー構成・暫定開発規約・Verification Notes が実装順序と検証に反映されている
- [x] `TBD`/`TODO`/`implement later` 等の placeholder は残っていない（未確定値は Open Questions / Gaps に明示）
- [x] file path・relation・status・コマンド・用語が design/spec/ADR と整合している（`ci` プロファイル・`*-ci` タスク名・`workflow_dispatch`・`contents: read` 等）

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（task-doc 化後、review checkpoint 付きで task ごとに実行）
- Independent work streams: 基本は直列（T1→T2→T3→T4→T5）。D-1〜D-3 のワークフロー記述のみ C のタスク検証と並行可能だが、同一 `mise.toml`/共有パスを考えると分離委譲の利益は小さく、単一 implementer が依存順に実行するのが自然
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 仮置きと判断が残る点: Rust pin バージョン・actionlint 採否・`apm install --frozen` 対応は実装時の実機確認に委ねており、選定根拠を作業メモに残すことを完了条件に含めている
- `relations.verified-by` は test-spec 4 件（lockfile 再現セットアップ・mise タスク共通検証・ci プロファイル・手動 CI ワークフロー）をリンク済み。task 作成時は各 task の `relations.verified-by` に対応する test-spec を記録する
- レビュー重点: workspace ルート `Cargo.toml` のブロック全体（`[workspace.lints]`・プロファイル含む）が design の verbatim 一致を逸脱していないか、`verify-ci` が逐次実行になるか、ワークフローに自動トリガーと `target/` キャッシュが混入していないか
- 上流参照: spec `SPEC-034IcwhwIFSNUNKtX7KK0Q`（Requirements/Acceptance Criteria）、design `DESIGN-034IddXFCRKDIW01WXkSla`（Components and Boundaries・Verification Notes）、ADR `ADR-034IcJdtqm6TP6MylyP5J7`（Decision Outcome/制約）、Issue #2
