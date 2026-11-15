---
id: "TSPEC-rlDmQ7Q1AfhkLpv6bGeSoQ"
type: "test-spec"
status: "approved"
title: "Issue #192 Virtual-first 受入 Owner remap の検証規則"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/designs/issue-192-virtual-first-owner-remap-design.md"
      - type: "file"
        path: "docs/test-specs/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
      - type: "file"
        path: "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
      - type: "file"
        path: "docs/impl/ir/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/impl/exp/issue-192-virtual-first-owner-remap.md"
    modified:
      - type: "file"
        path: "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "file"
        path: "docs/discovery/issue-192-ta-invariant-map.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-192-ta-invariant-map.md"
    - "docs/test-specs/issue-275-obs-owner-map.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-192-virtual-first-owner-remap-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
    - "docs/designs/issue-192-virtual-first-owner-remap-design.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #192 Virtual-first 受入 Owner remap の検証規則

## Purpose

本 test-spec は `docs/designs/issue-192-virtual-first-
owner-remap-design.md` が定める「tracking canonical 末尾
への dated addendum・対応表 Headless Gap 節への dated
addendum・文書チェーン」の記述が、実在する正本
（Issue・POLICIES・既存 docs）に基づき、append-only と
状態語彙の分離を保っていることを検証する規則を固定する。
文書監査タスクであり、コードの test ではない — crate の
contract test の正本は各領域の test-spec であり、本書は
それらを置き換えない。

## Feature

TA DAG L2/L3 受入 Owner 欄の Virtual-first remap が、
tracking canonical と対応表へ dated addendum として
正確に記録される。

## Rules

- **R1（addendum の dated・append-only 性）**: 両
  addendum は当日日付（2026-11-15）の追記として識別
  可能であり、既存の 2026-11-09 snapshot 表（TA gate・
  3 者 Ownership・capability map）と 2026-11-14 snapshot
  表（対応表・Headless Gap 接続一覧）に差分がない。
- **R2（remap 対応の網羅と実在性）**: tracking canonical
  の remap 表は Windows 実 WASAPI/WinMM 仮想 Endpoint →
  RF #629、正規 Plugin ABI → RF #630、Windows 使用 GUI
  → RF #631、OS-L1/両 Consumer 独立検証 → RF #294・
  AL #127・SV #28 を含み、共通基盤 RF #628 と方針基盤
  RF #627（CLOSED）を併記する。各参照は Issue 番号・
  役割・現行状態を持ち、`gh issue view` 実測
  （2026-11-15）と一致する。
- **R3（受入 Owner/実装 Owner の分離）**: 対応表
  addendum は「受入 Owner（acceptance）」と「実装
  Owner」を区別し、#251・#253・#388/#389 が実装 Owner
  として維持されること、#243/#313 が CLOSED になった
  旨が日付根拠付きで明記されることを要求する。
- **R4（降格禁止と非昇格）**: 「仮想 Endpoint の Pass
  は物理機材の Driver/ADC-DAC を保証しない」「未検証
  Case は `NotRun`」「物理機材・市販 Plugin の一括
  Required 化は課さない」「design-Close と impl 受入
  を分離」の原則が dated note に記され、Windows 利用
  機能の L2/L3 の Required→Deferred 降格・既存実装/CI
  への Pass 付与・`NotRun` の Pass 化がない。
- **R5（対象外行の不変）**: 対応表の指定外行
  （#262/#465・#219/#281/#294・#306/SV#84・Consumer
  継続追従行）に差分がない。
- **R6（文書チェーンの規約適合）**: spec/design/
  test-spec/plan/task/impl ir・exp の 7 ファイルが
  `issue-192-virtual-first-owner-remap` slug で存在し、
  front matter の `id`/`type`/`status`/`relations` が
  実態どおりで、ir が `metadata.experiments` を持つ。
- **R7（変更範囲の限定）**: 変更は `docs/` 配下と生成
  index のみであり、`crates/`・`Cargo.toml`・
  `Cargo.lock`・`mise.toml`・`POLICIES.md`・`AGENTS.md`・
  `.devin/`・umbrella 設計正本・`evidence/` に差分が
  ない。
- **R8（検証記録の正直さ）**: `docs/impl/exp/` の記録が
  実行した検証（コマンド・exit code）と参照 Issue の
  実在照合結果・変更差分要約を含み、cargo test/build
  を実行していないこと・docs-only であることを正直に
  記す。

## Examples

- Given tracking canonical 末尾の追記節、When remap 表を
  検査する、Then 実 Audio Input・OS MIDI が RF #629、
  Plugin ABI が RF #630、実 GUI が RF #631、OS-L1/両
  Consumer 独立検証が RF #294・AL #127・SV #28 に
  対応付けられ、各行に OPEN/CLOSED の状態が記される
  （R2）。
- Given 対応表の追記、When 「実 GUI」行を検査する、Then
  受入 Owner が RF #631（OPEN）であり、実装/旧受入参照
  として #243 CLOSED・#313 CLOSED が 2026-11-14
  snapshot との対比付きで記される（R3）。
- Given 本 PR の diff、When `git status`/`git diff` を
  確認する、Then 既存 snapshot 表・指定外行に差分がなく、
  変更は `docs/` と生成 index のみである（R1/R5/R7）。
- Given `docs/impl/exp/issue-192-virtual-first-owner-
  remap.md`、When 検証記録を検査する、Then `gh issue
  view` の照合日時・結果、実行コマンドと exit code、
  docs-only である旨が記される（R8）。

## Guarantees

- 両 addendum の参照先は実在する Issue・文書・POLICIES
  節に限定され、存在しない参照を根拠にしない。
- 「受入 Owner」と「実装 Owner」の区別が保たれ、
  #251/#253/#388/#389 の役割維持と #243/#313 の CLOSED
  化が読み取れる。
- Required→Deferred 降格・Pass 付与・NotRun の Pass 化
  がなく、POLICIES §2/§3 の状態語彙が維持される。
- Issue 状態 snapshot は `gh issue view` 実測日付付きで
  記録され、将来の状態変化に対する再照合の必要性が
  示される。

## Non-goals

- #628〜631・#294・Consumer repo での実装・実走・受入
  （各 Issue が Owner）。
- umbrella 設計正本・領域別正本・POLICIES・AGENTS の
  変更、GitHub blockedBy の登録・削除、新規 Issue 起票。
- `crates/` の test/build 実行、CI lane の追加・変更、
  `evidence/` の作成。
- TA gate・G0〜G7 の正式 Close 判定と Issue #192 自体
  の Close（perpetual tracker — 本 slice merge だけでは
  完了を宣言しない）。

## Risk

- 本 test-spec が検証するのは文書の正しさであり、
  remap 先の #628〜631・#294・Consumer 受入の実施は
  別段階で検証される。addendum が完璧でも Harness
  未構築なら Required Case は `NotRun` であり、Issue
  #192 の受入は未完である — この分離を崩すと
  「記録完了＝受入完了」の誤認が起きる。
- Issue 状態は変動する。snapshot 日付の明記で緩和する
  が、将来の main/Issue 変更で状態が変わった場合、
  本記録は再照合が必要になる。
