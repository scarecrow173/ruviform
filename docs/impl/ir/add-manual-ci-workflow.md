---
id: "IMPL-034IemQE6RlhuBXtirqKVX"
type: "impl"
status: "completed"
title: "Add manual CI workflow"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci.yml"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "mise.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/add-manual-ci-workflow.md"
  implemented-by: []
  depends-on:
    - "define-mise-dev-and-ci-tasks.md"
  blocks:
    - "../../tasks/verify-reproducible-setup-and-work-memo.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/dev-foundation-bootstrap.md"
    - "../../designs/dev-foundation.md"
    - "../../adr/rust-dev-foundation-toolchain-and-ci-profile.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/manual-ci-workflow.md"
    - "../../test-specs/ci-profile-build-behavior.md"
    - "../../test-specs/mise-verification-tasks.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add manual CI workflow

## Summary

task `TASK-034IdQtDnY6ZKHFhFhXKmW`（plan `PLAN-034IdrispOUms1c32192B6` スライス D / Phase D-1〜D-6）として、`workflow_dispatch` 専用の GitHub Actions 検証ワークフロー `.github/workflows/ci.yml` を新規作成した。`permissions: contents: read`、`ubuntu-latest` 単一 job、job env `CARGO_INCREMENTAL: "0"` で、checkout → 事前 `df -h` → `jdx/mise-action@v5.2.0` + `mise install --locked` → cargo registry/git index キャッシュ（`Cargo.lock` ハッシュキー）→ `mise exec -- prek run --all-files` → `mise run fmt-check` → `check-ci` → `lint-ci` → `test-ci` → 事後計測（`df -h` / `du -sh target` / `target/ci/deps`・`build`）の design 定義どおりのステップ列を、すべて明示的な `name` 付きで記述した。actionlint は推奨どおり採用し `mise.toml` `[tools]` に `actionlint = "1.7.12"` を pin、`mise.lock` に 7 platform エントリを追記した。静的検証（prek `check-yaml`・`actionlint`）はすべて exit 0。実動作は `workflow_dispatch` の default branch 前提制約により未検証であり、マージ後の手動ディスパッチで確認する。

## Context

- 実施タスク: `docs/tasks/add-manual-ci-workflow.md`（D-1〜D-6）
- 上流: `docs/plans/dev-foundation-bootstrap.md`（Phase D / V-6・V-7・V-8）、`docs/designs/dev-foundation.md`（「GitHub Actions ワークフロー設計」節: トリガー・権限・ステップ構成・キャッシュ方針・導入手順）、ADR `ADR-034IcJdtqm6TP6MylyP5J7`（`workflow_dispatch` 限定）、test-spec `TSPEC-034IdHt7P0ega4N0bbNANg`（手動専用・最小権限・計測の契約）・`TSPEC-034IdHsItEuo0TZWTPOJUu`（`--profile ci`・`CARGO_INCREMENTAL=0`）・`TSPEC-034IdHshzpNiPTqyyvwL6J`（mise タスク経由の同一コマンド）
- 前提: スライス C（`IMPL-034IedmXdoFo5aI22WR8m8`）で `fmt-check` / `check-ci` / `lint-ci` / `test-ci` / `build-ci` / `verify-ci` のタスク名が確定・実機検証済み。ワークフローはこれらを `mise run` 経由で呼ぶのみで、生の cargo コマンドを含まない。

## Implementation

### Changes

- `.github/workflows/ci.yml` — 新規作成（`.github/workflows/` ディレクトリ新設）。
  - `on: workflow_dispatch:` のみ。入力項目・`push` / `pull_request` / `schedule` 等の自動トリガーなし。
  - `permissions: contents: read` のみ。
  - `jobs.verify`（`ubuntu-latest`、matrix なし）に `env: { CARGO_INCREMENTAL: "0" }`。
  - ステップ（全て `name` 付き、design の順序どおり）:
    1. `actions/checkout@v7.0.1`
    2. `df -h`（事前計測）
    3. `jdx/mise-action@v5.2.0`（フルバージョン pin）
    4. `mise trust` + `mise install --locked`（`mise.lock` から toolchain 復元）
    5. `actions/cache@v6.1.0` で `~/.cargo/registry/index/`・`~/.cargo/registry/cache/`・`~/.cargo/git/db/` を `key: ${{ runner.os }}-cargo-${{ hashFiles('Cargo.lock') }}`・`restore-keys: ${{ runner.os }}-cargo-` でキャッシュ
    6. `mise exec -- prek run --all-files`（構成ファイル検証）
    7. `mise run fmt-check`
    8. `mise run check-ci`
    9. `mise run lint-ci`
    10. `mise run test-ci`
    11. 事後計測 `df -h`・`du -sh target`・`du -sh target/ci/deps target/ci/build 2>/dev/null || true`（存在する範囲で）
