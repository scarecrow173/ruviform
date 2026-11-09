---
id: "SPEC-034UjQv2KPJWujHxiCffIK"
type: "spec"
status: "approved"
title: "Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/384"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/plans/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/tasks/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-257-driver-neutral-ui-contract.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
    - "docs/specs/issue-386-driver-neutral-gui-host.md"
    - "docs/specs/issue-387-component-view-specs.md"
    - "docs/specs/issue-390-driver-contract-lab.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
    - "docs/specs/issue-465-legacy-webview-p0p1.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/384"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/455"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/synvelis/issues/112"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  defers: []
  deferred-by: []
---

# Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合

## What Should Be Built

Issue #257（Driver-neutral Reusable UI Contract — GPUI は標準
Presentation Driver の横断設計正本）の要求に対し、**蓄積済みの契約を
1 つの正本設計書へ統合する docs 成果物**を作成する。具体的には
`docs/designs/issue-257-driver-neutral-ui-contract.md` を新設し、
次を満たす。

- identity / generation / View lifecycle / Focus / IME /
  Unsupported・Presentation Driver 抽象（GPUI = 標準 driver の
  位置づけ）・Scenario / Fake / 実 driver の層区別を、**merge 済みの
  既存文書と実装型への参照として**索引・整列する。契約の再発明・
  再定義はしない。
- Issue #257 の受入項目ごとの**証拠対応表**（各 AC → 実現した
  PR / 文書 / 型ファイル）を正本内に含める。
- 陳腐化した参照（WebView 時代の前提・廃止済み判断）を「歴史的
  経緯 / 廃止」として明確に区別する。
- **GPUI の最終採用判定は未確定（#262 verdict pending）**である
  ことを正本上に明記し、本書を「GPUI 採用確定」の根拠文書にしない。

本 spec / plan / task / test-spec はこの docs 成果物を対象リポジトリの
文書チェーンとして記録する内部文書である。チェーン:
[design](../designs/issue-257-driver-neutral-ui-contract.md)・
[plan](../plans/issue-257-driver-neutral-ui-contract.md)・
[task](../tasks/issue-257-driver-neutral-ui-contract.md)・
[test-spec](../test-specs/issue-257-driver-neutral-ui-contract.md)。

## Why Now

- Issue #257 は driver-neutral 再利用 UI contract の横断設計正本を
  要求する親 Issue であり、契約の実体は #385〜#390・#464・#451・
  #478・#479 の merge 済み文書と `crates/ruviform-ui*/` の型に分散
  蓄積されている。受入項目と証拠の対応を 1 箇所で追跡できる正本が
  無い。
- Issue #257 本文には WebView-first / SolidJS / DOM・CustomEvent 前提の
  旧記述が履歴として残っており、現行契約（#384/#465 で確定した
  GPUI 標準 driver + WebView legacy 撤去方針）との区別を文書上で
  固定する必要がある。
- GPUI 最終採用は #262 verdict・#388/#389 実機証拠・#419 後継 ADR
  待ちであり、統合文書が採用確定を誤って示唆しないことが重要。

## Users and Value

- #386/#387 系の後続実装・レビュー担当: 契約の所在（文書・型・
  fixture）を 1 つの索引から辿れる。
- 両 Consumer（Synvelis / AudioLens）の GUI 実装者: driver-neutral
  契約の適用範囲と未確定事項（GPUI 採否・Native Dock）を誤解なく
  確認できる。
- Coordinator / reviewer: Issue #257 の Close 判断に必要な証拠
  対応をレビューできる。

## Scope

### In Scope

- `docs/designs/issue-257-driver-neutral-ui-contract.md` の新設
  （統合正本: 契約領域別の参照索引・受入証拠対応表・歴史的参照の
  区別・未確定事項の明記）。
- 本 spec / plan / task / test-spec の文書チェーン。
- 生成物である docs index（`docs/*/README.md`）の再生成。

### Out of Scope

- すべてのコード変更（`crates/` 配下一切）・API 変更・テスト追加。
- 既存の他 Issue 文書の編集（`issue-384-*`・`issue-38[5-9]-*`・
  `issue-39[0]-*`・`issue-45[0-9]-*`・`issue-46[0-9]-*`・
  `issue-47[0-9]-*` 等はリンク・参照のみ）。
