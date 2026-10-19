---
id: "TSPEC-034IdHmKPCUPXNnJh1Y7f7"
type: "test-spec"
status: "proposed"
title: "lockfile による再現セットアップ"
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
  derived-by: []
  verifies:
    - "../specs/establish-rust-dev-foundation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# lockfile による再現セットアップ

## Purpose

Issue #2 の開発基盤が「新規 clone から固定 lockfile で再現できる」という意図を保護する。[spec](../specs/establish-rust-dev-foundation.md)（Requirements §1、Acceptance Criteria）と [design](../designs/dev-foundation.md)（Non-Functional Constraints: 再現性）が要求する、ツール・依存の正本（`mise.toml` / `apm.yml` / `Cargo.toml`）と確定値（`mise.lock` / `apm.lock.yaml` / `Cargo.lock`）の分離契約を固定する。将来ツール追加やバージョン更新でこの分離が崩れたとき、本 test spec が「再現性は契約である」ことを思い出させる。廃止してよいのは、lockfile 追跡運用をやめる方針が ADR で明示的に採用されたとき。

## Feature

固定 lockfile からの再現セットアップ: 新規 clone が、記録された前提条件（ネットワーク・権限・認証）の範囲で、検証可能な状態まで再現できること。

## Rules

- ツールと依存の確定値は追跡対象の lockfile（`mise.lock` / `apm.lock.yaml` / `Cargo.lock`）にのみ存在し、正本ファイルと分離される。lockfile はすべて Git 追跡対象。
- 復元経路は `mise install --locked` と、使用中 APM CLI が対応する lock 検証／frozen install（`apm install --frozen`）で成立する。`apm install --frozen` が未対応の場合は、同等の再現性検証手順が定義され記録されていること。浮動バージョン解決への暗黙の依存は許容しない。
- 既存の正本ファイルを破壊的に再初期化しない。`apm init --yes` や `cargo init` 等の初期化・雛形コマンドは対象が未作成の場合に限り許容され、既存 `mise.toml` / `apm.yml` を上書きする実行は行わない。
- lockfile の再生成条件と、初回実行に必要な前提（ネットワーク・権限・認証）が作業メモに記録されている。
- 生成物・秘密情報と追跡対象が分離される（`target/`・`apm_modules/`・`.env*` 等は非追跡、lockfile 群は追跡）。

## Examples

- Given: 追跡済みの `mise.lock` / `apm.lock.yaml` がある。When: クリーンな作業ツリー（新規 clone 相当）で `mise install --locked` と APM の再現インストールを実行する。Then: `cargo` / `rustfmt` / `clippy` を含む toolchain が復元され、両者とも exit 0 となる。
- Given: `apm install --frozen` が使用 CLI で未対応。When: 復元検証を実施する。Then: 失敗を放置せず、`apm.lock.yaml` 差分なし確認等の同等手順が定義され、手順と結果が作業メモに記録されている。
- Given: workspace 構築後の作業ツリー。When: `git status --ignored` / `git check-ignore` / `git ls-files` で確認する。Then: `Cargo.lock`・`mise.lock`・`apm.lock.yaml` が追跡対象であり、`target/`・`apm_modules/`・`.env*`・Serena ローカル状態が非追跡である。
- Given: `mise.lock` が不整合になった。When: 追跡済み lockfile に戻して `mise lock` を再実行する。Then: `mise install --locked` が再び exit 0 となる復旧経路が成立する。

## Guarantees

- 新規 clone から、記載した前提条件の範囲でセットアップと toolchain が再現でき、ローカルと CI のツールバージョン差異が発生しない。
- lockfile の不整合に対して、追跡済みファイルへ戻して各 lock コマンドを再実行する復旧経路が存在することが記録されている。
- 既存の mise／APM／prek／Git LFS 運用が破壊的再初期化で失われない。

## Non-goals

- lockfile 内の具体的バージョン値の選択正当性は検証しない（選定根拠の記録は plan のステップと IR の責務）。
- 実施した OS 以外での再現性は保証しない。未検証 OS が明記されることのみ要求する。
- CI ランナー上での lockfile 復元の実行は `manual-ci-workflow` 側の関心事とし、ここでは扱わない。
- APM／Serena の生成物の中身や機能正当性は検証しない（追跡／非追跡の分離のみ）。

## Risk

- lockfile が未生成・非追跡になると、新規 clone と CI の toolchain が浮動解決になり「自分の環境では動く」状態に戻る。再現性は Issue #2 の最優先要件である。
- 破壊的再初期化の許容は、正本である既存 `mise.toml` / `apm.yml` の内容を失わせ、既存 APM／prek／LFS 運用を壊す。
- 復元手順・前提条件の記録欠落は #3 による README／AGENTS への転記を不可能にし、Issue #2 を完了状態にできない。
