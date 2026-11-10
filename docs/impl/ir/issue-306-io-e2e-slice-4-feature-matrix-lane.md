---
id: "IMPL-034V8l54mhl7RUcgflLUyv"
type: "impl"
status: "completed"
title: "Issue #306 IO 合成 E2E slice-4 feature-matrix lane 実測証拠"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "evidence/rf306/feature-matrix-report.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-e2e-slice-4-feature-matrix-lane.md"
    modified:
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-306-io-msrv-dag-rt-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-306-io-composition-e2e.md"
    - "docs/impl/ir/issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md"
    - "evidence/rf306/report.md"
    - "evidence/rf306/fault-fuzz-report.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #306 IO 合成 E2E slice-4 feature-matrix lane 実測証拠

## Summary

Issue #306 受入条件の「Feature matrix lane（compression /
structured / e2e scoped lanes）の実測結果を Pass/Fail/NotRun で
証拠記録する」行を閉じた。**コード・テスト・設定ファイルを
一切変更しない検証・証拠 slice** であり、9 本の scoped
feature-matrix lane（`check/lint/test-io-compression`、
`check/lint/test-io-structured`、`check/lint/test-io-e2e`）と
workspace 最終検証 5 本（`fmt-check` / `check` / `lint` / `test` /
`test-rt`）を全て `mise run` 経由で実走し、コマンド・exit
code・toolchain・test 数を `evidence/rf306/feature-matrix-report.md`
へ記録した。全 lane Pass。`crates/`・`mise.toml`・
`Cargo.toml`/`Cargo.lock`・`.github/workflows/` は無変更。

## Context

- 上流 Issue: [Issue #306](https://github.com/scarecrow173/ruviform/issues/306)
  （IO-IMP-12 共通 I/O 合成 E2E）・親 [Issue #288](https://github.com/scarecrow173/ruviform/issues/288)。
- 前段 slice: [IR slice-1](issue-306-io-composition-e2e.md)・
  [IR slice-2](issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md)・
  [IR slice-3](issue-306-io-msrv-dag-rt-audit.md)（`test-io-msrv`
  lane 追加済み・main 統合済み）。
- 証拠: [evidence/rf306/feature-matrix-report.md](../../../evidence/rf306/feature-matrix-report.md)。
  既存 [report.md](../../../evidence/rf306/report.md)（slice 1・3）と
  [fault-fuzz-report.md](../../../evidence/rf306/fault-fuzz-report.md)
  （slice 2）の形式に従い、本 slice は独立ファイルとした。

## Implementation

### Changes

- `evidence/rf306/feature-matrix-report.md`（新規）: slice-4
  実行証拠。worktree/branch/base/環境、9 本の feature-matrix
  lane と 5 本の workspace lane のコマンド・exit code・結果、
  compression 5 組合せ・structured 4 組合せの test 数内訳、
  io-e2e 77 tests の binary 別内訳、NotRun/Unsupported 一覧。
- `docs/impl/ir/README.md`: 本 IR の row を追加（index は
  tool 非管理のため手動追記 — slice-1 IR の記録どおり）。
- 本 IR（新規）。
- `crates/` 配下・`mise.toml`・`Cargo.toml`/`Cargo.lock`・
  `.github/workflows/`・他 docs: **無変更**。

### Decisions During Implementation

- **証拠は新規ファイル `feature-matrix-report.md`**: slice-2
  （`fault-fuzz-report.md`）と同じく、slice 固有の検証証拠は
  独立ファイルとし、`report.md` への追記（slice-3 方式）と
  使い分けた。feature-matrix lane は crate 単位の scoped lane
  群であり、slice-1/3 の E2E/MSRV 証拠とは対象が異なるため。
- **組合せ別 test 数は log の機械集計で記録**: `mise run
  test-io-compression` / `test-io-structured` は `&&` chain の
  1 タスクのため、各 `cargo test` 起動境界（`Running
  unittests src\lib.rs` / `Doc-tests` 行）で log を分割し、
  組合せ別の passed/failed/ignored を awk で集計した。値は
  実測 log 由来であり推測ではない。
- **workspace test の ignored 7 件は回帰と断定しない**: 全
  result 行が `ok` で failed=0 を確認した上で、ignored の
  内訳までは本 slice で個別監査していない（slice-3 時点の
  7 ignored と同数だが、同一性の推測記述は避けた）。
- **doc チェーンは IR のみ**: 本 slice の編集範囲は
  `evidence/` と `docs/impl/`（IR + index）に限定されており、
  spec/design/plan/task 文書は作成していない（検証・証拠のみの
  slice であり、task spec の成果物定義に従った）。

## Related Experiments

なし（既存 lane の実測記録のみ。新規の技術的探索は行ってい
ない）。

## Validation

全て `mise run` タスクで実走（exit code は `mise run` 自体の
終了コード）:

- `mise run check-io-compression` / `lint-io-compression` /
  `test-io-compression` — Pass (exit 0)。test は 5 組合せ合計
  **264 passed / 0 failed / 0 ignored**（default 3・gzip 52・
  deflate 26・zstd 58・gzip,deflate,zstd 125）。
- `mise run check-io-structured` / `lint-io-structured` /
  `test-io-structured` — Pass (exit 0)。test は 4 組合せ合計
  **251 passed / 0 failed / 0 ignored**（default 5・xml 47・
  xml,file 49・all-features 150）。
- `mise run check-io-e2e` / `lint-io-e2e` / `test-io-e2e` —
  Pass (exit 0)。test は **77 passed / 0 failed / 0 ignored**
  （12 統合 binary + lib/doctest 0）。
- `git fetch origin main && git merge origin/main` —
  "Already up to date"（base = `4d41b021`、追加 merge 不要）。
- `mise run fmt-check` — Pass (exit 0)。
- `CARGO_BUILD_JOBS=2 mise run check` — Pass (exit 0)。
- `CARGO_BUILD_JOBS=2 mise run lint` — Pass (exit 0, 34.92s)。
- `CARGO_BUILD_JOBS=2 mise run test` — Pass (exit 0)。
  **3696 passed / 0 failed / 7 ignored**（197 result 行全て ok）。
- `CARGO_BUILD_JOBS=2 mise run test-rt` — Pass (exit 0)。
  **288 passed / 0 failed / 0 ignored**。
- 実行環境: Windows 11 x64 (MINGW64_NT-10.0-26300) /
  rustc 1.99.0 (b940084d7 2026-09-28) / cargo 1.99.0 / NTFS。

## Risks

- feature-matrix lane の Pass は headless な compile/lint/test
  の緑であり、実 OS FS・consumer 受入を意味しない（証拠
  ファイルに明記済み）。
- `test-io-msrv`（MSRV 1.89 lane）は本 slice で未再実行。
  slice-3 の実測値は main 上の既存証拠を参照するのみ。
- workspace test の ignored 7 件は本 slice で内訳監査して
  いない。failed=0 は確認済み。

## Follow-ups

- Issue #306 の残存受入行: macOS/Linux 実機・Network/Cloud FS・
  実 4 GiB 計測（NotRun）、consumer gate（AudioLens/Synvelis、
  別 owner）、`ruviform-io-file` mock 注入点の public
  fixture 化（別 Issue へ延期済み）。
