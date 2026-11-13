---
id: "DESIGN-fH3jK9mP2qW8rT5yU7iO4a"
type: "design"
status: "approved"
title: "Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完 設計"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-306-io-e2e-cases.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-io-e2e-cases.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-306-io-e2e-cases.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-306-io-composition-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-e2e-cases.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完 設計

## Overview

`docs/specs/issue-306-io-e2e-cases.md` の実装設計。2 つの独立
した作業面を持つ: (V) 既存 CI run の証拠抽出と記録、
(T) `ruviform-io-e2e` への L1 Case 補完。両者は同じ
Issue #306 受入条件の別行を満たし、相互に依存しない。

## 証拠抽出設計（Slice V）

### 入力と採用規則

- 対象 SHA は `e93c438adc4af5b498e10926fe859ac1338ac390`
  に固定。Coordinator dispatch の run `43428172820` を第一
  候補とし、完了前は同 SHA の最新 completed run を採用
  する規則（今回は dispatch run が completed/success で
  確定）。
- `gh run view --json` で status/conclusion、
  `gh api .../jobs` で job 一覧、`gh api .../jobs/{id}/logs`
  で生ログを取得する。

### 集計方法と正直さの担保

- 生ログ中の `Running unittests src/lib.rs`・
  `Running tests/<binary>` 告知と直後の `test result:`
  行を対応付け、crate 名（告知中の crate パスまたは
  binary 名→package 対応）で集計する。
- doctest・compile-fail batch・複数 cargo 呼出しを含む
  workspace log では section 単位で対応付け、誤対応は
  診断として扱い対象 crate の値が安定することを確認する。
- job conclusion=success は前提条件であり、Pass 判定は
  対象 crate の test binary 実行行（passed/failed/ignored
  カウント）で行う。集計スクリプトは補助証拠であり、
  canonical 検証は mise task。

### 記録様式

`evidence/rf306/ci-os-l1-report.md` に run メタ・job 表・
crate 別集計・feature lane 内訳・NotRun/Unsupported 表を
既存 evidence（`report.md`・`fault-fuzz-report.md`・
`feature-matrix-report.md`）と同じ日本語・表形式で記録
する。#288 の 38 Case・#294 consumer 結果とは L1/追加能力を
分離して言及するのみで再実行しない。

## Case 補完設計（Slice T）

### 追加先の選定

`staged_lifecycle.rs` は staged publish lifecycle の e2e
正本であり、module doc が「実 OS で再現可能な振る舞いのみ
assert する」方針を明記済み。今回の 3 Case は全て実 FS
で再現可能なため同ファイルへ追加し、新規 binary を増や
さない。

### Case 1: post-seal temp rewrite

- `StagedFile::create` → write → `seal()` →
  `SealedFile::temp_path()`（公開 API）で temp path 取得
  → `std::fs::write` で外部 rewrite（長さ変更あり）→
  `publish(CreateNew, FileAndDirectorySync)`。
- 期待: publish は temp 内容を再検証しないため成功し、
  destination に改竄 bytes が入る。これは seal 時点保証の
  境界を契約として固定するもので、io-file の動作変更を
  意図しない。test 名・コメントで「not reverified」を
  明示し、将来の再検証実装時に契約見直しを強制する。

### Case 2: post-seal temp delete

- seal 後に `std::fs::remove_file(temp)` → publish。
- 期待: rename 源が無いため全 OS で `NotPublished`。
  `Durability::BestEffort` で file/dir sync を外し、
  publish stage 自体の fault に assertion を限定する
  （Windows の delete-pending 中 fsync 挙動差を排除）。
- 失敗後も `take_recovery_handle` は常に Some（`fail()`
  が ops を必ず RecoveryHandle 化）であり、absent temp
  の cleanup は `TempState::Absent` で成功することを確認。

### Case 3: recovery cleanup failure→retry

- CreateNew conflict（incumbent file）で publish 失敗 →
  temp retained + RecoveryHandle 取得。
- sabotage: temp file を `moved` に rename し、temp path
  に directory を作成 → `handle.cleanup()` が
  remove-file 失敗で `CleanupFailure`。
- `take_recovery_handle` で handle 返却を確認 → directory
  除去・temp 復元 → retry `cleanup()` が成功し
  `TempState::Absent`。
- rename 先行は、ops が publish 失敗時点で temp handle を
  release 済みでも Windows の delete-pending 名前再利用
  制約に依存しない堅牢な手順（handle が残る将来実装でも
  成立）。

### 責務分離

- Indeterminate・`TempState::Unknown`・directory-sync 失敗
  は公開 API で誘発不能（fault seam が `pub(crate)`/
  `cfg(test)`）のため追加しない。io-file の mock 正本が
  canonical であり、e2e での重複・迂回を作らない。
- `PlatformFileOps`/`MockFileOps` の pub 化判定（G07）は
  設計判断として別 slice。本 slice は evidence に到達
  不能性を記録するのみ。

## RT Fake Fault の解釈

io-e2e・io 系 crate の公開 API は全て non-RT（doc 明記）。
rt-verify lane（`cargo test --workspace rt_`）の実測でも
io-e2e の一致は `assert_` 部分文字列の誤検出のみで実 rt_
test は 0。**rt_ 契約 test の追加は不要**と判定し、
DAG audit（io-e2e leaf・rt 非 io/blocking 依存）の既存
証跡を引用する。rt_ 名称 filter の非混入は lane 実測で
確認済み。

## 検証設計

- 追加 test は `mise run test ruviform-io-e2e`（canonical）
  で全件実走。`cargo test` 直接実行は補助証拠。
- `fmt-check`/`check`/`lint`/`test` を変更範囲に比例して
  実行。`*-full`/`verify-full`/`*-msrv` はタスク指定により
  ローカル必須検証に含めない（CI lane が正本）。
- merge 前に `git fetch origin main && git merge
  origin/main` で最新 main を取り込み、docs index を
  `doc-maintenance` の `index plan/apply` で再生成する。

## Risk と Rollback

- tamper test が「改竄 bytes が publish される」現行動作を
  固定するため、io-file が将来 publish 時再検証を実装した
  際に test 更新が必要 — これは意図した契約固定であり
  risk ではなく設計上のトリガー。
- test 3 の sabotage は OS 差（Windows delete-pending・
  Unix EISDIR）を rename 先行で吸収。万が一の失敗時は
  `cargo test --test staged_lifecycle` で単独再現可能。
- Rollback は test 追加の revert のみで production 影響
  なし。