- `mise.toml` — `[tools]` に `actionlint = "1.7.12"` を 1 行追記（既存エントリ不変。追記のみ +1 行）。
- `mise.lock` — `mise lock` で `[[tools.actionlint]]` の 7 platform エントリを追記（diff は追加のみ 166 行、削除・変更なし）。

### Decisions During Implementation

- **cargo キャッシュ: 採用（design が許容する限定形のみ）**。`target/` 全体・中間成果物はキャッシュしない。`~/.cargo` の registry index・registry cache（.crate アーカイブ）・git db のみを `Cargo.lock` ハッシュでキー付けした `actions/cache` を追加した。手動実行のみ・頻度が低いワークフローでも、依存ダウンロードの再実行コストを抑えられる。`restore-keys` に `${{ runner.os }}-cargo-` プレフィックスを付け、`Cargo.lock` 変更時も部分ヒットで index 取得を再利用できるようにした（design の例示キーをそのまま使用）。
- **actionlint: 採用**。design / plan の推奨どおり、手動ディスパッチ前にブランチ上で検出可能な唯一の意味論検証として採用した。`mise registry actionlint` が `aqua:rhysd/actionlint` を解決し、`mise ls-remote actionlint` の最新 `1.7.12` を pin した（`minimum_release_age = "7d"` 下でも明示 pin はインストール可能であることを実機確認）。CI 側は `mise install --locked` で actionlint も復元されるが、ワークフロー本体では prek までしか使わず actionlint 自体は呼ばない（lint はローカル／別ジョブ運用の範囲外）。
- **`mise trust` を復元ステップに同梱**。GitHub Actions の非対話環境ではリポジトリの `mise.toml` が未信頼としてブロックされ得るため、`mise install --locked` の前に `mise trust` を実行する（冪等・害なし）。`jdx/mise-action` が信頼処理を内包していても安全側の明示とした。
- **action の pin はフルバージョンタグ**。plan Risk 4 の緩和策どおり `actions/checkout@v7.0.1`・`jdx/mise-action@v5.2.0`・`actions/cache@v6.1.0` を GitHub API の latest release タグで pin した（2026-10-19 時点実測）。
- **事後計測の `du` は `2>/dev/null || true` で防御**。`target/ci/deps`・`target/ci/build` は test/check の実行内容次第で存在しない可能性があり、「存在する範囲で計測」の契約を満たしつつステップを失敗させないため。
- **`mise lock` の既知 caveat（既存条件）**: `mise lock` は全 platform エントリを書き込んだ後 `mise ERROR pipx:apm-cli dependency locking does not support uvx_args or pipx_args` で exit 1 となる。原因は既存の `"pipx:apm-cli" = { uvx_args = "--prerelease allow" }` であり、スライス A の IR `IMPL-034IdUNIqDaqtMaA6rRfEx` に記録済みの既存条件。本変更は `mise.lock` への actionlint 追記（追加のみ）を正常に完了させており退行ではない。
- **workflow_dispatch の default branch 前提制約**: 本ワークフローは default branch に存在しないと Actions UI からディスパッチできない。ブランチ上の検証は静的検証（prek `check-yaml`・`actionlint`）に留まるのが意図した運用（design「導入・検証手順」・plan Risk 2）。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64、mise 2026.10.1、actionlint 1.7.12。task の `## Verification` と plan V-7・V-8 の静的検証部分を実機実行した結果:

