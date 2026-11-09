---
id: "TSPEC-034UtKOMzAChsfMRSOpIdg"
type: "test-spec"
status: "approved"
title: "Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計

## Purpose

Issue #288 の Gate 設計正本（[design](../designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)）
が「判定語彙を誠実に固定し、受入 matrix・否定系判定表・Issue 接続表が
実在する正本への参照で構成され、設計合意とテスト完了を混同していない」
ことを検証する。対象 spec は
[spec](../specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)、
実施記録は [task](../tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md) /
[plan](../plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md) を参照。
コード変更を伴わない docs 成果物のため、検証は文書検査
（link/relation 解決・参照先の実在確認・内容の突合・lint）で行う。
本 test-spec は Gate の**設計文書の正しさ**を対象とし、各 Gate 行が
指す実走テスト自体の合否は対象外（実装段階の要件）。

## Feature

IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計文書

## Rules

- **R1（matrix 網羅）**: 受入 matrix は IO-01 File・IO-02
  Structured・IO-03 Compression・IO-04 Archive・IO-05 Migration・
  合成 E2E の各対象について、正常系と否定系の双方を、入力条件・
  障害条件・Capability/OS/FS 軸・判定基準・証拠/根拠の列で
  定義する。Issue #288 の受入スコープ表（正常系/否定系の列挙）
  の各項目が少なくとも 1 行の matrix 行に対応する。
- **R2（判定語彙の誠実さ）**: `impacted`/`no impact`/`unknown` と
  `pass`/`fail`/`blocked`/`not run`/`unsupported` の語彙が
  Issue #219/#294 と整合し、`unknown`・`not run` を PASS 扱い
  しない規則、`unsupported` の限定意味（正しく拒否した否定
  テスト成功のみ）、Consumer 必須 Capability 欠落時の採用 Gate
  不通過、ベースライン不具合と新規退行の分離が明記される。
- **R3（参照の実在性）**: matrix・接続表・証拠導線の各参照
  （Issue 番号・PR 番号・test-spec パス・mise task 名・
  workflow 名・crate 名）が作成時点で実在する。存在しない
  Issue/文書/command を根拠として記載しない。
- **R4（否定系判定表の完備）**: publish/recovery・安全性・
  resource budget・Media/PCM/RT・Consumer 非回帰の 5 領域に
  否定系判定表があり、各行が期待分類・pass/fail 判定・根拠を
  持つ。
- **R5（Owner/境界接続）**: IO-01〜05 のテスト要件と Owner・
  OS/FS/脅威モデル境界が実装 Issue（#295〜#306）と Consumer
  Issue（AL #139/#140/#141/#143、SV #82/#84/#25）へ対応付け
  られている。
- **R6（Consumer 要件）**: Ruviform/AudioLens/Synvelis の影響・
  Golden・回帰・性能・Dependency 更新要件が明文化され、
  Consumer 独立判定・両 Consumer 影響 Breaking Change の採用
  完了条件が含まれる。
- **R7（Breaking Change 導線）**: 旧 `StagedOutput`/旧
  `PublishOptions` の採用手順・戻し方の追跡先が #287/#305・
  PR #391・各 Consumer Issue へ接続されている。
- **R8（証拠導線の簡素性）**: 証拠は既存 Actions/PR/Issue の
  参照のみで追え、新規 CI YAML・証拠スキーマ・Dashboard・
  重複レポートを追加していない。
- **R9（設計/実装分離）**: 「実装段階では各必須 Gate の実走
  PASS を要求する」旨が明記され、本 slice の文書合意を
  テスト完了と混同しない記述がある。未実施項目は
  `not run`/`NotRun`/`Unsupported` と記録する方針が明示される。
- **R10（変更範囲）**: 差分が `docs/` の新規 `issue-288-*`
  文書と生成 index のみであり、`crates/`・`Cargo.toml`・
  `mise.toml`・`.github/` に変更がない。

## Examples

- Given Issue #288 の受入スコープ表の IO-04 Archive 行
  （Path Traversal/Link/重複・casefold 衝突/Zip Bomb/未読 TAR
  Entry/TOCTOU/Cleanup）、When matrix を検査する、Then 各否定
  項目が A-X1〜A-X5 の判定基準と `issue-304`/`issue-306`
  test-spec への参照を持つ。
- Given consumer-verify.yml の run が AudioLens lane=pass・
  Synvelis lane=not_run、When Gate 判定を行う、Then 「両
  Consumer 受入済み」とは判定されず Synvelis は `not run`
  のまま記録される（R2/R6 と整合）。
- Given `unsupported` が「CreateNew 非対応 FS での正しい拒否」
  のみを意味する、When Consumer 必須 Capability が欠ける
  OS/FS で採用判定を行う、Then 当該 Consumer の採用 Gate は
  通過しない（R2）。
- Given 本 PR の diff、When `git status`/`git diff --stat` を
  確認する、Then 変更は `docs/` の `issue-288-*` 新規文書と
  `docs/*/README.md` index のみ（R10）。

## Guarantees

- matrix の各行は正本（test-spec/Issue/command）への参照を持ち、
  判定基準の複写ではなく参照で構成される — 正本更新時に
  本書が自動的に陳腐化しない構造。
- 判定語彙は #219/#294 の既存契約と矛盾しない（独自の別語彙を
  作らない）。
- OS/FS の Supported 主張は実測済み範囲に限定され、未実測は
  NotRun/Unsupported と記録される。
- Breaking Change の追跡は既存 Issue/PR の状態を snapshot
  として記録し、状態変動を隠さない。

## Non-goals

- 各 Gate 行が指す実走テストの合否（実装段階の要件）。
- consumer-verify.yml・daily-ci.yml・ci.yml の変更。
- AudioLens/Synvelis repo の検証・変更。
- macOS/Linux 実機・Network/Cloud FS の実測。
- matrix 実走結果の集約・可視化基盤。

## Risk

- 本 test-spec が検証するのは設計文書の正しさであり、Gate の
  実効性（実走 PASS）は別段階で検証される。設計文書が完璧でも
  実装段階の Gate が未実走なら Issue #288 の受入は未完である
  — この分離を崩すと「設計合意＝テスト完了」の誤認が起きる。
- 参照先 Issue の状態（OPEN/CLOSED）は変動する。本書は
  2026-11-09 時点の snapshot を記録しており、将来の状態変化で
  接続表の「役割」記述が古くなる可能性がある — snapshot 日付の
  明記で緩和する。
