---
id: "DESIGN-xYI287PLIiNGr8mOrzVr7c"
type: "design"
status: "approved"
title: "IO-06 Gate matrix 実走証拠の記録設計"
created: "2026-11-11"
updated: "2026-11-12"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/io-06-gate-run.md"
      - type: "doc"
        path: "docs/designs/io-06-gate-run-design.md"
      - type: "doc"
        path: "docs/plans/io-06-gate-run-plan.md"
      - type: "doc"
        path: "docs/tasks/io-06-gate-run-task.md"
      - type: "doc"
        path: "docs/test-specs/io-06-gate-run.md"
      - type: "doc"
        path: "docs/impl/ir/io-06-gate-run-ir.md"
      - type: "file"
        path: "evidence/rf288/io06-gate-matrix.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/io-06-gate-run.md"
  implemented-by: []
  depends-on:
    - "docs/specs/io-06-gate-run.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "evidence/rf306/report.md"
    - "evidence/rf306/fault-fuzz-report.md"
    - "evidence/rf306/feature-matrix-report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/io-06-gate-run.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/219"
  defers: []
  deferred-by: []
---

# IO-06 Gate matrix 実走証拠の記録設計

## Context

[spec](../specs/io-06-gate-run.md) が定義する実走記録 slice の
設計。Gate の判定語彙・matrix 構造・否定系判定表の正本は
[issue-288 design](issue-288-io-06-cross-consumer-os-fs-security-gate.md)
であり、本書は **matrix セルを実在する lane・証拠へ写像する
手順と記録形式** のみを設計する。判定基準の再定義・新規
証拠基盤の構築は行わない。

## 設計判断

### D1: matrix は design 正本の行 ID をそのまま使う

issue-288 design の受入 matrix は行 ID を持つ（F-N1〜F-X4、
S-N1〜S-X6、C-N1〜C-X4、A-N1〜A-X5、M-N1〜M-X3、E-N1〜E-X2）。
本 slice の記録も同じ行 ID を使い、design 正本との対応を
一意に保つ。加えて否定系判定表 5 領域
（publish/recovery・安全性・resource budget・Media/PCM/RT・
Consumer 非回帰）と OS/FS capability 軸を別節で記録する。

### D2: セル判定は「venue × 判定語彙」で記録

issue-288 design「結果記録フォーマット」に従い、各セルの結果は
実行 venue ごとに `pass`/`fail`/`blocked`/`not run`/
`unsupported` で記録する。本 slice の venue は:

- **local-ntfs**: 本 worktree（Windows 11 x64 / NTFS /
  rustc 1.99.0）の `mise run` 実走。exit code を記録。
- **rf306**: `evidence/rf306/` の既存実走記録（同一 Windows
  NTFS 環境・同一 lane 群・過去 SHA）への参照。内容の重複
  実走はしないが、本 slice でも主要 lane は再実走して
  最新 SHA での状態を確認する。
- **ci**: `ci.yml` の既存 lane（verify ubuntu/windows/
  macos・rt-verify・compression-verify・io-structured-verify）
  の成功 run への参照。OS/FS 軸の Linux/macOS セルはこの
  venue が一次証拠（design 正本の「CI lane 結果の参照に委ねる」
  方針どおり）。
- **consumer-verify**: `consumer-verify.yml` の tuple 証拠
  （`consumer-evidence-*.json` artifact・Job Summary・run URL）
  への参照。AudioLens/Synvelis を独立に記録し、片側 PASS を
  両 Consumer 受入とみなさない。

### D3: lane → matrix 行の対応付け規則

各 matrix 行の「証拠/根拠」列が指す test-spec・mise task を
そのまま実行対象とする。対応の粒度:

- IO-01 File 行 → `test-io-file`（io-file + io-archive の
  unit/integration/os_publish/rooted_read）+ `test-io-e2e`
  （staged_lifecycle・fault_boundary・os_fs_probe）。
- IO-02 Structured 行 → `test-io-structured`（format feature
  matrix: default/xml/xml,file/all-features）+ `test-io-e2e`
  （structured_staged・structured_formats_staged・negative_fuzz
  の encoding/DTD 系）。
- IO-03 Compression 行 → `test-io-compression`（feature matrix:
  default/gzip/deflate/zstd/gzip,deflate,zstd）+ `test-io-e2e`
  （archive_compression_staged・negative_fuzz の checksum/
  truncated 系）。
- IO-04 Archive 行 → `test-io-file`（io-archive 側:
  zip/tar/extract 系試験）+ `test-io-e2e`（extraction・
  negative_fuzz の traversal/bomb 系）。
- IO-05 Migration 行 → `test-io-e2e`（wav_staged・wav_fault・
  wav_golden）+ workspace `test`（旧 API 参照 0 件は grep で
  補助確認）+ `test-rt`（RT 非回帰）。
- 合成 E2E 行 → `test-io-e2e` 全件 + `test-io-msrv`（MSRV
  1.89 での同 lane）。
- publish/recovery 否定系 → `test-io-file`（mock cfg(test)
  fault matrix 正本）+ `test-io-e2e`（fault_boundary・
  os_fs_probe の実 FS 分）。
