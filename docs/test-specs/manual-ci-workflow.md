---
id: "TSPEC-034IdHt7P0ega4N0bbNANg"
type: "test-spec"
status: "proposed"
title: "手動ディスパッチ専用の CI ワークフロー"
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
  implements: []
  implemented-by: []
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
  verifies:
    - "../specs/establish-rust-dev-foundation.md"
    - "../adr/rust-dev-foundation-toolchain-and-ci-profile.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 手動ディスパッチ専用の CI ワークフロー

## Purpose

「初期段階の CI は原則 `workflow_dispatch` のみ・最小権限で起動する」という運用方針の意図を保護する。[spec](../specs/establish-rust-dev-foundation.md)（Requirements §5）と [ADR](../adr/rust-dev-foundation-toolchain-and-ci-profile.md)（CI 起動方針の決定）が定める契約を固定する。基盤未整備期に自動トリガーが恒常的な失敗・Pending・未検証マージ圧力を生むことを避ける決定であり、自動トリガーの導入は別 Issue の明示的方針変更として扱う。廃止してよいのは、自動トリガー導入を決める Issue／ADR が本ワークフロー方針を置き換えたとき。

## Feature

手動ディスパッチ専用の CI ワークフロー: `.github/workflows/ci.yml` が `workflow_dispatch` のみで起動し、`contents: read` の最小権限で `mise.lock` 固定の toolchain からローカルと同一の検証を実行すること。

## Rules

- `on:` のトリガーは `workflow_dispatch` のみとし、`push` / `pull_request` / `schedule` 等の自動トリガーを設定しない。入力項目は最小限とし、入力なしで全チェックを実行できる構成を優先する。
- `permissions` は `contents: read` のみの最小権限とする。
- 実行環境は `ubuntu-latest` の単一 job とし、matrix を張らない。job env に `CARGO_INCREMENTAL: "0"` を明示する。
- toolchain は `mise install --locked` で `mise.lock` から復元し、検証は mise タスク（`fmt-check` / `check-ci` / `lint-ci` / `test-ci`、必要時 `build-ci`）経由でローカルと同一コマンドを使う。
- ビルド前後に `df -h`、`du -sh target`（`target/ci/deps`・`target/ci/build` は存在する範囲で）を実行し、計測値をログに記録する。同一 job で `dev` と `ci` を二重ビルドしない。
- `target/` 全体や中間成果物を無条件にキャッシュ／アップロードしない。許容するキャッシュは `~/.cargo` の registry/git index を `Cargo.lock` ハッシュでキー付けしたものに限定する。
- `workflow_dispatch` は default branch にワークフローが存在することが前提となるため、ブランチ上では静的検証（prek `check-yaml`、採用時 `actionlint`）に留め、初回の実動作確認は default branch マージ後の手動ディスパッチで行う運用が明記される。

## Examples

- Given: `.github/workflows/ci.yml` が存在する。When: `on:` と `permissions:` を確認する。Then: `workflow_dispatch` のみで `push` / `pull_request` / `schedule` を含まず、`contents: read` のみである。
- Given: default branch にマージ済みのワークフロー。When: Actions UI から入力なしで手動ディスパッチする。Then: checkout → 事前 `df -h` → `mise install --locked` → `prek run --all-files` → `fmt-check` → `check-ci` → `lint-ci` → `test-ci` → 事後計測の順に実行され、`df -h` / `du -sh target` の実測値がログに残る。
- Given: ブランチ上のワークフロー変更。When: マージ前に検証する。Then: `workflow_dispatch` できない制約により `check-yaml` / `actionlint` の静的検証に留まり、それが意図した運用として記録されている。
- Given: キャッシュ設定の検討。When: ワークフローのキャッシュを確認する。Then: `target/` 全体・中間成果物のキャッシュはなく、`~/.cargo` の registry/git index を `Cargo.lock` ハッシュでキー付けしたキャッシュのみが許容される（または不採用の理由が記録される）。

## Guarantees

- push／PR で CI が自動実行されないことが設定上確認でき、手動 CI を実行しなくても PR が「自動検証済み」にならない。
- コード未整備の初期段階でも、各ステップの明示的な `name` と実行コマンドにより、失敗時の原因がログから追跡できる。
- 実行対象 ref・依存バージョン（`mise.lock` 固定）・結果が追跡可能である。

## Non-goals

- ワークフローの実動作結果（ディスパッチ後の成功・失敗そのもの）はここでは保証しない。実行は default branch マージ後に可能となり、結果は IR と実施 PR の記録対象である。
- macOS／Windows の CI matrix、自動トリガー、branch protection の必須チェック設定は扱わない（別 Issue の方針決定）。
- ワークフロー YAML の構文正当性そのものは prek `check-yaml`・`actionlint` の静的検証の責務であり、本 test spec は方針特性の契約を保証する。
- `ci` プロファイルの意味論（`debug` 値・成果物分離）は `ci-profile-build-behavior` 側の関心事。

## Risk

- 自動トリガーが混入すると、基盤未整備期に恒常的な失敗・Pending・未検証マージ圧力が生じ、「原則手動」という方針を黙って逸脱する。
- `contents: read` を超える権限や `target/` の無制限キャッシュは、最小権限・容量抑制の方針を侵食する。
- ディスク計測の欠落は容量問題発生時の調査材料を失わせ、「設定変更だけで枯渇が解消した」と断定する誤判断を招く（cheaper PR #109 の教訓）。
- ローカルと異なる検証手順が CI に書かれると、手動ディスパッチの結果がローカル検証と整合せず、検証の信頼性が失われる。
