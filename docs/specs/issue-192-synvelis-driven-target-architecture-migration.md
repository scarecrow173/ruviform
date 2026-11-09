---
id: "SPEC-034UBU16WQsh5QzC5ak17G"
type: "spec"
status: "approved"
title: "Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/musical-time-model.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/191"
    - "https://github.com/scarecrow173/ruviform/issues/194"
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/197"
    - "https://github.com/scarecrow173/ruviform/issues/198"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/297"
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/299"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/17"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/19"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/23"
    - "https://github.com/scarecrow173/synvelis/issues/25"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/83"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
  defers: []
  deferred-by: []
---

# Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本

## What Should Be Built

Issue #192（Synvelis-driven Ruviform Target v0.7.1 —
Dependency DAG / Graph / State Ownership、umbrella
migration tracking issue）の合意済み設計を `docs/` チェーン
（spec → design → test-spec → plan → task）として正本化
する。具体的には
`docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
を新設し、次の要素を文書化する。

1. **正本関係の固定**: 目標アーキテクチャの設計権威は
   approved の
   `docs/designs/synvelis-driven-target-architecture-and-migration.md`
   （2026-11-01 合意）と実行計画 Issue #193（M0〜M7・
   G0〜G7・GUI lane 補足を含む）にあり、本書はそれらを
   置き換えず、Issue #192 の受入管理（TA-001〜TA-012）
   と各領域の正本文書・実装 Issue・証拠の対応を統合する
   tracking canonical として機能することを明記する。
2. **Ownership boundary**: Ruviform / Synvelis /
   AudioLens の 3 者について、umbrella 設計 §2 の
   Synvelis 中心表を、2026-11-02 追補（両 Consumer 対等
   原則）と #219 互換契約に基づき AudioLens 列を含む
   形へ更新した対応表。
3. **Capability → 正本文書 map**: M0〜M7 の各段階・
   独立 GUI lane・共通基盤（IO/Observability）・
   Cross-Consumer gate について、担当 Issue・spec/
   design/test-spec/plan/task の正本パス・merge 済み PR・
   未解決事項を 2026-11-09 時点の snapshot として対応付け
   る索引。
4. **TA-001〜TA-012 gate 対応**: 各受入 gate の要求証拠を、
   担当する正本文書・Issue・検証レーン（headless PoC /
   CI / consumer-verify / 実機）へ対応付け、merge 済み
   実装と未検証・未受入を分離して記録する。
5. **G0〜G7・E2E・GUI lane の構造**: #193 の受入 gate
   定義、F0/E2E-L/E2E-R/E2E-X の並列構造、2026-11-07/08
   追補の GPUI Driver 移行 lane と #465 WebView 撤去を
   現行計画の一部として記述する。
6. **ADR・互換・Rollback policy の現状反映**: umbrella
   設計 §8 の ADR 表を 2026-11-09 時点へ更新する
   （`ui-architecture-webview-hybrid` は #384 後継 ADR
   proposed により superseded、可変 PPQN successor ADR は
   #189 で実装済み、等）。
7. **Consumer 接続面**: Synvelis（#17〜#23・#25・#28・
   #82・#83・#84 等）と AudioLens（#95・#127・#139〜#143
   等）が Ruviform の汎用能力をどう利用し、schema/
   revision/precision 等の製品意味論を各 repo が所有する
   境界、および #219/#281/#288/#294 の gate 運用を記述
   する。
8. **設計合意と実装/検証の分離**: 「設計 approved ≠
   実装完了 ≠ Consumer 受入」を明記し、未実施項目は
   `not run`/`NotRun`/`Unsupported` と記録する方針を
   固定する。

## Why Now

- Issue #192 は umbrella tracking issue であり、M1〜M6 の
  実装 Issue（#184〜#198）は merge 済みだが、#192 自体を
  代表する spec→design→plan→task の docs チェーンが
  存在しない。領域別の正本は各 Issue の docs チェーンに
  分散しており、「どの gate がどの正本と証拠で閉じるか」
  を一箇所から辿る手段がない。
- Consumer 側の IO 移行（Synvelis #82/#84、AudioLens
  #139/#143）や Cross-Consumer 受入（#219・#281・#288・
  #294）が進行中であり、#192 の TA gate と既存正本・
  consumer Issue の対応関係を固定する需要がある。
- umbrella 設計文書は 2026-11-01 時点の合意を保持するが、
  その後の GUI lane 追加（#193 2026-11-07/08 追補）・
  ADR supersede（#384）・IO/Observability 正本群の merge
  （#284〜#306・#409・#275/#276/#280）を反映した現状
  snapshot が未整理である。

## Users and Value

- **Ruviform maintainer / #192 Owner**: TA-001〜012 と
  M0〜M7 の各 gate がどの正本文書・Issue・証拠で閉じるか
  の対応表を得て、残件の優先順位と受入判定の根拠を
  固定できる。
- **Synvelis / AudioLens Owner**: 自 repo の Issue
  （SYN-IO/AL-IO・consumer-verify・実機受入）が Ruviform
  側のどの正本・gate に接続するかを確認できる。
- **#219/#281/#288/#294 の gate Owner**: consumer 別の
  impacted/no-impact/unknown 判定と証拠送付先の対応を
  辿れる。
- **後続の実装・保守担当**: 領域別詳細は各 Issue の
  spec/design/test-spec を正本として参照し、本書では
  umbrella レベルの対応と現状のみを確認できる。

## Scope

### In Scope

- `docs/` 配下の文書チェーン（spec/design/test-spec/
  plan/task）と生成 index のみ。**コード変更なし**。
- Issue #192 本文（umbrella 設計・TA-001〜012・
  2026-11-02 両 Consumer 対等原則追補）と Issue #193
  （M0〜M7 実行計画・G0〜G7・GUI lane 補足）の要求事項の
  文書化。
- approved の umbrella 設計文書・#193・関連 ADR・領域別
  正本文書群・Consumer Issue の main 上の成果を消費した
  対応表・現状 snapshot の記述。

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/` の変更（本 slice は docs 設計専用）。
- 目標アーキテクチャの再決定・umbrella 設計文書の本文
  改訂（同文書は approved の設計権威として維持し、本書は
  その追跡 canonical として参照関係を記述する）。
