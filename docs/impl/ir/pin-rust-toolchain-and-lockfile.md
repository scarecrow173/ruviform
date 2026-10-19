---
id: "IMPL-034IdUNIqDaqtMaA6rRfEx"
type: "impl"
status: "completed"
title: "Pin Rust toolchain and lockfile"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added: []
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
    - "../../tasks/pin-rust-toolchain-and-lockfile.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/dev-foundation-bootstrap.md"
    - "../../designs/dev-foundation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/lockfile-reproducible-setup.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Pin Rust toolchain and lockfile

## Summary

task `TASK-034IdQiK2RW8ol4gpyUYLb`（plan `PLAN-034IdrispOUms1c32192B6` スライス A）として、`mise.toml` の `[tools]` に Rust toolchain を具体 stable バージョン `1.99.0` で pin し、`mise install` → `mise lock` で `mise.lock` に確定値を記録した。既存ツール・`[settings]`・`[env]`・`[tasks.setup]` は変更していない。

## Context

- 実施タスク: `docs/tasks/pin-rust-toolchain-and-lockfile.md`（A-1〜A-8）
- 上流: `docs/plans/dev-foundation-bootstrap.md`（Phase A）、`docs/designs/dev-foundation.md`（toolchain pin 方針: `rust-toolchain.toml` 併用禁止・lockfile 運用）、`docs/test-specs/lockfile-reproducible-setup.md`（再現セットアップ契約）
- 前提制約: `mise.toml` `[settings]` の `minimum_release_age = "7d"` を尊重し、浮動指定（`stable`/`latest`）は禁止。design の仮置き MSRV `1.85` 以上であること。

## Implementation

### Changes

- `mise.toml` — `[tools]` に `rust = "1.99.0"` を末尾へ追記（既存エントリ `[settings]`/`[env]`/`[tasks.setup]` 不変）
- `mise.lock` — `mise lock` で rust エントリを確定（追跡継続）

### Decisions During Implementation

- **選定バージョン: `1.99.0`**。`mise ls-remote rust`（2026-10-19 実施、Windows x64 / mise 2026.10.1）では `minimum_release_age = "7d"` により 2 件のより新しいリリースが隠され、表示される最新 stable は `1.99.0` であった。根拠: (1) stable 系の最新かつ 7 日経過済み、(2) design 仮置き MSRV `1.85` を満たす、(3) 浮動指定ではない具体バージョン。patch まで完全 pin としたのは「具体バージョン」要件を曖昧さなく満たし、`mise.lock` の確定値と二重のずれを生じさせないため。#1 で edition/MSRV とあわせ再評価される仮置き値。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64、mise 2026.10.1（windows-x64）。task の `## Verification` 全項目を実機で実行した結果:

| Check | コマンド | 結果 |
| --- | --- | --- |
| cargo 疎通 | `mise exec -- cargo --version` | exit 0 → `cargo 1.99.0 (5f94df478 2026-08-27)` |
| rustfmt 疎通 | `mise exec -- cargo fmt --version` | exit 0 → `rustfmt 1.10.0-stable (b940084d7e 2026-09-28)` |
| clippy 疎通 | `mise exec -- cargo clippy --version` | exit 0 → `clippy 0.1.99 (b940084d7e 2026-09-28)` |
| lockfile 追跡 | `git ls-files mise.lock apm.lock.yaml` | exit 0、両ファイルを出力（`apm.lock.yaml` / `mise.lock`） |
| apm.yml 無差分 | `git diff --exit-code -- apm.yml` | exit 0（差分なし） |
| 既存運用の無破壊 | `mise run setup` | exit 0（`apm install` 9 deps 成功、`prek install`、`git lfs install --local` いずれも成功） |
| 具体バージョン pin | `grep -n 'rust = "' mise.toml` | `15:rust = "1.99.0"`（`stable`/`latest` ではない） |
| mise.lock rust エントリ | `git diff mise.lock` / 目視 | `[[tools.rust]] version = "1.99.0" backend = "core:rust"` が記録済み |

補助的に実施した追加確認:

- `mise ls-remote rust` — `1.99.0` が表示される最新 stable。末尾に `mise WARN 2 newer rust releases hidden by minimum_release_age` が出力され、`minimum_release_age = "7d"` のフィルタが効いていることを確認。
- `mise install` — exit 0、`rust@1.99.0 already installed`（13 tools すべて成功）。
- `mise exec -- rustc --version` — exit 0 → `rustc 1.99.0 (b940084d7 2026-09-28)`。
- `mise exec -- apm lock` — exit 0、`Lockfile written to apm.lock.yaml`。`git status --porcelain -- apm.yml apm.lock.yaml` は空（`apm.yml`/`apm.lock.yaml` ともに無差分）。
- rustup default プロファイルで `cargo`/`rustfmt`/`clippy` が同梱されていたため、component 追加手順を `setup` タスクへ追記する必要はなかった（design の想定どおり）。

判明した caveat（いずれも本タスクの変更とは無関係な既存条件）:

- `mise lock` は rust エントリを含む全ツールのエントリを `mise.lock` に書き込んだ後、`mise ERROR pipx:apm-cli dependency locking does not support uvx_args or pipx_args` で exit 1 となった。原因は既存の `"pipx:apm-cli" = { uvx_args = "--prerelease allow" }` 設定であり、HEAD 時点の `mise.lock` も同じ部分生成形（platform セクションなし）だったため本変更による退行ではない。
- `mise install --locked` はこの環境では exit 1。失敗したのは global mise config（`C:\Users\akiyoshi\.config\mise\config.toml`）由来のツール（`ripgrep@latest`/`ffmpeg@latest`/`python@3.13`/`node@26`/`pnpm@latest`/`npm:@openai/codex@latest`/`npm:ccusage@latest` が lockfile に無い）と、その依存失敗に連鎖した `pipx:*` 2 件であり、repo 管理の `rust@1.99.0` は正常に解決された（`already installed`）。クリーン環境（global config 非存在）での `--locked` 復元は Phase E（`verify-reproducible-setup-and-work-memo.md`）の検証範囲。

`git status --porcelain` の変更は `mise.toml`・`mise.lock`・本 task・本 IR のみで、意図しない生成物の差分はない。

## Risks

- 実施 OS は Windows x64 のみ。macOS/Linux での `mise install --locked` 復元は未検証（後続タスク `verify-reproducible-setup-and-work-memo.md` でクリーン作業ツリー検証を実施予定）。
- `mise` 自体は 2026.10.1（2026.10.8 が利用可能と警告あり）を使用。
- 上記 `mise lock` / `mise install --locked` の caveat は既存条件だが、Phase E の V-2 検証（`mise install --locked` exit 0）に影響し得るため記録した。platform セクションを持たない `mise.lock` での復元可否は Phase E で実機確認が必要。

## Follow-ups

- `mise.lock` を含む復元検証（`mise install --locked`、新規 clone 相当の worktree での再現）は plan の Phase E（`verify-reproducible-setup-and-work-memo.md`）で実施する。
- Rust バージョン・edition・MSRV の確定は #1 の設計対象（design の Deferred Design Concerns）。
