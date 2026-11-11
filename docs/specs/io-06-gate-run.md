---
id: "SPEC-A8NXYmNb4ydatXQlOH1ptr"
type: "spec"
status: "approved"
title: "IO-06 Gate matrix 実走と証拠記録（Issue #288 slice）"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/219"
  defers: []
  deferred-by: []
---

# IO-06 Gate matrix 実走と証拠記録（Issue #288 slice）

## Purpose

Issue #288（IO-06 Cross-Consumer・OS/FS・Security・性能・互換性
Gate）の受入 matrix を、Windows 実機で実走可能な mise lane の
実測と既存証拠（`evidence/rf306/`・GitHub Actions/PR/Issue
記録）へ対応付け、セルごとの判定（`pass` / `fail` / `blocked` /
`not run` / `unsupported`）を `evidence/rf288/io06-gate-matrix.md`
へ記録する slice の spec。

Gate 判定語彙・matrix の構造・否定系判定表・Owner/境界接続表の
正本は [design 正本](../designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)
（PR #510 で main 統合済み）であり、本 spec はその**実走記録
slice** の範囲と受入条件のみを定義する。判定基準の再定義は
行わない。

## Why Now

- IO-IMP-12（#306）slice-1〜4 が main に統合済みで、
  `evidence/rf306/` に Windows NTFS での実走記録
  （report.md・fault-fuzz-report.md・feature-matrix-report.md）
  が存在する。本 slice はそれらを IO-06 matrix のセルへ
  写像し、未カバーの lane を本 worktree で実走して埋める。
- Issue #288 の受入条件は「実装段階では各必須 Gate の実走
  PASS を要求し、設計の合意をテスト完了と混同しない」を
  要求する。設計文書（#510）の存在だけではこの条件を満たさ
  ないため、実走結果の記録が必要。

## Users and Value

- **Issue #288 Owner / Coordinator**: matrix の各セルが
  どの lane・証拠で判定されたかを一箇所で追跡できる。
- **Consumer 採用判定（#219/#294）**: Ruviform 側 Gate の
  実走状況（pass/fail/not run/unsupported の内訳）を
  consumer-verify の tuple 判定と突合する基礎資料になる。
- **後続 slice 担当**: 残存 NotRun（macOS/Linux 実機・
  Network/Cloud FS・実 4GiB・consumer gate・fixture 化）の
  一覧が明確になり、次の作業を選べる。

## Scope

### In Scope

- Issue #288 受入 matrix（IO-01〜05 + 合成 E2E、正常系・否定系
  × Capability/OS/FS）の各セルと、実在する mise lane・
  `evidence/rf306/`・既存 PR/Actions 記録の対応付け。
- Windows 11 x64 / NTFS で実走可能な lane の実走と、
  セルごとの `pass`/`fail`/`blocked`/`not run`/`unsupported`
  判定の根拠付き記録。
- `evidence/rf288/io06-gate-matrix.md` の作成。
- 実走結果と残存 NotRun の Issue #288 への `gh issue comment`
  投稿と PR 本文への記録。
- matrix の穴（どの lane にも対応しない必須セル）の列挙。
- 本 slice の文書チェーン（spec/design/plan/task/test-spec/
  impl IR）。

### Out of Scope

- `crates/`・`mise.toml`・`.github/`・`Cargo.toml` 系の変更
  （コード変更なし。gate 実走と記録のみ）。
- 新規 CI Workflow・証拠スキーマ・集約 DB・Dashboard の作成
  （Issue #288 の既存機構参照方針どおり）。
- macOS/Linux 実機・Network/Cloud FS・実 4GiB ファイル・
  consumer 実測の実施（環境外・別 owner。NotRun/Unsupported
  として記録）。
- `Indeterminate`・DirSync 後失敗・`TempState::Unknown` の
  fixture 化（公開 API から誘発不能。`ruviform-io-file` の
  `#[cfg(test)]` MockFileOps 試験が正本 — rf306 と同じ分類）。
- Issue #288 の Open/Close 判定（Coordinator が行う）。
- `docs/*/README.md` index の編集（Coordinator が merge 時に
  再生成）。

## Requirements

- matrix の各セルは `pass`/`fail`/`blocked`/`not run`/
  `unsupported` のいずれかで、**根拠**（lane 名・exit code・
  OS・toolchain・既存記録/PR/Actions への参照）を持つ。
  `unknown`・`not run` を PASS 扱いしない。
- `unsupported` は「当該 OS/FS が必要保証を提供できないと
  正しく拒否した否定テストの成功」に限定して記録し、
  Consumer 必須 Capability の欠落を免除しない。
- 実走 lane の exit code・OS・toolchain を matrix に記録する
  （Issue #288「実装段階では各必須 Gate の実走 PASS を要求」）。
- 既存 Actions/PR/Issue を参照するだけで検証を追える構成とし、
  専用 YAML・重複レポートを新設しない。
- `evidence/rf306/` の既存実走記録と同一内容の重複実走は
  行わず参照する（本 slice で新規に実走するのは Issue #288
  matrix の Windows lane と `test-io-msrv` 等の未再実走 lane）。
- 実走不能なセルは理由付きで `not run`/`unsupported`/`blocked`
  として記録し、残存 NotRun を Issue #288 comment と PR 本文に
  列挙する。

## Acceptance Criteria

- [x] Issue #288 受入 matrix の全セル（IO-01〜05 + 合成 E2E、
      正常系・否定系、publish/recovery・安全性・resource
      budget・Media/PCM/RT・Consumer 非回帰の否定系判定表を含む）
      が `evidence/rf288/io06-gate-matrix.md` で lane/証拠/判定
      付きで記録されている。
- [x] Windows で実走可能な lane（fmt-check・check・lint・test・
      test-rt・check/lint/test-io-file・check/lint/test-io-
      structured・check/lint/test-io-compression・check/lint/
      test-io-e2e・test-io-msrv）が実走され、exit code・OS・
      toolchain が記録されている（`test-io-msrv` は rust@1.89
      が利用可能な場合のみ。不可なら NotRun と記録）。
- [ ] 実走結果と残存 NotRun（macOS/Linux 実機・Network/Cloud
      FS・実 4GiB・consumer gate・fixture 化）が `gh issue
      comment` で ruviform#288 に投稿され、PR 本文にも同内容が
      記録されている。
- [ ] matrix の穴（どの lane にも対応しない必須セル）が PR 本文と
      Issue comment に列挙されている。
- [x] `mise run fmt-check` と docs 検証（doc-status 系 lint/
      audit で本 PR 追加分に新規 blocking finding がないこと）が
      Pass。
