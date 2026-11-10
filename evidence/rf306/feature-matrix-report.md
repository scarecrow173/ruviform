# RF306 slice-4 実行証拠レポート — IO feature-matrix lane 実測

対象: Issue #306（IO-IMP-12）4 本目の slice。**コード・テスト・
設定ファイルを一切変更しない検証・証拠 slice** であり、scoped
feature-matrix lane（`ruviform-io-compression` /
`ruviform-io-structured` / `ruviform-io-e2e`）の実測記録を行う。
本レポートの Pass は当該 headless lane の実走結果であり、実 OS /
consumer 受入を意味しない（後述の NotRun 節を参照）。

作業 worktree: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf306-io-matrix`
branch: `feature/306-io-e2e-feature-matrix`
base: `origin/main` = `4d41b021014dff94e542e699097187b86e332cec`
（branch 起点。完了時点で `git fetch origin main && git merge
origin/main` を実施し "Already up to date" — 追加 merge 不要を確認）
環境: Windows 11 x64 (MINGW64_NT-10.0-26300) / rustc 1.99.0
(b940084d7 2026-09-28) / cargo 1.99.0 / NTFS
実行日時: 2026-11-10 (UTC)

## Feature-matrix lane 実測結果（全て実走・実値）

全コマンドは `mise run <task>` 経由（canonical = mise タスク）。
exit code は `mise run` 自体の終了コードを記録。workspace 系
lane は先行 evidence と同じく `CARGO_BUILD_JOBS=2` を付与。

| lane | コマンド | 結果 | 備考 |
| --- | --- | --- | --- |
| check-io-compression | `mise run check-io-compression` | Pass (exit 0) | default / `gzip` / `deflate` / `zstd` / `gzip,deflate,zstd` の 5 組合せを `cargo check -p ruviform-io-compression --all-targets` で連続実走（`&&` chain、全段 `Finished`） |
| lint-io-compression | `mise run lint-io-compression` | Pass (exit 0) | 同 5 組合せを `cargo clippy --all-targets -- -D warnings` で実走 |
| test-io-compression | `mise run test-io-compression` | Pass (exit 0) | 同 5 組合せを `cargo test` で実走。**合計 264 passed / 0 failed / 0 ignored**（45 test result 行 = 9 行 × 5 組合せ） |
| check-io-structured | `mise run check-io-structured` | Pass (exit 0) | default / `xml` / `xml,file` / `--all-features` の 4 組合せを `cargo check --all-targets` で実走 |
| lint-io-structured | `mise run lint-io-structured` | Pass (exit 0) | 同 4 組合せを `cargo clippy --all-targets -- -D warnings` で実走 |
| test-io-structured | `mise run test-io-structured` | Pass (exit 0) | 同 4 組合せを `cargo test` で実走。**合計 251 passed / 0 failed / 0 ignored**（16 test result 行 = 4 行 × 4 組合せ） |
| check-io-e2e | `mise run check-io-e2e` | Pass (exit 0) | `cargo check -p ruviform-io-e2e --all-targets` |
| lint-io-e2e | `mise run lint-io-e2e` | Pass (exit 0) | `cargo clippy -p ruviform-io-e2e --all-targets -- -D warnings` |
| test-io-e2e | `mise run test-io-e2e` | Pass (exit 0) | `cargo test -p ruviform-io-e2e`。**77 passed / 0 failed / 0 ignored**（14 test result 行: lib 0 + 統合 12 binary + doctest 0） |

### `test-io-compression` 組合せ別内訳（実測）

各組合せは独立した `cargo test` 起動のため feature unify は
組合せ間で発生しない。format 別 test binary
（`gzip_format` / `deflate_format` / `zstd_format`）は対応
feature 無効時に 0 tests となることを実測で確認。

| feature 組合せ | passed | failed | ignored | 主な内訳 |
| --- | --- | --- | --- | --- |
| default（feature なし） | 3 | 0 | 0 | doctest 3 のみ。format 系 test は全て feature gate で 0 |
| `gzip` | 52 | 0 | 0 | gzip_format 20・encoder_contract 11・decoder_contract 12・api_surface 4・doctest 5 |
| `deflate` | 26 | 0 | 0 | deflate_format 11・decoder_contract 3・encoder_contract 3・api_surface 4・doctest 5 |
| `zstd` | 58 | 0 | 0 | zstd_format 34・encoder_contract 9・decoder_contract 6・api_surface 4・doctest 5 |
| `gzip,deflate,zstd` | 125 | 0 | 0 | zstd_format 34・gzip_format 20・deflate_format 11・encoder_contract 23・decoder_contract 21・composition 3・api_surface 4・doctest 9 |

format 別 test 数（gzip 20 / deflate 11 / zstd 34）は slice-3
`test-io-msrv` lane の実測値と一致。

### `test-io-structured` 組合せ別内訳（実測）

| feature 組合せ | passed | failed | ignored | 主な内訳 |
| --- | --- | --- | --- | --- |
| default | 5 | 0 | 0 | api_surface 2・structured 0・doctest 3 |
| `xml` | 47 | 0 | 0 | structured 40・api_surface 3・doctest 4 |
| `xml,file` | 49 | 0 | 0 | structured 41・api_surface 4・doctest 4 |
| `--all-features` | 150 | 0 | 0 | structured 129・api_surface 9・doctest 12 |

### `test-io-e2e` 内訳（77 件全 pass・slice-3 MSRV lane と同数）

| test binary | passed |
| --- | --- |
| staged_lifecycle | 6 |
| structured_staged | 6 |
| structured_formats_staged | 8 |
| archive_compression_staged | 6 |
| extraction | 10 |
| fault_boundary | 6 |
| negative_fuzz | 10 |
| os_fs_probe | 1 |
| rooted_boundary | 4 |
| wav_staged | 3 |
| wav_fault | 9 |
| wav_golden | 8 |
| lib unittests / doctests | 0 / 0 |

## 最終検証（workspace 全体・`origin/main` merge 後）

`git merge origin/main` は "Already up to date"（base SHA と同一）
のため merge commit なし。その状態で以下を実走。

| 検証 | コマンド | 結果 | 備考 |
| --- | --- | --- | --- |
| fmt | `mise run fmt-check` | Pass (exit 0) | `cargo fmt --all -- --check` |
| workspace check | `CARGO_BUILD_JOBS=2 mise run check` | Pass (exit 0) | `cargo check --workspace --all-targets` |
| workspace lint | `CARGO_BUILD_JOBS=2 mise run lint` | Pass (exit 0, 34.92s) | `cargo clippy --workspace --all-targets -- -D warnings` |
| workspace test | `CARGO_BUILD_JOBS=2 mise run test` | Pass (exit 0) | `cargo test --workspace`。**3696 passed / 0 failed / 7 ignored**（197 test result 行全て `ok`。ignored 7 件は既存の意図的 ignore で本 slice 起因の回帰ではない） |
| RT lane | `CARGO_BUILD_JOBS=2 mise run test-rt` | Pass (exit 0) | `cargo test --workspace rt_` name filter。**288 passed / 0 failed / 0 ignored**（153 test result 行） |

## NotRun / Blocked / Unsupported（正直な記録）

- macOS/Linux 実機での feature-matrix / IO 実測: **NotRun** —
  本検証は Windows 11 x64 / NTFS のみ。
- Network/Cloud FS での IO/durability 実測: **Unsupported** —
  実測環境なし。FS 依存の durability 保証は主張しない
  （slice 1〜3 と同じ分類）。
- 実 4 GiB ファイル publish・性能計測: **NotRun** — scope 外。
- consumer gate（AudioLens #139/#143・Synvelis #82/#84・
  #219/#294）: **NotRun** — cross-repo gate であり別 owner。
  本 slice の headless lane 成功を consumer 受入に転用しない。
- `Indeterminate`・DirSync 後失敗・`TempState::Unknown` の
  fixture 化: **Unsupported（fixture 化せず・別 Issue へ延期済み）**
  — `ruviform-io-file` `#[cfg(test)]` MockFileOps 試験が正本
  （slice 1〜3 と同じ分類。本 slice で偽装 fixture は追加して
  いない）。
- `test-io-msrv`（MSRV 1.89 lane）: **本 slice では未再実行** —
  slice-3 で実測済み（`evidence/rf306/report.md` Slice 3 節）。
  本 slice の対象は feature-matrix lane であり、MSRV lane の
  再実測は要求されていない。
- Issue #306 の close: **対象外** — consumer 実測等の残存受入行が
  残る。

## Slice 4 の性質と境界

- 本 slice は `evidence/` と `docs/impl/`（IR + index）のみを
  変更する。`crates/`・`mise.toml`・`Cargo.toml`/`Cargo.lock`・
  `.github/workflows/` は無変更（`git status` で確認可能）。
- feature-matrix lane の Pass は「当該 toolchain・当該 OS 上で
  scoped lane が緑である」ことの記録であり、実機 FS/consumer
  gate の合格を意味しない。
