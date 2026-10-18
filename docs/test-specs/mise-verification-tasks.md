---
id: "TSPEC-034IdHshzpNiPTqyyvwL6J"
type: "test-spec"
status: "proposed"
title: "mise タスクによるローカル・CI 共通検証"
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

# mise タスクによるローカル・CI 共通検証

## Purpose

検証コマンドの入口を `mise.toml` タスクに一本化し、ローカルと CI が同一コマンド・同一ツールバージョンで検証するという意図を保護する。[spec](../specs/establish-rust-dev-foundation.md)（Requirements §4）と [design](../designs/dev-foundation.md)（mise タスク設計）の契約を固定し、将来のタスク追加・分割・CI 変更で入口が分散したり、CI が生の cargo コマンドを直接呼ぶ設計に戻ったりしないようにする。廃止してよいのは、タスクランナー方針を変える ADR が明示的に採用されたとき。

## Feature

mise タスクによるローカル・CI 共通検証: `fmt` / `fmt-check` / `check` / `lint` / `test` / `build` と CI 向け `*-ci`・`verify-ci` が mise タスクとして定義され、ローカルと CI が同じ入口を共有すること。

## Rules

- 検証入口は `mise.toml` のタスクに一本化される。just／Makefile 等の別タスクランナーや、CI ワークフロー内での生の cargo コマンドの直接記述を併設しない。
- ローカル通常タスクは `dev` / `test` プロファイル（フルデバッグ情報を維持）、CI 向け `*-ci` タスクは `--profile ci` と `env = { CARGO_INCREMENTAL = "0" }` を使う。検証内容（対象 workspace・`--all-targets`・`-D warnings`）は `*-ci` 系で重複定義せず、同一コマンドのプロファイル差分として扱う。
- 各タスクは `description` で用途・実行コマンド・副作用を明示する。`fmt` はソースファイルを書き換える整形タスク、`fmt-check` / `check` / `lint` / `test` / `build` 系は非破壊で `target/` への成果物生成のみ、という区別が読み取れる。
- `verify-ci` は `fmt-check` → `check-ci` → `lint-ci` → `test-ci` を順次実行する集約入口であり、同じ `CARGO_INCREMENTAL=0` を伝播させる。
- prek の軽量フック（YAML／TOML／改行／秘密鍵チェック）と cargo の重い検証は分担し、コミットフックに cargo 系検証を追加しない。
- `cargo fmt` はプロファイル非依存のため `fmt-ci` 相当の `*-ci` タスクを設けない。

## Examples

- Given: 最小 workspace が構築済み。When: `mise run fmt-check`・`check`・`lint`・`test`・`build` を順に実行する。Then: すべて exit 0 で、ツールは `mise.lock` 固定のバージョンが使われる。
- Given: `verify-ci` が定義済み。When: `mise run verify-ci` を実行する。Then: `fmt-check` → `check-ci` → `lint-ci` → `test-ci` の順に、`--profile ci`・`CARGO_INCREMENTAL=0` の条件で逐次実行される。
- Given: CI ワークフロー定義。When: 検証ステップを確認する。Then: `mise run fmt-check|check-ci|lint-ci|test-ci` を経由する呼び出しのみで、生の `cargo fmt` / `cargo clippy` / `cargo test` が直接記述されていない。
- Given: 局所許可のない `unsafe` を含むコード。When: `mise run lint` または `lint-ci` を実行する。Then: `workspace.lints` の `unsafe_code = "warn"` が `-D warnings` ゲートで検証を通過させない。

## Guarantees

- ローカル開発者と CI が同一の検証コマンドと `mise.lock` 固定のツールバージョンで検証でき、「ローカルだけ通る／CI だけ落ちる」の乖離が生じない。
- `lint` / `lint-ci` の `-D warnings` ゲートにより、`workspace.lints` ポリシー違反（局所許可のない `unsafe` を含む）が検証を通過しない。
- プロジェクト実装が未着手でも、最小 workspace と smoke test で全タスクが成功する（デバイス非依存）。

## Non-goals

- タスクの実行速度・キャッシュ効率・並列実行の最適化は検証しない。
- `cargo-nextest` / `cargo-llvm-cov` / ベンチマーク補助ツールの有無は扱わない（spec の Out of Scope）。
- 個々のタスクの具体的コマンド定義と実行確認は plan の Verification Matrix と task の `## Verification` の責務であり、ここでは入口一本化とプロファイル分離の不変条件のみを保証する。
- `ci` プロファイル自体の意味論（`debug` 値・`incremental`・成果物分離）は `ci-profile-build-behavior` 側の関心事。

## Risk

- 検証入口が分散すると CI とローカルの検証が静かに乖離し、不一致の原因追跡が困難になる。
- コミットフックに cargo 検証を入れると、コミット毎の重いビルドが開発フローを阻害する。
- `description`・副作用の未明記は、`fmt` のような書き換え系タスクと非破壊チェックの区別を曖昧にし、意図しないファイル変更や検証漏れを招く。
- ローカルで `*-ci` を常用すると `target/ci/` と `target/debug/` が並存して容量を圧迫するため、CI 用途限定の運用が記録される必要がある。