| Check | コマンド | 結果 |
| --- | --- | --- |
| YAML 構文 | `mise exec -- prek run --all-files` | exit 0。`check yaml` を含む全フック Passed（`ci.yml` も対象として検証済み） |
| 意味論検証 | `mise exec -- actionlint .github/workflows/ci.yml` | exit 0（警告・エラーなし） |
| 手動トリガー | `grep -n 'workflow_dispatch' .github/workflows/ci.yml` | 行 7 に一致 |
| 自動トリガーなし | `grep -nE '(push|pull_request|schedule):' .github/workflows/ci.yml` | exit 1（一致なし） |
| 最小権限 | `grep -n 'contents: read' .github/workflows/ci.yml` | 行 10 に一致 |
| 増分無効 | `grep -n 'CARGO_INCREMENTAL' .github/workflows/ci.yml` | 行 17 に一致（job env） |
| mise タスク経由 | `grep -cE 'mise run (fmt-check|check-ci|lint-ci|test-ci)' .github/workflows/ci.yml` | `8`（name + run 各行、閾値 4 以上） |
| 生 cargo なし | `grep -nE 'cargo (fmt|clippy|test|check|build)' .github/workflows/ci.yml` | exit 1（一致なし。`~/.cargo/...`・`cargo-${{ ... }}` キーは `cargo ` 空白+サブコマンド形に非一致） |
| ディスク計測 | `grep -n 'df -h' .github/workflows/ci.yml` | 行 23（事前）・行 60（事後）の 2 箇所 |
| actionlint 導入 | `mise install`（`actionlint = "1.7.12"`）→ `mise lock` | インストール成功・`mise.lock` に 7 platform エントリ追記（diff は追加のみ）。`mise lock` は既知の `pipx:apm-cli` uvx_args エラーで exit 1（既存条件、スライス A IR 記録済み） |
| 目視 | `on:` ブロック・入力なし・単一 job・全ステップ `name` 付き | `on: workflow_dispatch:` のみ、inputs なし、`jobs.verify` 単一、11 ステップすべて `name` あり |

未検証（設計上ブランチでは不可能）: ワークフローの実動作。`workflow_dispatch` は default branch にファイルが存在することが前提のため、本 PR マージ後に確認する。

**マージ後の手動ディスパッチ手順**（記録義務より明記）:

1. 本変更が default branch にマージされたことを確認する（`git ls-remote` や GitHub 上のファイル存在確認）。
2. GitHub のリポジトリページ → **Actions** タブ → 左ナビの **ci** ワークフローを選択。
3. **Run workflow** ボタンから対象 ref（通常 default branch）を選び、入力なしで実行。
4. 実行ログで以下を確認する: `mise install --locked` による toolchain 復元、`prek run --all-files` 通過、`fmt-check` / `check-ci` / `lint-ci` / `test-ci` の exit 0、事前・事後の `df -h` と `du -sh target`（および `target/ci/deps`・`target/ci/build`）の計測値がログに記録されていること。
5. 計測値と結果を Phase E（`verify-reproducible-setup-and-work-memo.md`）の作業メモに転記する。

## Risks

- ワークフローの実動作は未検証（静的検証のみで代替するのが設計方針）。マージ後ディスパッチで初めて判明する失敗（runner 環境差異・`mise trust` 挙動・action バージョンの runner 互換性等）の可能性が残る。
- action pin は GitHub latest release の実測値であり、マージ時点でより新しいタグが出ている可能性がある。pin 自体は design の緩和策どおりであり、更新は別タスク判断。
- cargo キャッシュは手動実行頻度が低いため効果は限定的だが、害はなく design が許容する形に限定済み。

## Follow-ups

- スライス E（`verify-reproducible-setup-and-work-memo.md`）で以下を引き継ぐ: 本ワークフローの default branch 前提制約と手動ディスパッチ手順、cargo キャッシュ・actionlint 採用の判断、`mise lock` の `pipx:apm-cli` uvx_args 既知 caveat、実施 OS（Windows）と CI（ubuntu-latest）の差異。
- `target/` の無条件キャッシュを今後追加しないこと（design のキャッシュ方針）。自動トリガー導入は別 Issue の明示的方針変更として扱う。
- `ci.yml`・`mise.toml`/`mise.lock` 差分・本 IR・task 更新のコミットは orchestrator が実施する。
