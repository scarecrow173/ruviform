---
id: "TSPEC-034UjQC8AnQxl52Q8wCT6N"
type: "test-spec"
status: "approved"
title: "Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/257"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/specs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/plans/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/tasks/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-257-driver-neutral-ui-contract.md"
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
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-257-driver-neutral-ui-contract.md"
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合

## Purpose

Issue #257 の統合正本（[design](../designs/issue-257-driver-neutral-ui-contract.md)）
が「契約を再定義せず参照で構成され、証拠が誠実で、歴史的参照と
未確定事項が明確に区別されている」ことを検証する。対象 spec は
[spec](../specs/issue-257-driver-neutral-ui-contract.md)、実施記録は
[task](../tasks/issue-257-driver-neutral-ui-contract.md) /
[plan](../plans/issue-257-driver-neutral-ui-contract.md) を参照。
コード変更を伴わない docs 成果物のため、検証は文書検査
（link/relation 解決・内容の突合・lint）で行う。

## Feature

Driver-neutral Reusable UI Contract 蓄積契約の正本統合文書

## Rules

- **R1**: 統合正本は契約の意味論・型シグネチャを再定義しない。
  各契約領域は merge 済み文書（spec/design/ADR/test-spec）または
  実装型（`crates/ruviform-ui*/src/`）への参照で構成される。
- **R2**: Issue #257 の各受入項目に対し、実現した PR / 文書 / 型
  ファイルを対応付けた証拠表が存在し、各エントリが実在する
  merge 済み成果物を指す。
- **R3**: 「merge 済み・契約レベル決定」と「実機 Pass」を混同
  しない。未実施・未検証は `NotRun`/`unsupported`/`blocked`/
  owner Issue 参照として記録される。
- **R4**: WebView 時代の前提（`ruviform-ui-webview` への ViewSpec
  描画義務、DOM/CustomEvent/SolidJS/Playwright の必須依存、
  wry/tao 経路の受入要件化、旧 ADR 2 件）は「歴史的経緯 / 廃止」
  として区別され、現行契約と混在しない。
- **R5**: GPUI の最終採用判定が未確定（#262 verdict pending・
  #419 後継 ADR）であることが正本に明記され、本書が GPUI 採用
  確定の根拠文書にならない。
- **R6**: 既存の他 Issue 文書は編集されない（参照・リンクのみ）。
  変更は新規 `issue-257-*` 文書チェーンと生成 index に限定される。
- **R7**: front matter の relations・status は doc-driven-dev 規約に
  従い、index（`docs/*/README.md`）が再生成されている。

## Examples

- **E1**: 証拠対応表 AC1 行が `driver.rs`/`view.rs`/`dock.rs`/
  `native_slot.rs` の実在型と #386/#387 の merge 済み文書を指し、
  「契約レベルで決定・merge 済み」かつ実機 IME/focus は owner Issue
  残置と読める。
- **E2**: AC2 行が `check-ui-driver-boundary`・`SemanticGuiTest`・
  AL PR #188（merged）を指し、SV 側は SV#112（Open）本体と
  記録される。
- **E3**: 歴史的参照表が旧 2 ADR の `superseded` 状態・
  `ruviform-ui-webview` の legacy 分類・#455 Closed(not planned)・
  #258〜#263/#310〜#312 Closed 履歴を列挙する。
- **E4**: 「未確定事項」節が #262/#419/#388/#389/SV#112/
  WebView 物理削除 gate を列挙し、GPUI 採用確定を示唆する記述が
  ない。

## Guarantees

- 統合正本の全リンク・relation が解決可能（doc-status lint で
  broken relation なし）。
- 証拠表の PR 番号・文書パス・型名・Issue 状態が作成時点の
  main / GitHub 状態と一致する。
- 本 PR の diff が docs 新規文書 + 生成 index のみで、コード・
  既存他 Issue 文書・`.github/`・`mise.toml`・`Cargo.*` を
  含まない。

## Non-goals

- 契約内容の正しさ自体の再検証（各正本の test-spec/実装 lane が
  担う）。
- 実 OS / 実 Plugin / Native Dock / IME の実機検証（owner Issue
  の範囲）。
- Issue #257 の Close 判断・SV#112 の consumer slice 受入。

## Risk

- 索引が将来の契約更新に追従しない陳腐化 → 参照型設計と
  「状態は 2026-11-09 snapshot」の明記で緩和。正本側の status
  変更時に本書の記述を見直す運用を残す。
- 証拠の誤読（契約 merge ≠ 実機 Pass）→ R3 の状態列と owner
  Issue 明記で防ぐ。