- 領域別設計の再定義（#184〜#198・IO #283〜#306・
  Observability #275〜#280・GUI #384〜#390/#465 等の
  各正本が詳細を所有する）。
- AudioLens/Synvelis repo の変更・実測（cross-repo gate、
  #219/#294 の consumer-verify が Owner）。
- 新規 CI Workflow・証拠 YAML/JSON スキーマ・Dashboard
  の作成。
- M0〜M7 の実装作業そのもの（#193 と各実装 Issue が
  Owner。本書は対応関係の記録のみ）。

## Requirements

- 正本関係: umbrella 設計文書と #193 が設計・実行の権威で
  あり、本書がそれらを supersede しないこと、および本書の
  役割（gate→正本→証拠→状態の統合索引）を明文化する。
- Ownership boundary: Ruviform/Synvelis/AudioLens の責務
  分界を、両 Consumer 対等原則（#192 2026-11-02 追補・
  #219）に整合する 3 者表で記述する。
- Capability map: M0〜M7・GUI lane・共通基盤・
  Cross-Consumer gate の各領域について、担当 Issue・
  正本 docs パス・merge 済み PR・未解決事項を対応付ける。
- TA gate 対応: TA-001〜TA-012 の各項目について、要求証拠・
  担当正本・検証レーン・現在の証拠状態を対応付ける。
- Gate/E2E 構造: G0〜G7・F0・E2E-L/E2E-R/E2E-X・GUI lane
  の依存と並列関係を #193 の現行版に基づき記述する。
- ADR/互換: umbrella §8 の ADR 表を現状（#384 proposed に
  よる WebView hybrid supersede 等）へ更新した対応表を
  記述し、Snapshot Swap 維持・後継 ADR 原則・Rollback
  方針を保持する。
- Consumer 接続面: 両 Consumer の IO/Persistence/精度
  契約が Ruviform 正本のどこに接続するか、schema/
  revision/precision の製品所有を明記する。