- #262（GPUI 採否 verdict）・#419（後継 ADR）・#388/#389（native
  dock PoC）の実作業。
- Synvelis 側 2-consumer slice の受入実作業（SV#112 本体。
  本正本はその参照先として位置づけるに留める）。
- Issue #257 の Close 判断（Coordinator が PR レビュー後に行う）。
- `.github/`・`.devin/`・`mise.toml`・`Cargo.*` の変更。

## Requirements

- 統合正本は契約を**再定義せず**、merge 済み文書（spec / design /
  ADR / test-spec）と実装型（`crates/ruviform-ui*/src/`）への
  参照で構成する。
- Issue #257 の各受入項目に対し、実現した PR / 文書 / 型ファイルを
  対応付けた証拠表を含める。未検証・未実施の項目は `NotRun` /
  `Unsupported` / `blocked` として正直に記録し、Pass と混同しない。
- WebView 時代の前提（`ruviform-ui-webview` の ViewSpec 描画義務、
  DOM / CustomEvent / SolidJS / Playwright の必須依存、wry/tao
  経路の受入要件化）を「歴史的経緯 / 廃止」として区別する。
- GPUI の最終採用判定が未確定（#262 verdict pending・#419 後継
  ADR）であることを正本に明記する。
- `docs/designs/` の命名・front matter・relations 規約
  （`doc-driven-dev` 文書スイート）に従う。

## Acceptance Criteria

Issue #257 本文の受入条件を本 dispatch のスコープへ対応付けたもの:

- [ ] `docs/designs/issue-257-*.md` が存在し、identity /
  generation / View lifecycle / Focus / IME / Unsupported・
  Presentation Driver 抽象（GPUI = 標準 driver）・Scenario /
  Fake / 実 driver の層区別を既存 merge 済み文書・実装型への
  参照として統合している（契約の再発明・再定義なし）。
- [ ] Issue 受入項目ごとの証拠対応表（各 AC → 実現した PR / 文書 /
  型ファイル）が正本内に含まれる。
- [ ] 陳腐化した参照（WebView 時代の前提・廃止済み判断）が
  「歴史的経緯 / 廃止」として明確に区別されている。
- [ ] GPUI の最終採用判定が未確定（#262 verdict pending）である
  ことが正本に明記され、本書が GPUI 採用確定の根拠文書に
  なっていない。
- [ ] 既存の他 Issue 文書が編集されていない（リンク・参照のみ）。
- [ ] `mise run fmt-check` と文書系 lint（doc-status 系）が pass。

## Deferred Design Concerns

- GPUI 技術採用の最終判定 → #262 verdict + #388/#389 実機証拠 +
  #419 後継 ADR。本正本は採否を決定しない。
- Synvelis 側 2-consumer slice の受入 → SV#112 本体（Open）。
  AudioLens 側は #464 Scenario Contract 採用済み（AL PR #188）。
- 旧 WebView 実装の物理削除 → #465 が Owner、削除 Gate は #419
  採否 + #294 両 Consumer 非回帰の確認後。

## Sources and Relations

- Issue #257（2026-11-07 確定仕様 v2・2026-11-08 現行契約補足・
  2026-11-09 UI 依存方針が旧記述に優先）、#384（上位 ADR owner）、
  #385〜#390（契約子 Issue）、#464（Scenario Contract）、#451
  （focus traversal / keyboard tab / Tabs pointer・descriptor）、
  #478（Fader/Knob 縦寸法）、#479（keyboard value controls）、
  #262（GPUI 採否 PoC、Open）、#419（後継 ADR、Open）、#465
  （legacy WebView 撤去、P0/P1 済）、#294（cross-consumer gate）。
- 実装側の正本: `crates/ruviform-ui`（driver-neutral 契約層）、
  `crates/ruviform-ui-theme`（DTCG/ResolvedTheme）、
  `crates/ruviform-ui-gpui`（標準 driver adapter）、
  `crates/ruviform-ui-test`（Fake driver / driver-lab / scenario
  contract）、`crates/ruviform-ui-webview`（legacy adapter）。