- 安全性否定系 → `test-io-e2e`（extraction・negative_fuzz・
  rooted_boundary）+ `test-io-structured`（DTD/encoding/alias
  系）。
- resource budget 否定系 → `test-io-structured`/`test-io-
  compression`/`test-io-file`（LimitExceeded 系試験）。
- Media/PCM/RT 非回帰 → workspace `test` + `test-rt`。
- Consumer 非回帰 → `consumer-verify.yml` run の tuple 証拠。

### D4: 実走不能セルの分類

- macOS APFS / Linux ext4 実機: ローカル実機なし → CI
  `verify-macos`/`verify (ubuntu-latest)` の `test-ci`
  （workspace test、io 系 crate を含む）を一次証拠として
  `pass`（CI run 参照付き）とし、実機固有の FS 保証差異
  （dir fsync 等）は design 正本の capability matrix の
  「実測」列どおり未実測部分を `not run` として残す。
- Network/Cloud FS: 実測環境なし → `unsupported`（保証を
  提供できない FS への対応を主張しない、の意味）または
  `not run`。design 正本は「未実測を Supported としない」
  のみ要求するため、セルの性質に応じて分類する。
- 実 4GiB ファイル publish・性能計測: lane なし → `not run`。
- `Indeterminate`・DirSync 後失敗・`TempState::Unknown`:
  公開 API から誘発不能 → `unsupported`（fixture 化しない。
  MockFileOps `#[cfg(test)]` 試験が正本であることを根拠に
  記載）。
- consumer gate の consumer 固有受入（AL/SV の Golden・
  性能・OS 実機）: consumer-verify の `not_run`/`unsupported`
  lane をそのまま転記し、Ruviform 側 lane の pass を consumer
  受入完了と混同しない。

### D5: 記録ファイルは `evidence/rf288/io06-gate-matrix.md` 1 本

Issue #288「専用 YAML・重複レポートは不要」に従い、matrix 記録は
`evidence/rf288/` 配下の Markdown 1 ファイルに集約する。
`evidence/rf306/` と同じ「実走証拠レポート」形式（環境・
コマンド・exit code・test 数・NotRun 節）を踏襲する。

## OS/FS 軸の扱い

design 正本の capability matrix（OS/FS × primitive）に対し、
本 slice では実測 venue ごとの結果を記録する:

| OS/FS | 本 slice での証拠 venue | 期待される記録 |
| --- | --- | --- |
| Windows 11 x64 / NTFS | local-ntfs `mise run` 実走 + ci verify (windows-latest) | pass（実測済み範囲） |
| Linux ext4（ubuntu-latest runner） | ci verify (ubuntu-latest)・compression-verify・io-structured-verify の test-ci/feature matrix | pass（CI 実測。FS 種別は runner 既定） |
| macOS APFS（macos-latest runner） | ci verify-macos の test-ci | pass（CI 実測。FS 種別は runner 既定） |
| Linux vfat/exFAT・NFS、Windows FAT32/exFAT/ReFS、その他 unix | 該当 lane なし | not run（未実測を Supported としない） |
| Network/Cloud FS | 該当 lane なし | not run / unsupported（FS 依存保証を主張しない） |

FS 種別が runner 既定である点は証拠に明記し、「ext4/XFS・
APFS を個別に実測した」とは記録しない（Issue #288 の
「OS・Server・Mount 条件と保証範囲を個別に扱い、未実測を
Supported としない」に整合）。

## Failure / Blocked の扱い

- lane が fail した場合は `fail` として exit code・失敗 test
  名を記録し、原因がスコープ外（main 既存破損・他 Worker
  起因・infra flaky）と確定した場合のみ `blocked` とし、
  証拠とともに報告する。自分のスコープの失敗は修正して
  再検証する（本 slice はコード変更禁止のため、スコープ内
  失敗が起きた場合は docs 修正か lane 記録の訂正で対処）。
- `test-io-msrv` は `mise exec rust@1.89.0` が利用可能な
  場合のみ実走。不可なら `not run`（理由: toolchain 不在）。

## Risks and Trade-offs

| # | Risk | Mitigation |
| --- | --- | --- |
| 1 | CI run の参照 SHA が最新 main とずれ、判定の鮮度を誤認 | run の headSha・event・実行日時を matrix に明記し、参照 SHA と実走 SHA を区別する |
| 2 | consumer-verify の tuple が古い Ruviform Revision で「最新の consumer 受入」と誤読 | evidence JSON の `candidate.ruviformRev`・`consumer.resolvedSha`・run URL を転記し、revision を明示する |
| 3 | `test-io-file` の `#[ignore]` helper を「未実行の試験」と誤読 | ignore 対象は spawned child entry point であり、親 test `create_new_contention_multi_process` は通常実行されることを確認済み（`os_publish.rs` L406–L472） |
| 4 | matrix 記録が design 正本の行 ID とずれる | 行 ID は design 正本から転記し、test-spec で突合を検証する |

## Verification

- [x] `mise run fmt-check` pass。
- [x] doc-status 系 lint/audit で本 PR 追加分に新規 blocking
      finding がないこと。
- [x] matrix の全セルに判定と根拠があり、`not run`/`unknown` を
      PASS 扱いしていないこと（test-spec の R 系で検証）。