- 設計/実装分離: 設計 approved・merge 済み実装・Consumer
  受入・実機検証を別の状態として記録し、未実施を実施済み
  と報告しない方針を明記する。
- 参照実在性: 記載する Issue 番号・PR 番号・docs パス・
  crate/機能名が作成時点で実在することを確認する。

## Acceptance Criteria

- [ ] TA-001〜TA-012・M0〜M7・G0〜G7・E2E 構造・GUI lane・
      Consumer 接続面・ADR/互換 policy を含む正本設計文書が
      `docs/designs/` に存在する。
- [ ] 各領域（M0〜M7・GUI・IO・Observability・
      Cross-Consumer）が担当 Issue・正本 docs パス・証拠
      状態へ対応付けられ、参照先が実在する。
- [ ] umbrella 設計文書（approved）と #193 が設計・実行の
      権威であり本書が supersede しない旨が明記されて
      いる。
- [ ] Ruviform/Synvelis/AudioLens の責務分界が両 Consumer
      対等原則に整合して記述されている。
- [ ] ADR 対応表が 2026-11-09 時点の status（accepted /
      proposed / superseded）を反映している。
- [ ] 「設計合意と実装/検証の分離」が明記され、未実施項目は
      `not run`/`NotRun`/`Unsupported` と記録する方針が
      明示されている。
- [ ] 変更が `docs/` のみであり、`crates/`・`Cargo.toml`・
      `Cargo.lock`・`mise.toml`・`.github/` に差分がない。

## Deferred Design Concerns

- Consumer 実測・consumer-verify tuple run・実機 OS での
  受入 | cross-repo gate（#219/#281/#288/#294）と各
  Consumer Issue が Owner であり本 slice の対象外 |
  Consumer 候補 tuple の実走・Consumer repo の受入実施時
  | 設計どおりの互換性が実測で裏付けられないまま採用される
  リスク。
- GUI lane の最終採否（GPUI Native First・#262）・
  Legacy WebView 撤去完了（#465） | #193 GUI lane と各
  Issue が Owner | #262 の採否判定と #465 の撤去完了時 |
  WebView/GPUI の併存期間が長期化し二重保守になるリスク。
- TA gate の正式 Close 判定 | #192 Owner と gate 証拠の
  棚卸し時 | 各 gate の証拠が揃った時点 | 証拠未完のまま
  Issue を実装完了扱いで閉じるリスク。

## Sources and Relations

- 正本 Issue: [#192](https://github.com/scarecrow173/ruviform/issues/192)
  （umbrella — Target v0.7.1・TA-001〜012・2026-11-02 両
  Consumer 対等原則追補）。
- 実行計画: [#193](https://github.com/scarecrow173/ruviform/issues/193)
  （M0〜M7・G0〜G7・GUI lane 補足・consumer 別受入表）。
- 設計権威: `docs/designs/
  synvelis-driven-target-architecture-and-migration.md`
  （approved 2026-11-01）・Synvelis 正本
  `synvelis-driven-ruviform-target-architecture-v07.md`。
- M0 baseline: [#194](https://github.com/scarecrow173/ruviform/issues/194)
  CLOSED・`docs/discovery/issue-194-m0-baseline-audit.md`・
  PR #200。
- 領域別実装 Issue（全て CLOSED — 設計+実装 slice 完了、
  2026-11-09 時点）: #181/#182/#184/#185/#186/#187/#188/
  #189/#190/#191/#195/#196/#197/#198。未解決の後続:
  #243（実 GUI 検証）・#251（実 Audio Input）・#253（OS
  MIDI/VST3）・#262（GPUI Native 採否）。
- Cross-Consumer / Gate: #219（OPEN・spec proposed）・
  #281（OPEN）・#288（OPEN・design approved）・#294（OPEN）。
- Consumer Issue: Synvelis #17〜#23・#25・#28・#82・#83・
  #84、AudioLens #95・#127・#139〜#143（状態は
  2026-11-09 時点の snapshot）。
- 参照設計文書: 領域別 `docs/designs/issue-18x/19x-*.md`、
  IO `issue-28x/29x/30x/40x-*.md`、Observability
  `issue-27x/28x-*.md`、GUI `issue-38x/46x-*.md` の各正本。
